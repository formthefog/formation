apt update && apt install tmux
mv /root/.cache /workspace/.cache && ln -s /workspace/.cache /root/.cache
pip install -U vLLM
pip install --upgrade pyzmq
vllm serve deepseek-ai/DeepSeek-R1-Distill-Llama-8B --tensor-parallel-size 2