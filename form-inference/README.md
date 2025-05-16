# Multi-GPU Inference Setup

This setup runs a multi-node, multi-GPU distributed PyTorch inference environment across remote machines using Docker, Ansible, and the official `pytorch/pytorch` CUDA 12.4 image.

We use SGLang to run models with both tensor and pipeline parallelism across our own GPU stack.

## Local

### SSH

Ensure you have two machines that each have a set of nodes. In our case, we tested this with two H200\*8 nodes.

node 1 (lucky-eft):
`ssh user@XXX.X.X.X`

node 2 (smooth-alien):
`ssh user@XXX.X.X.X`

### form-inference

The `src` directory contains essential configuration files for setting up and managing the distributed multi-GPU environment using Ansible. It includes:

- `machine_setup.yml`: An Ansible playbook that installs Docker and the NVIDIA Container Toolkit on the remote machines, ensuring they are ready for containerized workloads.
- `containers_setup.yml`: An Ansible playbook that deploys the PyTorch container cluster across the nodes, configuring network settings and launching the containers.
- `inventory.ini`: An inventory file that lists the remote machines (nodes) under the `[distributed]` group, specifying their IP addresses and user credentials for SSH access.

Ansible uses these files to automate the setup and deployment process across the nodes, ensuring a consistent and efficient environment for running distributed PyTorch inference tasks.

## Machine

The bare metal machine requires the installation and configuration of the following components to ensure optimal performance for distributed multi-GPU tasks.

This is defined in the ansible script. Run:

```
ansible-playbook -i src/inventory.ini src/machine/machine_setup.yml --ask-become-pass
```

### Docker

To install Docker on Ubuntu, follow these steps:

1. **Install Dependencies**:

   ```bash
   sudo apt update
   sudo apt install -y ca-certificates curl gnupg lsb-release
   ```

2. **Add Docker’s official GPG key**:

   ```bash
   sudo install -m 0755 -d /etc/apt/keyrings
   curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo tee /etc/apt/keyrings/docker.asc
   sudo chmod a+r /etc/apt/keyrings/docker.asc
   ```

3. **Set up the Docker repository**:

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

You need to ensure that we have the following (otherwise we risk 802 errors for incompatible setups):

- **Driver Version**: 550.144.03
- **CUDA Version**: 12.4
- **NVIDIA Fabric Manager**: Ensures proper management of GPU resources across multiple nodes.

1. **Install NVIDIA Container Toolkit**:

   ```bash
   sudo apt update
   sudo apt install -y nvidia-container-toolkit
   ```

2. **Configure NVIDIA Runtime**:

   ```bash
   sudo nvidia-ctk runtime configure --runtime=docker
   ```

3. **Restart Docker**:
   ```bash
   sudo systemctl restart docker
   ```

### Python

- **pyenv**: This tool is essential for managing different Python versions and ensuring they are compatible with your testing scripts. To install pyenv, execute the following command:
  ```bash
  curl https://pyenv.run | bash
  ```
  After installation, make sure to follow the instructions to add pyenv to your shell configuration file and restart your shell for the changes to take effect.

To verify that your setup is correct and that your GPU is properly configured, run the following Python command:

```python
python -c "import torch; print(torch.cuda.is_available()); print(torch.cuda.get_device_name(0))"
```

It should return `True`, indicating that CUDA is available, and also display the name of the GPU device.

## Container

After setting up the bare metal machines, the next goal is to create docker containers with envrionements capable of running our models on top of the infrastructure we have created.

These environments can be setup and deployed using the following ansible script. Run:

```bash
ansible-playbook -i src/inventory.ini src/containers/container_setup.yml --ask-become-pass
```

### Container Setup

- The `containers_setup.yml` file is an Ansible playbook designed to deploy a torch container cluster across distributed hosts. It sets up necessary variables, ensures the existence of a workspace directory, creates a custom Docker network if it doesn't exist, copies a Docker Compose template, and launches the container using Docker Compose. The playbook is configured to run with elevated privileges and is designed to work with a group of distributed hosts defined in an inventory file.
- The `docker-compose.yml.j2` file defines the configuration for deploying a PyTorch container using Docker. It specifies the image, container name, restart policy, runtime, network mode, environment variables, working directory, and volumes. The container is set up to run a series of commands, including updating the package list, installing necessary packages, cloning a GitHub repository, setting up a Python virtual environment, and installing Python packages. The environment contains the following variables required to make connections work between the two nodes.

| Variable           | Description                                                                                                  |
| ------------------ | ------------------------------------------------------------------------------------------------------------ |
| PRIMARY_ADDR       | The primary address of the node, used for communication between nodes.                                       |
| PRIMARY_PORT       | The primary port used for communication between nodes.                                                       |
| MASTER_ADDR        | The address of the master node, typically the same as the primary address.                                   |
| MASTER_PORT        | The port of the master node, typically the same as the primary port.                                         |
| NODE_ADDR          | The address of the current node.                                                                             |
| NODE_RANK          | The rank or index of the current node in the cluster.                                                        |
| NUM_NODES          | The total number of nodes in the cluster.                                                                    |
| NUM_TRAINERS       | The number of trainers or processes per node.                                                                |
| HOST_NODE_ADDR     | The combined address and port of the host node.                                                              |
| WORLD_SIZE         | The total number of processes across all nodes, calculated as the product of `NUM_NODES` and `NUM_TRAINERS`. |
| GLOO_SOCKET_IFNAME | The network interface name used by Gloo for communication.                                                   |
| NCCL_SOCKET_IFNAME | The network interface name used by NCCL for communication.                                                   |

### GPU Connections

This section was inspired by the documentation available at https://docs.runpod.io/instant-clusters/pytorch.

Assuming you have run both of these scripts, both containers should now be running on each node. You can run the following command to enter the container environment on each node:

```bash
sudo docker exec -it torch-runner /bin/bash
```

If all works well, you should be able to run both of these commands on each node and output prints from each GPU registering that commmunication is working effectively:

- On node 1 run:

```bash
torchrun --nproc_per_node=$NUM_TRAINERS --nnodes=$NUM_NODES --node_rank=$NODE_RANK   --master_addr=$MASTER_ADDR --master_port=$MASTER_PORT torch-demo/main.py
```

- On node 2 run:

```bash
torchrun --nproc_per_node=$NUM_TRAINERS --nnodes=$NUM_NODES --node_rank=$NODE_RANK   --master_addr=$MASTER_ADDR --master_port=$MASTER_PORT torch-demo/main.py
```

Consider using `nc` (netcat) to verify if TCP connections are open. Ensure that port 29500 is open on each node, and also check that ports in the range 32768–60999 are accessible.

### Downloading the Model

Run the following in the `/workspace` folder in the container. This downloads the model into the workspace which will persist across runs that we can run after this is complete.

```bash
source venv/bin/activate
pip install huggingface_hub
huggingface-cli download deepseek-ai/DeepSeek-R1-Distill-Llama-70B --local-dir ./models/deepseek-ai/DeepSeek-R1-Distill-Llama-70B
```

### Running the Model

https://github.com/sgl-project/sglang/tree/main/benchmark/deepseek_v3#example-serving-with-two-h208-nodes

Run the following on each node:

```bash
python3 -m sglang.launch_server --model-path ./models/deepseek-ai/DeepSeek-R1-Distill-Llama-70B --tp $NUM_TRAINERS --dist-init-addr $MASTER_ADDR:$MASTER_PORT --nnodes $NUM_NODES --node-rank $NODE_RANK --trust-remote-code
```

You can only query the master node, not a child:

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
