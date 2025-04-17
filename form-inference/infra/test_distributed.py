import os
import torch
import torch.distributed as dist

def init_distributed():
   """Initialize the distributed training environment"""
   # Initialize the process group
   dist.init_process_group(backend="nccl")
   
   # Get local rank and global rank
   local_rank = int(os.environ["LOCAL_RANK"])
   global_rank = dist.get_rank()
   world_size = dist.get_world_size()
   
   # Set device for this process
   device = torch.device(f"cuda:{local_rank}")
   torch.cuda.set_device(device)
       
   return local_rank, global_rank, world_size, device

def cleanup_distributed():
   """Clean up the distributed environment"""
   dist.destroy_process_group()

def main():
   # Initialize distributed environment
   local_rank, global_rank, world_size, device = init_distributed()
   
   print(f"Running on rank {global_rank}/{world_size-1} (local rank: {local_rank}), device: {device}")

   # Your code here
   
   # Clean up distributed environment when done
   cleanup_distributed()
   
if __name__ == "__main__":
   main()