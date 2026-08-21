import { invoke } from '@tauri-apps/api/core'

export type LocalStatus = {
  enabled: boolean
  installed: boolean
  running: boolean
  hasModel: boolean
  busy: boolean
  message: string
  percent: number
}

export const localStatus = () => invoke<LocalStatus>('local_model_status')

export const installLocal = () => invoke('install_local_model')

export const setLocal = (enabled: boolean) => invoke('set_local_model', { enabled })
