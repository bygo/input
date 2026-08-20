import type { ReactNode } from 'react'

export type ProviderId = 'claude' | 'openai' | 'openrouter' | 'spacexai' | 'ollama'

export type ModelOption = { id: string; label: string }

export type Provider = {
  id: ProviderId
  name: string
  baseUrl: string
  keyHint: string
  models: ModelOption[]
}

export const PROVIDERS: Provider[] = [
  {
    id: 'claude',
    name: 'Claude',
    baseUrl: 'https://api.anthropic.com/v1',
    keyHint: 'sk-ant-…',
    models: [
      { id: 'claude-sonnet-4-5', label: 'Sonnet 4.5' },
      { id: 'claude-opus-4-1', label: 'Opus 4.1' },
      { id: 'claude-haiku-4-5', label: 'Haiku 4.5' },
    ],
  },
  {
    id: 'openai',
    name: 'GPT',
    baseUrl: 'https://api.openai.com/v1',
    keyHint: 'sk-…',
    models: [
      { id: 'gpt-4.1', label: 'GPT-4.1' },
      { id: 'gpt-4o', label: 'GPT-4o' },
      { id: 'gpt-4o-mini', label: 'GPT-4o mini' },
      { id: 'o4-mini', label: 'o4-mini' },
    ],
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    keyHint: 'sk-or-…',
    models: [
      { id: 'anthropic/claude-sonnet-4.5', label: 'Claude Sonnet 4.5' },
      { id: 'anthropic/claude-opus-4.1', label: 'Claude Opus 4.1' },
      { id: 'openai/gpt-4o', label: 'GPT-4o' },
      { id: 'openai/gpt-4o-mini', label: 'GPT-4o mini' },
      { id: 'google/gemini-2.5-flash', label: 'Gemini 2.5 Flash' },
      { id: 'x-ai/grok-4.5', label: 'Grok 4.5' },
    ],
  },
  {
    id: 'spacexai',
    name: 'Grok',
    baseUrl: 'https://api.x.ai/v1',
    keyHint: 'xai-…',
    models: [
      { id: 'grok-4.5', label: 'Grok 4.5' },
      { id: 'grok-3', label: 'Grok 3' },
      { id: 'grok-3-mini', label: 'Grok 3 mini' },
    ],
  },
  {
    id: 'ollama',
    name: 'Ollama',
    baseUrl: 'http://127.0.0.1:11434/v1',
    keyHint: '可留空',
    models: [
      { id: 'llama3.2', label: 'Llama 3.2' },
      { id: 'llama3.1', label: 'Llama 3.1' },
      { id: 'qwen2.5', label: 'Qwen 2.5' },
      { id: 'mistral', label: 'Mistral' },
    ],
  },
]

export function providerById(id: string) {
  return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[3]
}

function IconFrame({ color, children }: { color: string; children: ReactNode }) {
  return (
    <span
      className="flex size-8 items-center justify-center rounded-lg"
      style={{ background: color }}
    >
      {children}
    </span>
  )
}

export function ProviderIcon({ id }: { id: ProviderId }) {
  if (id === 'claude') {
    return (
      <IconFrame color="#C15F3C">
        <svg viewBox="0 0 24 24" className="size-4 fill-white">
          <path d="M12 2.8l1.2 7.2L20.5 12l-7.3 2 1.2 7.2L12 16.6 5.6 21.2 6.8 14 0 12l6.8-2L5.6 2.8 12 7.4z" transform="translate(1.7 0)" />
        </svg>
      </IconFrame>
    )
  }
  if (id === 'openai') {
    return (
      <IconFrame color="#10A37F">
        <svg viewBox="0 0 24 24" className="size-4 fill-white">
          <path d="M12 3.2a4 4 0 0 1 3.4 1.9 4 4 0 0 1 4.3 5.4 4 4 0 0 1-2.3 5.4 4 4 0 0 1-5.4 2.9A4 4 0 0 1 7 19.6a4 4 0 0 1-4.2-5.5A4 4 0 0 1 5 8.7 4 4 0 0 1 10.4 6 4 4 0 0 1 12 3.2z" />
        </svg>
      </IconFrame>
    )
  }
  if (id === 'openrouter') {
    return (
      <IconFrame color="#6566F1">
        <svg viewBox="0 0 24 24" className="size-4 stroke-white fill-none" strokeWidth="2">
          <circle cx="6" cy="12" r="2.2" fill="white" stroke="none" />
          <circle cx="18" cy="6.5" r="2.2" fill="white" stroke="none" />
          <circle cx="18" cy="17.5" r="2.2" fill="white" stroke="none" />
          <path d="M8 12h8M16.2 8.2 8.8 11.2M16.2 15.8 8.8 12.8" />
        </svg>
      </IconFrame>
    )
  }
  if (id === 'spacexai') {
    return (
      <IconFrame color="#111111">
        <svg viewBox="0 0 24 24" className="size-4 fill-white">
          <path d="M4 18 12 4l8 14h-3.2L12 9.6 7.2 18H4z" />
        </svg>
      </IconFrame>
    )
  }
  return (
    <IconFrame color="#222222">
      <svg viewBox="0 0 24 24" className="size-4 fill-white">
        <rect x="5" y="7" width="14" height="10" rx="2" />
        <path d="M8 7V5.5A4 4 0 0 1 16 5.5V7" fill="none" stroke="white" strokeWidth="1.8" />
      </svg>
    </IconFrame>
  )
}
