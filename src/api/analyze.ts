import type { Analysis } from '../atoms'
import { invoke } from '@tauri-apps/api/core'

export function analyze(text: string, into?: string, select?: boolean, reqId?: number) {
  return invoke<Analysis>('analyze', { text, into, select, reqId })
}
