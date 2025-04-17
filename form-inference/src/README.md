# AI Infrastructure

To integrate llama.cpp, DeepSeek, and LocalAI, here's how they fit together:

1. **DeepSeek (AI Model)**
    - Purpose: DeepSeek provides open-source AI models (like DeepSeek LLMs) for tasks like chat, code generation, and search.
    - Usage: Instead of using LLaMA models, you can run DeepSeek models within llama.cpp.
    - How It Fits: DeepSeek models (quantized versions) can be loaded and executed in llama.cpp.
2. **llama.cpp (Inference Engine)**
    - Purpose: Runs large language models (LLMs) locally using CPU/GPU.
    - Usage: Efficient for running Meta's LLaMA and other compatible models on local hardware.
    - How It Fits: Acts as the backend for running models, processing input prompts, and generating responses.
3. **LocalAI (API Wrapper & Management)**
    - Purpose: Provides an OpenAI-like API layer for running local AI models.
    - Usage: Allows easier integration with apps, chatbots, and services that are already built on top of OpenAI-compatible APIs.
    - How It Fits: Wraps around llama.cpp to serve AI responses via an OpenAI-style API, making it accessible to applications (making it usable as the basis for an AI agent).

## Llama.cpp

[Useful guide for setup.](https://blog.steelph0enix.dev/posts/llama-cpp-guide/)

1. Let’s start by grabbing a copy of llama.cpp source code, and moving into it.

    ```
    git clone https://github.com/ggerganov/llama.cpp.git
    cd llama.cpp
    git submodule update --init --recursive
    ```

2. Now we’ll use CMake to generate build files. Run the following command to generate build files in build/ subdirectory:

    ```
    sudo apt install cmake
    sudo apt install ninja-build
    cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$HOME/.local -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=ON -DLLAMA_BUILD_SERVER=ON
    ```

3. Now let's build the project from the build files. Determine how many cores you have using `nproc` to find `X` and then run:
    ```
    cmake --build build --config Release -j X
    ```

4. Finally, let's install the project in the `CMAKE_INSTALL_PREFIX` location and ensure that you have set the path for local libraries for binaries:
    ```
    cmake --install build --config Release
    echo 'export LD_LIBRARY_PATH=$HOME/.local/lib:$LD_LIBRARY_PATH' >> ~/.bashrc
    echo 'export PATH=$HOME/.local/bin:$PATH' >> ~/.bashrc
    source ~/.bashrc
    ```

5. This produces a number of executables in `$HOME/.local/bin` that we will use.

## Deep Seek

Ensure you have installed Python 3.10 before running these commands.

1. We use the 1.7B distilled version of the famed Deep Seek R1 model available at this [link](https://huggingface.co/deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B?clone=true) via Hugging Face.
    ```
    GIT_LFS_SKIP_SMUDGE=1 git clone https://huggingface.co/deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B
    ```

2. You must manually download the `model.safetensors` file for the model and replace the temporary version in the folder with this larger version.

3. Next, we want to run the script `convert_hf_to_gguf.py` in the bin above to convert the Hugging Face model to a GGUF format that can be used by our inference engine `Llama.cpp`. To do this, we must install all necessary packages.
    ```
    python3 -m venv venv
    
    source ./venv/bin/activate
    
    pip install -r llama.cpp/requirements/requirements-convert_hf_to_gguf.txt
    
    python llama.cpp/convert_hf_to_gguf.py DeepSeek-R1-Distill-Qwen-1.5B/ --outfile ./DeepSeek-R1-Distill-Qwen.gguf
    ```
4. Now we have to quantize the model. Quantizing a model means converting its numerical weights and activations from higher precision (e.g., 32-bit floating point (FP32)) to lower precision (e.g., 16-bit floating point (FP16), 8-bit integer (INT8), or even 4-bit/2-bit). This reduces model size and speeds up inference while attempting to maintain accuracy. But why Quantize?
    - Smaller Model Size → Uses less storage and memory (VRAM/RAM).
    - Faster Inference → Reduces computation requirements.
    - Lower Power Consumption → Useful for edge devices and mobile.

Here is what you have to do:

The rule of thumb for picking quantization type is “the largest I can fit in my VRAM (GPU)/RAM (CPU-only), unless it’s too slow for my taste”.

- Check available memory
  - GPU Users: Run `nvidia-smi` to check available VRAM
  - CPU Users: Run `free -h` to check available RAM
- Understand the model’s original size
  - GGUF file of SmolLM2 1.7B-Instruct in BF16 format weighs 3.4GB
  - Most models use BF16 (16-bit) or FP16 (16-bit) formats
  - Some rare models use FP32 (32-bit), which is significantly larger
- Identify quantization method
  - FP16 / BF16: 16-bit weights (baseline size)
  - Q8_0: 8-bit weights (~50% of original size)
  - Q6_K: 6-bit weights (~33% of original size)
  - Q4_K_M: 4-bit weights (~25% of original size)
- Compute estimated quantized model size
  - Multiply the original size by the ratio of quantized bits to 16
  - Example: Q8_0 (8-bit) → `3.4GB × (8/16) = ~1.7GB`
  - Example: Q6_K (6-bit) → `3.4GB × (6/16) = ~1.28GB`
  - Example: Q4_K_M (4-bit) → `3.4GB × (4/16) = ~0.85GB`
- Pick the largest quantized model that fits in available memory
  - GPU Users: Fit within VRAM unless it is too slow
  - CPU Users: Fit within RAM without forcing the OS to use swap
- Verify the quantized model size
  - Run `ls -lh model.gguf` to check actual file size

## Local AI

Following the [instructions here](https://localai.io/docs/getting-started/models/#run-models-manually) to run a model manually.

```
# Prepare the models into the `models` directory
mkdir models

# Copy your models to the directory
cp your-model.gguf models/

# Run the LocalAI container
docker run -p 8080:8080 -v $PWD/models:/models -ti --rm quay.io/go-skynet/local-ai:latest --models-path /models --context-size 700 --threads 4

# Expected output:
# ┌───────────────────────────────────────────────────┐
# │                   Fiber v2.42.0                   │
# │               http://127.0.0.1:8080               │
# │       (bound on host 0.0.0.0 and port 8080)       │
# │                                                   │
# │ Handlers ............. 1  Processes ........... 1 │
# │ Prefork ....... Disabled  PID ................. 1 │
# └───────────────────────────────────────────────────┘

# Test the endpoint with curl
curl http://localhost:8080/v1/completions -H "Content-Type: application/json" -d '{
     "model": "your-model.gguf",
     "prompt": "A long time ago in a galaxy far, far away",
     "temperature": 0.7
   }'
```

However, the following issue was encountered:

"I have a 1.5B param DeepSeek model running with llama.cpp, but having issues getting this to run via local-ai. Common issue reached here:

https://github.com/mudler/LocalAI/issues/800

Seems to be impacting even the Deepseek images I didn't build myself and got straight from the local-ai repository."

The error results in us having `Error rpc error: code = Unknown desc = unimplemented
` instead of receiving a response for any DeepSeek model run through Local AI - whether that be one built and quantized ourselves or one that is downloaded directly from the Local AI repository.