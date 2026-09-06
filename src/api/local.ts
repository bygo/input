import { invoke } from '@tauri-apps/api/core'

export interface ModelRow {
  id: string
  label: string
  installed: boolean
}

export interface LocalStatus {
  enabled: boolean
  installed: boolean
  running: boolean
  hasModel: boolean
  hasLlm?: boolean
  model: string
  models: ModelRow[]
  busy: boolean
  message: string
  percent: number
}

export const localStatus = () => invoke<LocalStatus>('local_model_status')

export const installLocal = () => invoke('install_local_model')

export const setLocal = (enabled: boolean) => invoke('set_local_model', { enabled })

export const setLlmModel = (id: string) => invoke<LocalStatus>('set_llm_model', { id })

export const pullLlmModel = (id: string) => invoke<LocalStatus>('pull_llm_model', { id })

export const deleteLlmModel = (id: string) => invoke<LocalStatus>('delete_llm_model', { id })
