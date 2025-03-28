import time
import torch
import torch.distributed as dist
from transformers import GPT2LMHeadModel, GPT2Tokenizer
from device import Device
from model import Model
from worker import Worker

class GPT2Worker(Worker):
    def __init__(self, rank: int, model: Model, device: Device):
        super().__init__(rank, model, device)

    def run(self, world_size, input_data):
        start_time = time.time()
        try:
            self.init_distributed(world_size)
            print(f"Worker {self.rank}: Ready to process requests...")

            model = GPT2LMHeadModel.from_pretrained("gpt2").to(self.device)
            tokenizer = GPT2Tokenizer.from_pretrained("gpt2")
            total_layers = len(model.transformer.h)
            layers_per_node = total_layers // world_size
            start_layer = self.rank * layers_per_node
            end_layer = start_layer + layers_per_node if self.rank < world_size - 1 else total_layers

            for i, layer in enumerate(model.transformer.h):
                if i < start_layer or i >= end_layer:
                    layer.to("cpu")

            print(total_layers, layers_per_node, start_layer, end_layer)

            received_text = None

            if self.rank == 0:
                print(f"Worker 0: Processing input: {input_data}")
                input_ids = tokenizer.encode(input_data, return_tensors="pt").to(self.device)
                hidden_states = model.transformer.wte(input_ids)
                for i in range(start_layer, end_layer):
                    hidden_states = model.transformer.h[i](hidden_states)[0]

                print("Worker 0: Sending processed hidden states and input IDs to Worker 1...")
                hidden_states_shape = torch.tensor(hidden_states.shape, dtype=torch.int64).to(self.device)
                print(f"Worker 0: Sending hidden_states with shape {hidden_states.shape}")
                dist.send(hidden_states_shape, 1)
                dist.send(hidden_states, 1)

                input_ids_shape = torch.tensor(input_ids.shape, dtype=torch.int64).to(self.device)
                print(f"Worker 0: Sending input_ids with shape {input_ids.shape}")
                dist.send(input_ids_shape, 1)
                dist.send(input_ids, 1)

                text_length = torch.zeros(1, dtype=torch.int64).to(self.device)
                dist.recv(text_length, world_size - 1)

                recv_buffer = torch.zeros(text_length.item(), dtype=torch.uint8).to(self.device)
                dist.recv(recv_buffer, world_size - 1)

                received_text = recv_buffer.cpu().numpy().tobytes().decode("utf-8")
                print(f"Worker 0: Received generated text ({text_length.item()} bytes): {received_text}")

            elif self.rank > 0:
                print(f"Worker {self.rank}: Waiting for hidden states from Worker {self.rank - 1}...")

                hidden_states_shape = torch.zeros(3, dtype=torch.int64).to(self.device)
                print(f"Worker {self.rank}: Waiting to receive hidden_states shape from Worker {self.rank - 1}")
                dist.recv(hidden_states_shape, self.rank - 1)
                print(f"Worker {self.rank}: Expecting hidden_states with shape {hidden_states_shape.tolist()} from Worker {self.rank - 1}")
                recv_hidden_states = torch.zeros(tuple(hidden_states_shape.tolist()), device=self.device)
                dist.recv(recv_hidden_states, self.rank - 1)
                print(f"Worker {self.rank}: Received hidden states with shape {recv_hidden_states.shape}")

                input_ids_shape = torch.zeros(2, dtype=torch.int64).to(self.device)
                print(f"Worker {self.rank}: Expecting input_ids with shape {input_ids_shape.tolist()} from Worker {self.rank - 1}")
                dist.recv(input_ids_shape, self.rank - 1)
                recv_input_ids = torch.zeros(tuple(input_ids_shape.tolist()), dtype=torch.long, device=self.device)
                dist.recv(recv_input_ids, self.rank - 1)
                print(f"Worker {self.rank}: Received input IDs with shape {recv_input_ids.shape}")

                for i in range(start_layer, end_layer):
                    recv_hidden_states = model.transformer.h[i](recv_hidden_states)[0]

                if self.rank == world_size - 1:
                    attention_mask = torch.ones(recv_input_ids.shape, device=self.device)
                    generated_ids = model.generate(
                        input_ids=recv_input_ids,
                        attention_mask=attention_mask,
                        max_length=recv_input_ids.shape[1] + 300,
                        do_sample=True,
                        top_k=50,
                        pad_token_id=tokenizer.eos_token_id,
                    )
                    generated_sentence = tokenizer.decode(
                        generated_ids.squeeze().tolist(), skip_special_tokens=True
                    )
                    
                    encoded_text = torch.tensor(
                        list(generated_sentence.encode("utf-8")), dtype=torch.uint8
                    ).to(self.device)
                    
                    text_length = torch.tensor([encoded_text.shape[0]], dtype=torch.int64).to(self.device)
                    dist.send(text_length, 0)

                    print(f"Worker {self.rank}: Sending {text_length.item()} bytes of text to Worker 0...")
                    dist.send(encoded_text, 0)
                else:
                    print(f"Worker {self.rank}: Forwarding hidden states and input IDs to Worker {self.rank + 1}...")
                    hidden_states_shape = torch.tensor(recv_hidden_states.shape, dtype=torch.int64).to(self.device)
                    print(f"Worker {self.rank}: Sending hidden_states with shape {hidden_states_shape.tolist()} to Worker {self.rank + 1}")
                    dist.send(hidden_states_shape, self.rank + 1)
                    dist.send(recv_hidden_states, self.rank + 1)

                    input_ids_shape = torch.tensor(recv_input_ids.shape, dtype=torch.int64).to(self.device)
                    print(f"Worker {self.rank}: Sending input_ids with shape {input_ids_shape.tolist()} to Worker {self.rank + 1}")
                    dist.send(input_ids_shape, self.rank + 1)
                    dist.send(recv_input_ids, self.rank + 1)
        finally:
            dist.destroy_process_group()
            print(f"Worker {self.rank}: Process group destroyed.")
            end_time = time.time()
            print(f"Worker {self.rank}: Execution time: {end_time - start_time:.2f} seconds")
            return received_text
