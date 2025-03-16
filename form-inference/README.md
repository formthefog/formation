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