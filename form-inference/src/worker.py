import time
import torch
import torch.distributed as dist
from transformers import GPT2LMHeadModel, GPT2Tokenizer
import os
import psutil


def init_distributed(rank, world_size):
    os.environ["MASTER_ADDR"] = "localhost"
    os.environ["MASTER_PORT"] = "12345"
    print(f"Worker {rank}: Initializing process group...")
    dist.init_process_group(backend="gloo", rank=rank, world_size=world_size)
    print(f"Worker {rank}: Process group initialized.")


def run_worker(rank, world_size, input_data):
    start_time = time.time()
    try:
        # Set CPU affinity dynamically based on rank
        # p = psutil.Process(os.getpid())
        # p.cpu_affinity([rank])
        # print(f"Worker {rank}: Running on CPU core {p.cpu_affinity()}")

        init_distributed(rank, world_size)
        device = torch.device("cpu")

        print(f"Worker {rank}: Ready to process requests...")

        model = GPT2LMHeadModel.from_pretrained("gpt2").to(device)
        tokenizer = GPT2Tokenizer.from_pretrained("gpt2")

        # Splitting layers across workers
        total_layers = len(model.transformer.h)
        layers_per_node = total_layers // world_size
        start_layer = rank * layers_per_node
        end_layer = (
            start_layer + layers_per_node if rank < world_size - 1 else total_layers
        )

        print(total_layers, layers_per_node, start_layer, end_layer)

        # Move only assigned layers to the correct device
        for i, layer in enumerate(model.transformer.h):
            if i < start_layer or i >= end_layer:
                layer.to("cpu")

        received_text = None

        if rank == 0:
            print(f"Worker 0: Processing input: {input_data}")

            # Tokenize input_data
            input_ids = tokenizer.encode(input_data, return_tensors="pt").to(device)

            # Process through Worker 0's layers
            hidden_states = model.transformer.wte(input_ids)
            for i in range(start_layer, end_layer):
                hidden_states = model.transformer.h[i](hidden_states)[0]

            print(
                "Worker 0: Sending processed hidden states and input IDs to Worker 1..."
            )
            hidden_states_shape = torch.tensor(hidden_states.shape, dtype=torch.int64).to(device)
            print(f"Worker 0: Sending hidden_states with shape {hidden_states.shape}")
            dist.send(hidden_states_shape, 1)  # Send the shape first
            dist.send(hidden_states, 1)        # Then send the actual data

            # Send input IDs
            input_ids_shape = torch.tensor(input_ids.shape, dtype=torch.int64).to(device)
            print(f"Worker 0: Sending input_ids with shape {input_ids.shape}")
            dist.send(input_ids_shape, 1)      # Send the shape first
            dist.send(input_ids, 1)            # Then send the actual data

            # Receive the length of the incoming text
            text_length = torch.zeros(1, dtype=torch.int64).to(device)
            dist.recv(text_length, world_size - 1)

            # Allocate buffer dynamically based on received length
            recv_buffer = torch.zeros(text_length.item(), dtype=torch.uint8).to(device)
            dist.recv(recv_buffer, world_size - 1)

            # Decode received text
            received_text = recv_buffer.cpu().numpy().tobytes().decode("utf-8")
            print(f"Worker 0: Received generated text ({text_length.item()} bytes): {received_text}")

        elif rank > 0:
            print(f"Worker {rank}: Waiting for hidden states from Worker {rank - 1}...")

            hidden_states_shape = torch.zeros(3, dtype=torch.int64).to(device)  # Assuming 3D tensor
            print(f"Worker {rank}: Waiting to receive hidden_states shape from Worker {rank - 1}")
            dist.recv(hidden_states_shape, rank - 1)  # Receive the shape
            print(f"Worker {rank}: Expecting hidden_states with shape {hidden_states_shape.tolist()} from Worker {rank - 1}")
            recv_hidden_states = torch.zeros(tuple(hidden_states_shape.tolist()), device=device)  # Allocate buffer
            dist.recv(recv_hidden_states, rank - 1)  # Receive the actual data
            print(f"Worker {rank}: Received hidden states with shape {recv_hidden_states.shape}")

            # Receive input IDs
            input_ids_shape = torch.zeros(2, dtype=torch.int64).to(device)  # Assuming 2D tensor
            print(f"Worker {rank}: Expecting input_ids with shape {input_ids_shape.tolist()} from Worker {rank - 1}")
            dist.recv(input_ids_shape, rank - 1)  # Receive the shape
            recv_input_ids = torch.zeros(tuple(input_ids_shape.tolist()), dtype=torch.long, device=device)  # Allocate buffer
            dist.recv(recv_input_ids, rank - 1)  # Receive the actual data
            print(f"Worker {rank}: Received input IDs with shape {recv_input_ids.shape}")

            for i in range(start_layer, end_layer):
                recv_hidden_states = model.transformer.h[i](recv_hidden_states)[0]

            if rank == world_size - 1:
                attention_mask = torch.ones(recv_input_ids.shape, device=device)
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
                
                # Convert generated text to bytes
                encoded_text = torch.tensor(
                    list(generated_sentence.encode("utf-8")), dtype=torch.uint8
                ).to(device)
                
                # Send the length first
                text_length = torch.tensor([encoded_text.shape[0]], dtype=torch.int64).to(device)
                dist.send(text_length, 0)

                print(f"Worker {rank}: Sending {text_length.item()} bytes of text to Worker 0...")
                dist.send(encoded_text, 0)
            else:
                print(
                    f"Worker {rank}: Forwarding hidden states and input IDs to Worker {rank + 1}..."
                )
                hidden_states_shape = torch.tensor(recv_hidden_states.shape, dtype=torch.int64).to(device)
                print(f"Worker {rank}: Sending hidden_states with shape {hidden_states_shape.tolist()} to Worker {rank + 1}")
                dist.send(hidden_states_shape, rank + 1)  # Send the shape first
                dist.send(recv_hidden_states, rank + 1)   # Then send the actual data

                input_ids_shape = torch.tensor(recv_input_ids.shape, dtype=torch.int64).to(device)
                print(f"Worker {rank}: Sending input_ids with shape {input_ids_shape.tolist()} to Worker {rank + 1}")
                dist.send(input_ids_shape, rank + 1)      # Send the shape first
                dist.send(recv_input_ids, rank + 1)       # Then send the actual data
    finally:
        dist.destroy_process_group()
        print(f"Worker {rank}: Process group destroyed.")
        end_time = time.time()
        print(f"Worker {rank}: Execution time: {end_time - start_time:.2f} seconds")
        return received_text
