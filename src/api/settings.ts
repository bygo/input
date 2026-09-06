import { invoke } from '@tauri-apps/api/core'

export interface AppSettings {
  localModel: boolean
  llmModel?: string
  dictPack?: string
  targetLanguage: string
  nativeLanguage: string
  composeShortcut: string
  selectShortcut: string
  ocrShortcut: string
  selectSpeak: boolean
}

export const loadSettings = () => invoke<AppSettings>('load_settings')

export function setShortcut(kind: 'compose' | 'select' | 'ocr', shortcut: string) {
  return invoke<AppSettings>('set_shortcut', { kind, shortcut })
}

export function setSelectSpeak(enabled: boolean) {
  return invoke<AppSettings>('set_select_speak', { enabled })
}

export const beginShortcutCapture = () => invoke('begin_shortcut_capture')

export const endShortcutCapture = () => invoke('end_shortcut_capture')
