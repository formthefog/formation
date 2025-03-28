from fastapi import FastAPI
from concurrent.futures import ProcessPoolExecutor
from concurrent.futures import ThreadPoolExecutor
import asyncio
import requests
import signal
import os
from pydantic import BaseModel
import uvicorn
from dotenv import load_dotenv
from factory import create_worker
from model import Model

load_dotenv()

MOCK_URL = f"http://{os.getenv('MOCK_API_URL')}:{os.getenv('MOCK_API_PORT')}"
executor = ProcessPoolExecutor()

def broadcast_execute(input_data, world_size, model=None):
    """Broadcasts the execute request to all nodes and ensures parallel execution."""
    response = requests.get(f"{MOCK_URL}/nodes")
    if response.status_code != 200:
        return {"error": "Failed to retrieve active nodes."}

    nodes = response.json()
    responses = {}

    # Create thread pool with the exact number of nodes
    with ThreadPoolExecutor(max_workers=world_size) as thread_executor:
        futures = {}

        for node in nodes:
            node_id = node["id"]
            node_host = node.get("host", "127.0.0.1")
            node_port = 8000 + nodes.index(node)

            try:
                print(f"Service: Sending /execute request to {node_host}:{node_port} (Node {node_id})")
                # Submit request to each node in a separate thread
                future = thread_executor.submit(
                    requests.post,
                    f"http://{node_host}:{node_port}/execute",
                    json={"input_data": input_data, "model": model if model else ""}
                )
                futures[node_id] = future

            except Exception as e:
                responses[node_id] = f"Error: {str(e)}"

        # Wait for all nodes to finish execution
        for node_id, future in futures.items():
            try:
                resp = future.result()  # Wait for the response from each node
                responses[node_id] = resp.json() if resp.status_code == 200 else "Failed"
            except Exception as e:
                responses[node_id] = f"Error while processing: {str(e)}"

    print("Service: Broadcast completed.")
    return responses

class Service:
    def __init__(self, host='127.0.0.1'):
        self.host = host
        self.port = None
        self.node_id = None
        self.app = FastAPI()

        @self.app.post("/generate")
        async def generate(request: GenerateRequest):
            """Ensure a fresh process every time generate is called."""
            print(f"Service: Received generate request. Broadcasting prompt: {request.input_data}")

            rank, world_size = self.get_rank_and_world_size()

            # Create a new executor every time
            process_executor = ProcessPoolExecutor(max_workers=1)
            loop = asyncio.get_running_loop()
            result = await loop.run_in_executor(process_executor, broadcast_execute, request.input_data, world_size, request.model)

            # Forcefully terminate the process pool to avoid reuse
            process_executor.shutdown(wait=True, cancel_futures=True)

            return result[self.node_id]

        @self.app.post("/execute")
        def execute(request: GenerateRequest):
            """Each node directly runs its worker inside the request handler."""
            print(f"Service: Node {self.node_id} executing with prompt: {request.input_data}")
            rank, world_size = self.get_rank_and_world_size()

            # Directly run the worker (no additional threading needed)
            worker = create_worker(rank, Model(request.model))
            print(f"Worker {rank}: {worker.model}")
            result = worker.run(world_size, request.input_data)

            return result

        self.initialize()

    def initialize(self):
        if not self.node_id:
            print("Service: Initializing...")
            response = requests.post(f"{MOCK_URL}/nodes")
            if response.status_code == 201:
                self.node_id = response.json().get("node", {}).get("id")
                print(f"Service: Node added with ID: {self.node_id}")
            else:
                print("Service: Failed to add node.")
            print("Service: Initialization complete.")
            self.run_api()

    def get_rank_and_world_size(self):
        """Fetch the rank and world size from the mock API."""
        response = requests.get(f"{MOCK_URL}/nodes")
        if response.status_code == 200:
            nodes = response.json()
            node_ids = [node["id"] for node in nodes]
            rank = node_ids.index(self.node_id)
            world_size = len(node_ids)
            print(f"Service: Rank: {rank}")
        else:
            print("Service: Failed to retrieve active nodes.")
            return None
        return rank, world_size

    def run_api(self):
        rank, _ = self.get_rank_and_world_size()
        self.port = 8000 + rank

        print(f"Service: Running API on {self.host}:{self.port}")
        signal.signal(signal.SIGINT, self._handle_interrupt)
        try:
            uvicorn.run(self.app, host=self.host, port=self.port, log_level="warning")
        except KeyboardInterrupt:
            print("Service: Stopping due to keyboard interrupt.")
            self.cleanup()

    def _handle_interrupt(self, sig, frame):
        print("Service: Caught Ctrl+C, cleaning up...")
        self.cleanup()
        exit(0)

    def cleanup(self):
        if self.node_id:
            print(f"Service: Deleting node with ID: {self.node_id}")
            response = requests.delete(f"{MOCK_URL}/nodes/{self.node_id}")
            if response.status_code == 204:
                print("Service: Node deleted successfully.")
            else:
                print("Service: Failed to delete node.")

class GenerateRequest(BaseModel):
    input_data: str
    model: str = None
