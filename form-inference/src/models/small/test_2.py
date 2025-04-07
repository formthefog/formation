import os
import torch
import torch.distributed as dist
import torch.multiprocessing as mp
from transformers import GPT2LMHeadModel, GPT2Tokenizer

def setup_distributed(rank, world_size):
    """Set up the distributed environment."""
    os.environ['MASTER_ADDR'] = 'localhost'
    os.environ['MASTER_PORT'] = '12355'
    
    dist.init_process_group(
        backend='gloo',  # CPU-friendly backend
        rank=rank, 
        world_size=world_size
    )

def cleanup():
    """Clean up the distributed process group."""
    dist.destroy_process_group()

def distributed_generate_step(rank, world_size, input_ids, model, max_length):
    """
    Distribute generation steps across processes.
    
    Args:
        rank (int): Current process rank
        world_size (int): Total number of processes
        input_ids (torch.Tensor): Input token IDs
        model (GPT2LMHeadModel): Language model
        max_length (int): Maximum generation length
    
    Returns:
        torch.Tensor: Generated token for this process's step
    """
    # Placeholder for distributed generation logic
    with torch.no_grad():
        # Simulate distributed generation by having each process 
        # generate a portion of the sequence
        outputs = model.generate(
            input_ids, 
            max_length=max_length, 
            num_return_sequences=1,
            do_sample=True,
            top_k=50,
            top_p=0.95,
            temperature=0.7
        )
    
    return outputs[0]

def run_distributed_inference(rank, world_size):
    """
    Perform distributed inference for a single prompt.
    
    Args:
        rank (int): Current process rank
        world_size (int): Total number of processes
    """
    # Set up distributed environment
    setup_distributed(rank, world_size)
    
    try:
        # Load model
        model = GPT2LMHeadModel.from_pretrained("gpt2")
        tokenizer = GPT2Tokenizer.from_pretrained("gpt2")

        # Add padding token if not exists
        if tokenizer.pad_token is None:
            tokenizer.pad_token = tokenizer.eos_token

        # Single prompt for all processes
        prompt = "The future of artificial intelligence is"
        
        # Prepare input
        input_ids = tokenizer.encode(prompt, return_tensors="pt", padding=True)

        # Distributed generation
        generated_tokens = distributed_generate_step(
            rank, 
            world_size, 
            input_ids, 
            model, 
            max_length=50
        )

        # Decode and print from each process
        generated_text = tokenizer.decode(generated_tokens, skip_special_tokens=True)
        print(f"Rank {rank} generated: {generated_text}")

    except Exception as e:
        print(f"Error in process {rank}: {e}")
        import traceback
        traceback.print_exc()
    finally:
        cleanup()

def main():
    """Main function to orchestrate distributed inference."""
    # Determine number of processes
    world_size = mp.cpu_count()
    
    print(f"Using {world_size} processes for distributed inference")

    # Use spawn method for compatibility
    mp.set_start_method('spawn')
    
    # Launch distributed processes
    processes = []
    for rank in range(world_size):
        p = mp.Process(
            target=run_distributed_inference, 
            args=(rank, world_size)
        )
        p.start()
        processes.append(p)

    # Wait for all processes to complete
    for p in processes:
        p.join()

if __name__ == "__main__":
    main()