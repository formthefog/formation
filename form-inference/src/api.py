from fastapi import FastAPI
from pydantic import BaseModel
import torch
import torch.distributed as dist
from transformers import DistilBertTokenizer
import os
import psutil
import signal
import sys

app = FastAPI()

class InputText(BaseModel):
    text: str

def init_distributed():
    os.environ['MASTER_ADDR'] = 'localhost'
    os.environ['MASTER_PORT'] = '12345'  # Ensure consistent port
    os.environ['RANK'] = '0'  # Rank 0 for API
    os.environ['WORLD_SIZE'] = '3'

    print(f"API (Rank 0) initializing process group at {os.environ['MASTER_ADDR']}:{os.environ['MASTER_PORT']}")
    
    dist.init_process_group(backend='gloo', rank=0, world_size=3)

    print("API (Rank 0) process group initialized")

# Initialize at startup
init_distributed()

p = psutil.Process()
p.cpu_affinity([0])
print(f"API running on core: {p.cpu_affinity()}")

def signal_handler(sig, frame):
    print("API: Caught Ctrl+C, exiting...")
    if dist.is_initialized():
        dist.destroy_process_group()
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)

@app.post("/predict")
async def predict(input_text: InputText):
    tokenizer = DistilBertTokenizer.from_pretrained("distilbert-base-uncased")
    
    inputs = tokenizer(input_text.text, return_tensors="pt", padding=True, truncation=True)
    
    print(f"API (Rank 0): Sending input_ids to Rank 1 -> Shape: {inputs['input_ids'].shape}")
    
    dist.send(inputs["input_ids"], 1)  # Ensure this is sending
    
    result = torch.zeros(1, dtype=torch.long)
    
    print("API (Rank 0): Waiting to receive result from Rank 2...")
    dist.recv(result, 2)  # Ensure this is receiving
    print(f"API (Rank 0): Received result: {result.item()}")

    return {"label": result.item(), "meaning": "positive" if result.item() == 1 else "negative"}



