# Multi-GPU Inference Setup

This setup runs a multi-node, multi-GPU distributed PyTorch inference environment across remote machines using Docker, Ansible, and the official `pytorch/pytorch` CUDA 12.4 image.

We use SGLang to run models with both tensor and pipeline parallelism across our own GPU stack.



## Local

### SSH

To set up the distributed environment, you need two machines, each with a set of nodes. In our testing, we used two H200*8 nodes:

- Node 1: `ssh user@XXX.X.X.X`
- Node 2: `ssh user@XXX.X.X.X`

### Environment Setup

Before running the setup, you need to set the following environment variables:

```bash
# SSH Configuration
MACHINE1_IP=your_machine1_ip
MACHINE2_IP=your_machine2_ip
ANSIBLE_USER=your_ansible_user
```

You can add these to your `.env`.

### form-inference

The `src` directory contains three key configuration files for setting up the distributed multi-GPU environment:

1. `machine_setup.yml`: Ansible playbook for installing Docker and NVIDIA Container Toolkit on remote machines
2. `containers_setup.yml`: Ansible playbook for deploying the PyTorch container cluster across nodes
3. `inventory.ini`: Lists remote machines under the `[distributed]` group with their IP addresses and SSH credentials

## Machine

The bare metal machines require specific components for optimal distributed multi-GPU performance. The setup is automated through Ansible.

To set up the machines, run:
```bash
ansible-playbook -i src/inventory.ini src/machine/machine_setup.yml --ask-become-pass
```

### Docker

Install Docker on Ubuntu with these steps:

1. **Install Dependencies**:
   ```bash
   sudo apt update
   sudo apt install -y ca-certificates curl gnupg lsb-release
   ```

2. **Add Docker's GPG Key**:
   ```bash
   sudo install -m 0755 -d /etc/apt/keyrings
   curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo tee /etc/apt/keyrings/docker.asc
   sudo chmod a+r /etc/apt/keyrings/docker.asc
   ```

3. **Configure Docker Repository**:
   ```bash
   echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu \
   $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
   ```

4. **Install Docker Engine**:
   ```bash
   sudo apt update
   sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
   ```

### NVIDIA

Required NVIDIA components to prevent 802 errors:
- Driver Version: 550.144.03
- CUDA Version: 12.4
- NVIDIA Fabric Manager (version MUST match the driver version used)

Install NVIDIA components:

1. **Add NVIDIA GPG key and repository**:
   ```bash
   curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
   curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
   sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
   tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
   ```

2. **Install NVIDIA Container Toolkit**:
   ```bash
   sudo apt update
   sudo apt install -y nvidia-container-toolkit
   sudo nvidia-ctk runtime configure --runtime=docker
   sudo systemctl restart docker
   ```

3. **Install NVIDIA Driver and CUDA**:
   ```bash
   sudo apt install -y nvidia-driver-550
   sudo apt install -y cuda-toolkit-12-4
   ```

4. **Install and Configure NVIDIA Fabric Manager**:
   ```bash
   sudo apt install -y nvidia-fabricmanager-550=550.144.03-1
   sudo systemctl enable nvidia-fabricmanager
   sudo systemctl start nvidia-fabricmanager
   ```

### Python

Install pyenv for Python version management:
```bash
curl https://pyenv.run | bash
```

Verify GPU setup:
```python
python -c "import torch; print(torch.cuda.is_available()); print(torch.cuda.get_device_name(0))"
```
Expected output: `True` and your GPU device name.

## Container

After machine setup, create Docker containers for model deployment using:
```bash
ansible-playbook -i src/inventory.ini src/containers/container_setup.yml --ask-become-pass
```

### Container Setup

The container setup uses two main files:

1. `containers_setup.yml`: Ansible playbook that:
   - Sets up variables
   - Creates workspace directory
   - Configures Docker network
   - Deploys containers using Docker Compose

2. `docker-compose.yml.j2`: Defines container configuration including:
   - PyTorch image
   - Container settings
   - Environment variables
   - Volume mounts
   - Initial setup commands

Required Environment Variables:

| Variable           | Description                                    |
| ------------------ | ---------------------------------------------- |
| PRIMARY_ADDR       | Node's primary address for inter-node comms    |
| PRIMARY_PORT       | Primary port for inter-node comms              |
| MASTER_ADDR        | Master node address (usually = PRIMARY_ADDR)   |
| MASTER_PORT        | Master node port (usually = PRIMARY_PORT)      |
| NODE_ADDR          | Current node's address                         |
| NODE_RANK          | Node's rank in cluster                         |
| NUM_NODES          | Total nodes in cluster                         |
| NUM_TRAINERS       | Processes per node                             |
| HOST_NODE_ADDR     | Host node address:port                         |
| WORLD_SIZE         | Total processes (NUM_NODES × NUM_TRAINERS)     |
| GLOO_SOCKET_IFNAME | Network interface for Gloo comms               |
| NCCL_SOCKET_IFNAME | Network interface for NCCL comms               |

### GPU Connections

To access the container environment on each node:
```bash
sudo docker exec -it torch-runner /bin/bash
```

Test GPU communication on each node:

Node 1:
```bash
torchrun --nproc_per_node=$NUM_TRAINERS --nnodes=$NUM_NODES --node_rank=$NODE_RANK --master_addr=$MASTER_ADDR --master_port=$MASTER_PORT torch-demo/main.py
```

Node 2:
```bash
torchrun --nproc_per_node=$NUM_TRAINERS --nnodes=$NUM_NODES --node_rank=$NODE_RANK --master_addr=$MASTER_ADDR --master_port=$MASTER_PORT torch-demo/main.py
```

Verify connectivity:
- Check port 29500 is open
- Verify ports 32768-60999 are accessible
- Use `nc` (netcat) to test TCP connections and consider trying out utils/cpu.py if you believe there is a GPU error to debug but want to see if CPU connections work

### Downloading the Model

In the container's `/workspace` directory:
```bash
source venv/bin/activate
pip install huggingface_hub
huggingface-cli download deepseek-ai/DeepSeek-R1-Distill-Llama-70B --local-dir ./models/deepseek-ai/DeepSeek-R1-Distill-Llama-70B
```

### Running the Model

Start the server on each node:
```bash
python3 -m sglang.launch_server --model-path ./models/deepseek-ai/DeepSeek-R1-Distill-Llama-70B --tp $NUM_TRAINERS --dist-init-addr $MASTER_ADDR:$MASTER_PORT --nnodes $NUM_NODES --node-rank $NODE_RANK --trust-remote-code
```

Query the model (only from master node):
```bash
curl --request POST \
    --url http://127.0.0.1:30000/generate \
    --header 'Content-Type: application/json' \
    --data '{
    "text": "Write me a 1000 word story",
    "sampling_params": {
    "max_new_tokens": 5000,
    "temperature": 0.7
    },
    "stop_at_limit": false,
    "stream": true
    }'
```
