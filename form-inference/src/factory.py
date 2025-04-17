from device import Device
from model import Model
from workers.tiny.gpt2_worker import GPT2Worker
from workers.tiny.deepseek_worker import DeepSeekWorker


def create_worker(rank: int, model: Model = None, device: str = Device.CPU):
    if not isinstance(model, Model):
        raise ValueError("Invalid model type. Expected a Model instance.")
    
    if model == Model.GPT2:
        print("Creating GPT2Worker")
        return GPT2Worker(rank, model, device)
    elif model == Model.DEEPSEEK:
        print("Creating DeepSeekWorker")
        return DeepSeekWorker(rank, model, device)
    else:
        print("Creating default GPT2Worker")
        return GPT2Worker(rank, model, device)