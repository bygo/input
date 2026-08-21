import { invoke } from '@tauri-apps/api/core'

export type AppSettings = {
  localModel: boolean
  targetLanguage: string
  nativeLanguage: string
  composeShortcut: string
  selectShortcut: string
}

export const loadSettings = () => invoke<AppSettings>('load_settings')

export const setTargetLanguage = (language: string) =>
  invoke<AppSettings>('set_target_language', { language })

export const setNativeLanguage = (language: string) =>
  invoke<AppSettings>('set_native_language', { language })

export const setShortcut = (kind: 'compose' | 'select', shortcut: string) =>
  invoke<AppSettings>('set_shortcut', { kind, shortcut })

export const beginShortcutCapture = () => invoke('begin_shortcut_capture')

export const endShortcutCapture = () => invoke('end_shortcut_capture')
