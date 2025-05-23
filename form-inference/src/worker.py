import torch
import torch.distributed as dist
from transformers import GPT2LMHeadModel, GPT2Tokenizer
import os
import psutil

def init_distributed(rank, world_size):
    os.environ['MASTER_ADDR'] = 'localhost'
    os.environ['MASTER_PORT'] = '12345'
    print(f"Worker {rank}: Initializing process group...")
    dist.init_process_group(backend='gloo', rank=rank, world_size=world_size)
    print(f"Worker {rank}: Process group initialized.")

def run_worker(rank, world_size, input_data):
    # Set CPU affinity dynamically based on rank
    p = psutil.Process(os.getpid())
    p.cpu_affinity([rank])
    print(f"Worker {rank}: Running on CPU core {p.cpu_affinity()}")

    init_distributed(rank, world_size)
    device = torch.device("cpu")

    print(f"Worker {rank}: Ready to process requests...")

    model = GPT2LMHeadModel.from_pretrained("gpt2").to(device)
    tokenizer = GPT2Tokenizer.from_pretrained("gpt2")

    # Splitting layers across workers
    total_layers = len(model.transformer.h)
    layers_per_node = total_layers // 2  
    start_layer = rank * layers_per_node
    end_layer = start_layer + layers_per_node if rank == 0 else total_layers

    # Move only assigned layers to the correct device
    for i, layer in enumerate(model.transformer.h):
        if i < start_layer or i >= end_layer:
            layer.to("cpu")

    if rank == 0:
        print(f"Worker 0: Processing input: {input_data}")

        # Tokenize input_data
        input_ids = tokenizer.encode(input_data, return_tensors='pt').to(device)

        # Process through Worker 0's layers
        hidden_states = model.transformer.wte(input_ids)
        for i in range(start_layer, end_layer):
            hidden_states = model.transformer.h[i](hidden_states)[0]

        print("Worker 0: Sending processed hidden states and input IDs to Worker 1...")
        dist.send(hidden_states, 1)
        dist.send(input_ids, 1)  
        print("Worker 0: Data sent to Worker 1.")

        # Wait for final processed text from Worker 1
        recv_buffer = torch.zeros(1024, dtype=torch.uint8).to(device)  # Allocate large enough buffer
        dist.recv(recv_buffer, 1)

        # Decode received text
        received_text = recv_buffer.cpu().numpy().tobytes().decode("utf-8").strip("\x00")
        print(f"Worker 0: Received generated text: {received_text}")

        return received_text

    elif rank == 1:
        print("Worker 1: Waiting for hidden states from Worker 0...")

        # Receive hidden states from Worker 0
        recv_hidden_states = torch.zeros(1, 128, model.config.n_embd).to(device)
        dist.recv(recv_hidden_states, 0)

        # Ensure `input_ids` is properly received
        recv_input_ids = torch.zeros((1, 10), dtype=torch.long).to(device)
        dist.recv(recv_input_ids, 0)

        print(f"Worker 1: Received input_ids: {recv_input_ids.tolist()}")

        # Process through Worker 1's layers
        for i in range(start_layer, end_layer):
            recv_hidden_states = model.transformer.h[i](recv_hidden_states)[0]

        # Generate using autoregressive decoding
        generated_ids = model.generate(
            input_ids=recv_input_ids,
            max_length=recv_input_ids.shape[1] + 50,  
            do_sample=True,  
            top_k=50  
        )

        print(f"Worker 1: Generated token sequence: {generated_ids.tolist()}")

        # Decode the generated token sequence to a string
        generated_sentence = tokenizer.decode(generated_ids.squeeze().tolist(), skip_special_tokens=True)
        print(f"Worker 1: Generated sentence: {generated_sentence}")

        # Convert string to byte tensor and send to Worker 0
        encoded_text = torch.tensor(list(generated_sentence.encode("utf-8")), dtype=torch.uint8).to(device)
        dist.send(encoded_text, 0)
        print("Worker 1: Generated text sent to Worker 0.")

    dist.destroy_process_group()
