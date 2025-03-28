from abc import abstractmethod
import torch
import torch.distributed as dist
import os
from model import Model
from device import Device

class Worker:
    def __init__(self, rank: int, model: Model, device: Device = Device.CPU):
        self.rank = rank
        self.model = model
        self.device = torch.device(device.value)

    def init_distributed(self, world_size):
        os.environ["MASTER_ADDR"] = "localhost"
        os.environ["MASTER_PORT"] = "12345"
        print(f"Worker {self.rank}: Initializing process group...")
        dist.init_process_group(backend="gloo", rank=self.rank, world_size=world_size)
        print(f"Worker {self.rank}: Process group initialized.")

    @abstractmethod
    def run(self, world_size, input_data):
        pass