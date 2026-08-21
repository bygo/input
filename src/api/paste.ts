import { invoke } from '@tauri-apps/api/core'

export const pasteBack = (text: string) => invoke('paste_back', { text })
