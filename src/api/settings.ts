import { invoke } from '@tauri-apps/api/core'
import type { Settings } from '../atoms'

export const load = () => invoke<Settings>('load_settings')

export const save = (settings: Settings) => invoke('save_settings', { settings })

export const testProvider = (settings: Settings) =>
  invoke<string>('test_provider', { settings })
