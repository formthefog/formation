# backend/main.py
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field
from typing import List, Optional, Literal
from langchain_openai import ChatOpenAI
from langchain_anthropic import ChatAnthropic
from langchain.schema import ChatResult, HumanMessage, AIMessage, SystemMessage
from langchain.prompts import ChatPromptTemplate
from langchain.output_parsers import PydanticOutputParser
import json

app = FastAPI()

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

class Question(BaseModel):
    text: str
    response: Optional[str] = None
    follow_ups: List[str] = []
    responses: List[str] = []

class Conversation(BaseModel):
    current_question: Question 
    asking_llm: int = 1 
    conversation_history: List[Question] = Field(default_factory=list)

class LLMConfig(BaseModel):
    provider: Literal["openai", "anthropic"]
    api_key: str
    model_name: str = ""  # Optional, will use default if not provided

def get_llm(config: LLMConfig):
    """Initialize the appropriate LLM based on provider."""
    if config.provider == "openai":
        model = config.model_name or "gpt-4"
        return ChatOpenAI(
            model=model,
            temperature=0.7
        )
    elif config.provider == "anthropic":
        model = config.model_name or "claude-3-opus-20240229"
        return ChatAnthropic(
            model_name=model,
            timeout=600,
            stop=list(),
            temperature=0.7
        )
    else:
        raise ValueError(f"Unsupported provider: {config.provider}")

class QuestionList(BaseModel):
    questions: List[str]

async def generate_question(llm) -> str:
    """Generate initial questions using LangChain."""
    try:
        messages = [
            {
                "role": "system",
                "content": "You are a curious AI that wants to learn more about various topics."
            },
            {
                "role": "user",
                "content": """What is one interesting question you would like to ask?  
                Make the question thought-provoking and avoid basic factual questions.
                Provide just the question itself, without any preamble or additional text."""
            }
        ]
        
        response = await llm.agenerate([messages])
        response_text = response.generations[0][0].text.strip()
        
        response_text = response_text.replace('"', '').strip()
        if response_text.lower().startswith("here's"):
            response_text = response_text.split(":", 1)[-1].strip()
        if response_text.lower().startswith("question:"):
            response_text = response_text.split(":", 1)[-1].strip()

        return response_text 

    except Exception as e:
        print(f"Debug - full error: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Failed to generate questions: {str(e)}")


async def get_response(question: str, llm) -> str:
    """Get response for a question using LangChain."""
    try:
        messages = [
            {
                "role": "system",
                "content": "You are a knowledgeable AI assistant engaged in a conversation with another AI. Provide clear, informative responses."
            },
            {
                "role": "user",
                "content": question
            }
        ]
        response = await llm.agenerate([messages])
        return response.generations[0][0].text
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to get response: {str(e)}")

async def get_follow_up_questions(response: str, llm) -> List[str]:
    """Get follow-up questions based on the response using LangChain."""
    try:
        messages = [
            {
                "role": "system",
                "content": "You are a curious AI analyzing a response. If you have a follow-up question, provide ONE. If you have no follow-up questions, respond with exactly 'no'."
            },
            {
                "role": "user",
                "content": f"Based on this response, do you have a follow-up question?\n\nResponse: {response}"
            }
        ]
        # Fix: Convert dict messages to HumanMessage/SystemMessage
        formatted_messages = [
            SystemMessage(content=messages[0]["content"]),
            HumanMessage(content=messages[1]["content"])
        ]
        response: ChatResult = await llm.agenerate([formatted_messages])
        follow_up = response.generations[0][0].text.strip()
        return [] if follow_up.lower() == "no" else [follow_up]
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to get follow-up questions: {str(e)}")

# Store conversation state (in memory for this example)
conversation_state = None

@app.post("/start")
async def start_conversation(llm1_config: LLMConfig, llm2_config: LLMConfig):
    """Start a new conversation between the LLMs."""
    global conversation_state
    try:
        llm1 = get_llm(llm1_config)
        question = await generate_question(llm1)
        conversation_state = Conversation(
                current_question=Question(text=question),
                asking_llm=1,
                conversation_history=[]
        )
        return conversation_state
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/next")
async def next_step(llm1_config: LLMConfig, llm2_config: LLMConfig):
    """Progress the conversation to the next step."""
    global conversation_state
    if not conversation_state:
        raise HTTPException(status_code=400, detail="Conversation not started")
    
    try:
        llm1 = get_llm(llm1_config)
        llm2 = get_llm(llm2_config)
        
        current_q = conversation_state.current_question
        asking_llm = llm1 if conversation_state.asking_llm == 1 else llm2
        responding_llm = llm2 if conversation_state.asking_llm == 1 else llm1
        
        # If question hasn't been answered yet, get response
        if not current_q.response:
            current_q.response = await get_response(current_q.text, responding_llm)
            return conversation_state
        
        # Get follow-up questions
        follow_ups = await get_follow_up_questions(current_q.response, asking_llm)
        
        if follow_ups:
            current_q.follow_ups.extend(follow_ups)
            response = await get_response(follow_ups[-1], responding_llm)
            current_q.responses.append(response)
            current_q.response = response
        else:
            # Move to next question
            conversation_state.conversation_history.append(current_q)
            conversation_state.asking_llm = 2 if conversation_state.asking_llm == 1 else 1
            new_asking_llm = llm2 if conversation_state.asking_llm == 1 else llm1
            new_question = await generate_question(new_asking_llm)
            conversation_state.current_question = Question(text=new_question) 
        
        return conversation_state
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/status")
async def get_status():
    """Get current conversation state."""
    if not conversation_state:
        raise HTTPException(status_code=400, detail="Conversation not started")
    return conversation_state
