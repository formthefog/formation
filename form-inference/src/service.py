import requests
import signal
import psutil
from fastapi import FastAPI, BackgroundTasks
from pydantic import BaseModel
import uvicorn
import os
from dotenv import load_dotenv
from worker import run_worker

load_dotenv()

MOCK_URL = f"http://{os.getenv('MOCK_API_URL')}:{os.getenv('MOCK_API_PORT')}"

class Service:
    def __init__(self, host='127.0.0.1'):
        self.host = host
        self.port = None
        self.node_id = None
        self.app = FastAPI()

        @self.app.post("/generate")
        def generate(request: GenerateRequest):
            """Broadcast the request to all nodes."""
            print(f"Service: Received generate request. Broadcasting prompt: {request.input_data}")
            return self.broadcast_execute(request.input_data)

        @self.app.post("/execute")
        def execute(request: GenerateRequest, background_tasks: BackgroundTasks):
            """Each node executes the worker process asynchronously."""
            print(f"Service: Node {self.node_id} executing with prompt: {request.input_data}")
            rank, world_size = self.get_rank_and_world_size()

            # Run worker asynchronously
            background_tasks.add_task(run_worker, rank, world_size, request.input_data)

            return {"status": f"Node {self.node_id} started execution asynchronously"}

        self.initialize()  # Call initialize in the constructor

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
            print(f"Service: Rank: {rank}, World Size: {world_size}")
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

    def broadcast_execute(self, input_data):
        """Broadcasts the request to all nodes in the network to call /execute."""
        response = requests.get(f"{MOCK_URL}/nodes")
        if response.status_code != 200:
            return {"error": "Failed to retrieve active nodes."}

        nodes = response.json()
        responses = {}

        for node in nodes:
            node_id = node["id"]
            node_host = node.get("host", "127.0.0.1")
            node_port = 8000 + nodes.index(node)

            try:
                print(f"Service: Sending /execute request to {node_host}:{node_port} (Node {node_id})")
                resp = requests.post(
                    f"http://{node_host}:{node_port}/execute",
                    json={"input_data": input_data}
                )
                responses[node_id] = resp.json() if resp.status_code == 200 else "Failed"
            except Exception as e:
                responses[node_id] = f"Error: {str(e)}"

        print("Service: Broadcast completed.")
        return responses

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

if __name__ == "__main__":
    service = Service()
