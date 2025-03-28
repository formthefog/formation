import time
import torch.distributed as dist
from transformers import AutoModelForCausalLM, AutoTokenizer
from device import Device
from model import Model
from worker import Worker  # Assuming Worker is defined in worker.py


class DeepSeekWorker(Worker):
    def __init__(self, rank: int, model: Model, device: Device):
        super().__init__(rank, model, device)

    def run(self, world_size, input_data):
        start_time = time.time()
        try:
            self.init_distributed(world_size)
            print(f"Worker {self.rank}: Ready to process requests...")
            model_id = "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"
            tokenizer = AutoTokenizer.from_pretrained(model_id)
            model = AutoModelForCausalLM.from_pretrained(
                model_id,
            )

            prompt = input_data
            inputs = tokenizer(prompt, return_tensors="pt").to(self.device)
            outputs = model.generate(**inputs, max_new_tokens=100, do_sample=True)
            received_text = tokenizer.decode(outputs[0], skip_special_tokens=True)
            
            print(f"Worker {self.rank}: Generated text: {received_text}")
        finally:
            dist.destroy_process_group()
            print(f"Worker {self.rank}: Process group destroyed.")
            end_time = time.time()
            print(
                f"Worker {self.rank}: Execution time: {end_time - start_time:.2f} seconds"
            )
            return received_text
