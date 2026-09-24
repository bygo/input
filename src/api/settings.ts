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
  showNative: boolean
  autoGrammar: boolean
}

export const loadSettings = () => invoke<AppSettings>('load_settings')

export function setShortcut(kind: 'compose' | 'select' | 'ocr', shortcut: string) {
  return invoke<AppSettings>('set_shortcut', { kind, shortcut })
}

export function setSelectSpeak(enabled: boolean) {
  return invoke<AppSettings>('set_select_speak', { enabled })
}

export function setShowNative(enabled: boolean) {
  return invoke<AppSettings>('set_show_native', { enabled })
}

export function setAutoGrammar(enabled: boolean) {
  return invoke<AppSettings>('set_auto_grammar', { enabled })
}

export const beginShortcutCapture = () => invoke('begin_shortcut_capture')

export const endShortcutCapture = () => invoke('end_shortcut_capture')
