# form-inference

1. First run:
```
python3 src/form_mock.py
```

2. In two separate terminals, run:
```
python3 src/main.py
```

3. Run this curl command:
```
curl -X POST "http://127.0.0.1:8000/generate" -H "Content-Type: application/json" -d '{"input_data": "Whats the problem dog"}'
```

| Feature                | Pipeline Parallelism (Layer Sharding)                          | Tensor Parallelism (Generation Sharding)                      |
|------------------------|--------------------------------------------------------------|--------------------------------------------------------------|
| **Primary Goal**       | Divide model layers across workers                           | Distribute matrix multiplications and token generation across workers |
| **Sharding Strategy**  | Each worker gets a subset of layers                          | Each worker gets a subset of tensor operations               |
| **Data Flow**          | Data (hidden states) is passed sequentially between workers  | Workers collectively compute each layer and synchronize      |
| **Compute Distribution** | Each worker executes forward pass for assigned layers     | Each worker contributes to every forward/backward pass      |
| **Memory Usage**       | Lower per-worker memory usage as each stores only part of the model | Higher per-worker memory usage as each stores full weights of assigned layers |
| **Communication Overhead** | High, as hidden states must be passed between workers  | Lower than pipeline parallelism since activations stay local |
| **Scalability**        | Scales well for deep models                                  | Scales well for both training and inference                  |
| **Worker Role**        | Final worker performs generation, earlier workers only refine hidden states | All workers participate in both processing and generation |
| **Generation Execution** | Only the final worker runs `model.generate()`            | All workers contribute to `model.generate()` in parallel     |
| **Best Use Case**      | Large transformer models that don’t fit on a single device | Faster distributed generation when multiple GPUs are available |fff