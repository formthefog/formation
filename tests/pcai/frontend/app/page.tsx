"use client"
import React, { useState, useEffect, useCallback } from 'react';
import { AlertCircle, Play, Pause } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

interface Question {
  text: string;
  response?: string;
  follow_ups: string[];
  responses?: string[];
}

interface Conversation {
  asking_llm: number;
  current_question: Question;
  conversation_history: Question[];
}

interface Config {
  llm1_config: {
    provider: 'openai' | 'anthropic';
    api_key: string;
    model_name: string;
  };
  llm2_config: {
    provider: 'openai' | 'anthropic';
    api_key: string;
    model_name: string;
  };
}

interface LLMConfigProps {
  llmNum: 1 | 2;
  config: Config;
  updateConfig: (llmNum: 1 | 2, field: keyof Config['llm1_config'], value: string) => void;
}

const LLMConfig = ({ llmNum, config, updateConfig }: LLMConfigProps) => (
  <div className="space-y-4">
    <h3 className="font-medium">LLM {llmNum} Configuration</h3>
    <Select
      value={config[`llm${llmNum}_config`].provider}
      onValueChange={(value) => updateConfig(llmNum, 'provider', value)}
    >
      <SelectTrigger>
        <SelectValue placeholder="Select provider" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="openai">OpenAI</SelectItem>
        <SelectItem value="anthropic">Anthropic</SelectItem>
      </SelectContent>
    </Select>

    <input
      type="password"
      placeholder="API Key"
      className="w-full p-2 border rounded"
      value={config[`llm${llmNum}_config`].api_key}
      onChange={(e) => updateConfig(llmNum, 'api_key', e.target.value)}
    />

    <input
      type="text"
      placeholder="Model Name (optional)"
      className="w-full p-2 border rounded"
      value={config[`llm${llmNum}_config`].model_name}
      onChange={(e) => updateConfig(llmNum, 'model_name', e.target.value)}
    />
  </div>
);

const CuriousAI = () => {
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [config, setConfig] = useState<Config>({
    llm1_config: {
      provider: 'openai',
      api_key: '',
      model_name: ''
    },
    llm2_config: {
      provider: 'anthropic',
      api_key: '',
      model_name: ''
    }
  });

  const nextStep = useCallback(async () => {
    try {
      const response = await fetch('/api/next', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      });
      
      if (!response.ok) throw new Error('Failed to progress conversation');
      
      const data = await response.json();
      setConversation(data);
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('An unknown error occurred');
      }
      setIsRunning(false);
    }
  }, [config]);

  useEffect(() => {
    let interval: NodeJS.Timeout | undefined;
    if (isRunning) {
      interval = setInterval(nextStep, 5000);
    }
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isRunning, nextStep]);

  const startConversation = async () => {
    try {
      const response = await fetch('/api/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      });
      
      if (!response.ok) throw new Error('Failed to start conversation');
      
      const data = await response.json();
      setConversation(data);
      setIsRunning(true);
      setError(null);
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('An unknown error occurred');
      }
    }
  };

  const updateConfig = (llmNum: 1 | 2, field: keyof Config['llm1_config'], value: string) => {
    setConfig(prev => ({
      ...prev,
      [`llm${llmNum}_config`]: {
        ...prev[`llm${llmNum}_config`],
        [field]: value
      }
    }));
  };

  return (
    <div className="container mx-auto p-4">
      <div className="max-w-4xl mx-auto">
        <Card className="mb-6">
          <CardHeader>
            <CardTitle>Perpetually Curious AI</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-8 mb-6">
              <LLMConfig llmNum={1} config={config} updateConfig={updateConfig} />
              <LLMConfig llmNum={2} config={config} updateConfig={updateConfig} />
            </div>
            
            <div className="flex justify-between items-center mb-4">
              <button
                onClick={startConversation}
                disabled={isRunning}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
              >
                Start New Conversation
              </button>
              
              <button
                onClick={() => setIsRunning(!isRunning)}
                className="p-2 rounded hover:bg-gray-100"
              >
                {isRunning ? <Pause className="w-6 h-6" /> : <Play className="w-6 h-6" />}
              </button>
            </div>
          </CardContent>
        </Card>

        {error && (
          <Alert variant="destructive" className="mb-6">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {conversation && (
          <div className="space-y-4">
            <div className="bg-white rounded-lg shadow-lg p-4 border-2 border-blue-500">
              <div className="mb-4">
                <span className="font-semibold">
                  LLM {conversation.asking_llm} asks:
                </span>
                <p className="text-lg">{conversation.current_question.text}</p>
              </div>
              
              {conversation.current_question.response && (
                <div className="ml-4 mb-4 p-2 bg-gray-50 rounded border-l-4 border-gray-200">
                  <span className="font-semibold">
                    LLM {conversation.asking_llm === 1 ? 2 : 1} responds:
                  </span>
                  <p className="mt-1">{conversation.current_question.response}</p>
                </div>
              )}
              
              {conversation.current_question.follow_ups.map((followUp, j) => (
                <div key={j} className="space-y-3">
                  <div className="ml-8 p-2 bg-blue-50 rounded border-l-4 border-blue-200">
                    <span className="font-semibold">Follow-up Question:</span>
                    <p className="mt-1">{followUp}</p>
                  </div>
                  {conversation.current_question.responses && j < conversation.current_question.responses.length && (
                    <div className="ml-12 p-2 bg-gray-50 rounded border-l-4 border-gray-200">
                      <span className="font-semibold">Response:</span>
                      <p className="mt-1">{conversation.current_question.responses[j]}</p>
                    </div>
                  )}
                </div>
              ))}
            </div>

            {conversation.conversation_history.map((q, i) => (
              <div key={i} className="bg-white rounded-lg shadow p-4 opacity-75">
                <div className="mb-4">
                  <span className="font-semibold">
                    Previous Discussion:
                  </span>
                  <p className="text-lg mt-1">{q.text}</p>
                </div>
                
                {q.response && (
                  <div className="ml-4 mb-4 p-2 bg-gray-50 rounded border-l-4 border-gray-200">
                    <span className="font-semibold">
                      Response:
                    </span>
                    <p className="mt-1">{q.response}</p>
                  </div>
                )}
                
                {q.follow_ups.map((followUp, j) => (
                  <div key={j} className="space-y-3">
                    <div className="ml-8 p-2 bg-blue-50 rounded border-l-4 border-blue-200">
                      <span className="font-semibold">Follow-up:</span>
                      <p className="mt-1">{followUp}</p>
                    </div>
                    {q.responses && j < q.responses.length && (
                      <div className="ml-12 p-2 bg-gray-50 rounded border-l-4 border-gray-200">
                        <span className="font-semibold">Response:</span>
                        <p className="mt-1">{q.responses[j]}</p>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default CuriousAI;
