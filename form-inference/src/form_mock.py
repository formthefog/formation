from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import random
import string
import time
from dotenv import load_dotenv
import os

load_dotenv()

app = FastAPI()

class Node(BaseModel):
    id: str
    name: str
    public_key: str
    endpoint: str
    total_vcpus: int
    total_memory_mib: int
    total_disk_gb: int
    available_vcpus: int
    available_memory_mib: int
    available_disk_gb: int
    operator_id: str
    state: str
    created_at: int
    updated_at: int

nodes_list = []

def generate_random_string(length=10):
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))

@app.post("/nodes", status_code=201)
def add_node():
    node_id = f"node-{generate_random_string(9)}"
    name = f"Worker-{len(nodes_list) + 1}"
    public_key = generate_random_string(44)
    endpoint = f"203.0.113.{random.randint(1, 255)}:51820"
    total_vcpus = random.choice([8, 16, 32])
    total_memory_mib = random.choice([16384, 32768, 65536])
    total_disk_gb = random.choice([500, 1000, 2000])
    available_vcpus = random.randint(1, total_vcpus)
    available_memory_mib = random.randint(1024, total_memory_mib)
    available_disk_gb = random.randint(100, total_disk_gb)
    operator_id = f"peer-{generate_random_string(9)}"
    state = random.choice(["online", "offline"])
    current_time = int(time.time())
    
    node = Node(
        id=node_id,
        name=name,
        public_key=public_key,
        endpoint=endpoint,
        total_vcpus=total_vcpus,
        total_memory_mib=total_memory_mib,
        total_disk_gb=total_disk_gb,
        available_vcpus=available_vcpus,
        available_memory_mib=available_memory_mib,
        available_disk_gb=available_disk_gb,
        operator_id=operator_id,
        state=state,
        created_at=current_time,
        updated_at=current_time
    )
    nodes_list.append(node)
    return {"message": "Node added successfully", "node": node}

@app.get("/nodes/{node_id}", response_model=Node)
def get_node(node_id: str):
    for node in nodes_list:
        if node.id == node_id:
            return node
    raise HTTPException(status_code=404, detail="Node not found")

@app.get("/nodes", response_model=list[Node])
def list_nodes():
    return nodes_list

@app.delete("/nodes/{node_id}", status_code=204)
def delete_node(node_id: str):
    for index, node in enumerate(nodes_list):
        if node.id == node_id:
            del nodes_list[index]
            return {"message": "Node deleted successfully"}
    raise HTTPException(status_code=404, detail="Node not found")

if __name__ == "__main__":
    import uvicorn
    api_url = os.getenv("MOCK_API_URL", "127.0.0.1")
    api_port = int(os.getenv("MOCK_API_PORT", 5000))
    uvicorn.run(app, host=api_url, port=api_port)
