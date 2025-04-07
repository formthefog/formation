import time
import torch
import torch.distributed as dist
from transformers import AutoModelForCausalLM, AutoTokenizer
from device import Device
from model import Model
from worker import Worker

class DeepSeekWorker(Worker):
    def __init__(self, rank: int, model: Model, device: Device):
        super().__init__(rank, model, device)

    def run(self, world_size, input_data):
        start_time = time.time()
        try:
            self.init_distributed(world_size)
            print(f"Worker {self.rank}: Ready to process requests...")

            model_id = "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"
            model = AutoModelForCausalLM.from_pretrained(model_id).to(self.device)
            tokenizer = AutoTokenizer.from_pretrained(model_id)
            
            total_layers = len(model.model.layers)
            layers_per_node = total_layers // world_size
            start_layer = self.rank * layers_per_node
            end_layer = start_layer + layers_per_node if self.rank < world_size - 1 else total_layers

            # Move layers not assigned to this worker to CPU
            for i, layer in enumerate(model.model.layers):
                if i < start_layer or i >= end_layer:
                    layer.to("cpu")

            print(f"Total layers: {total_layers}, Layers per node: {layers_per_node}, Start layer: {start_layer}, End layer: {end_layer}")

            received_text = None

            if self.rank == 0:
                print(f"Worker 0: Processing input: {input_data}")
                input_ids = tokenizer(input_data, return_tensors="pt").to(self.device)
                
                # Initial embedding
                hidden_states = model.model.embed_tokens(input_ids.input_ids)

                # Process layers assigned to Worker 0
                for i in range(start_layer, end_layer):
                    hidden_states = model.model.layers[i](hidden_states)[0]

                print("Worker 0: Sending processed hidden states and input IDs to Worker 1...")
                
                # Send hidden states shape and content
                hidden_states_shape = torch.tensor(hidden_states.shape, dtype=torch.int64).to(self.device)
                dist.send(hidden_states_shape, 1)
                dist.send(hidden_states, 1)

                # Send input IDs shape and content
                input_ids_shape = torch.tensor(input_ids.input_ids.shape, dtype=torch.int64).to(self.device)
                dist.send(input_ids_shape, 1)
                dist.send(input_ids.input_ids, 1)

                # Receive generated text length and content
                text_length = torch.zeros(1, dtype=torch.int64).to(self.device)
                dist.recv(text_length, world_size - 1)

                recv_buffer = torch.zeros(text_length.item(), dtype=torch.uint8).to(self.device)
                dist.recv(recv_buffer, world_size - 1)

                received_text = recv_buffer.cpu().numpy().tobytes().decode("utf-8")
                print(f"Worker 0: Received generated text ({text_length.item()} bytes): {received_text}")

            elif self.rank > 0:
                print(f"Worker {self.rank}: Waiting for hidden states from Worker {self.rank - 1}...")

                # Receive hidden states
                hidden_states_shape = torch.zeros(3, dtype=torch.int64).to(self.device)
                dist.recv(hidden_states_shape, self.rank - 1)
                recv_hidden_states = torch.zeros(tuple(hidden_states_shape.tolist()), device=self.device)
                dist.recv(recv_hidden_states, self.rank - 1)

                # Receive input IDs
                input_ids_shape = torch.zeros(2, dtype=torch.int64).to(self.device)
                dist.recv(input_ids_shape, self.rank - 1)
                recv_input_ids = torch.zeros(tuple(input_ids_shape.tolist()), dtype=torch.long, device=self.device)
                dist.recv(recv_input_ids, self.rank - 1)

                # Process layers assigned to this worker
                for i in range(start_layer, end_layer):
                    recv_hidden_states = model.model.layers[i](recv_hidden_states)[0]

                if self.rank == world_size - 1:
                    # Final worker generates text
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
                    
                    # Encode and send generated text back to Worker 0
                    encoded_text = torch.tensor(
                        list(generated_sentence.encode("utf-8")), dtype=torch.uint8
                    ).to(self.device)
                    
                    text_length = torch.tensor([encoded_text.shape[0]], dtype=torch.int64).to(self.device)
                    dist.send(text_length, 0)
                    dist.send(encoded_text, 0)
                else:
                    # Forward hidden states and input IDs to next worker
                    print(f"Worker {self.rank}: Forwarding hidden states and input IDs to Worker {self.rank + 1}...")
                    hidden_states_shape = torch.tensor(recv_hidden_states.shape, dtype=torch.int64).to(self.device)
                    dist.send(hidden_states_shape, self.rank + 1)
                    dist.send(recv_hidden_states, self.rank + 1)

                    input_ids_shape = torch.tensor(recv_input_ids.shape, dtype=torch.int64).to(self.device)
                    dist.send(input_ids_shape, self.rank + 1)
                    dist.send(recv_input_ids, self.rank + 1)

        finally:
            dist.destroy_process_group()
            print(f"Worker {self.rank}: Process group destroyed.")
            end_time = time.time()
            print(f"Worker {self.rank}: Execution time: {end_time - start_time:.2f} seconds")
            return received_text