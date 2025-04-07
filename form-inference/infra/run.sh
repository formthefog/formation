#!/bin/bash

# Default configuration for distributed training
export NCCL_DEBUG=${NCCL_DEBUG:-WARN}

# Number of GPUs per node (default to 1)
NUM_TRAINERS=${NUM_TRAINERS:-1}

# Total number of nodes (default to 2)
NUM_NODES=${NUM_NODES:-2}

# Node rank (default to 0 for master)
NODE_RANK=${NODE_RANK:-0}

# Master address (default to localhost)
MASTER_ADDR=${MASTER_ADDR:-localhost}

# Fixed port for distributed training
MASTER_PORT=${MASTER_PORT:-29500}


# Run the training script using torchrun
torchrun \
  --nproc_per_node=$NUM_TRAINERS \
  --nnodes=$NUM_NODES \
  --node_rank=$NODE_RANK \
  --master_addr=$MASTER_ADDR \
  --master_port=$MASTER_PORT \
  test_distributed.py