from transformers import AutoModelForCausalLM, AutoTokenizer

model_id = "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"
tokenizer = AutoTokenizer.from_pretrained(model_id)
model = AutoModelForCausalLM.from_pretrained(
    model_id,
)

prompt = "Your input prompt here."
inputs = tokenizer(prompt, return_tensors="pt").to("cpu")
outputs = model.generate(**inputs, max_length=200)  # Increase max_length for more output text
generated_text = tokenizer.decode(outputs[0], skip_special_tokens=True)
print(generated_text)
