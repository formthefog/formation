import os
import torch
import torch.distributed as dist

def init_distributed():
    """Initialize the distributed training environment (CPU-only with Gloo)"""
    # Initialize the process group using Gloo for CPU
    dist.init_process_group(backend="gloo")

    # Get global rank and world size
    global_rank = dist.get_rank()
    world_size = dist.get_world_size()

    # For CPU, device is always "cpu"
    device = torch.device("cpu")

    return global_rank, world_size, device

def cleanup_distributed():
    """Clean up the distributed environment"""
    dist.destroy_process_group()

def main():
    # Initialize distributed environment
    global_rank, world_size, device = init_distributed()

    print(f"Running on rank {global_rank}/{world_size - 1}, device: {device}")

    """Your CPU-only training logic here"""

    # Clean up distributed environment
    cleanup_distributed()

if __name__ == "__main__":
    main()