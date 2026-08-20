import { atom } from 'jotai'

export type Issue = {
  span: string
  category: string
  severity: string
  explanation: string
  suggestion: string
}

export type Analysis = {
  corrected: string
  verdict: string
  issues: Issue[]
}

export type Settings = {
  shortcut: string
  provider: string
  baseUrl: string
  apiKey: string
  model: string
  keys: Record<string, string>
  lastModels: Record<string, string>
}

export const defaultSettings = (): Settings => ({
  shortcut: 'CommandOrControl+G',
  provider: 'spacexai',
  baseUrl: 'https://api.x.ai/v1',
  apiKey: '',
  model: 'grok-4.5',
  keys: {},
  lastModels: {},
})

export const textAtom = atom('')
export const sendingAtom = atom(false)
export const resultAtom = atom<Analysis | null>(null)
export const errorAtom = atom('')
export const viewAtom = atom<'compose' | 'settings'>('compose')
export const settingsAtom = atom<Settings>(defaultSettings())
