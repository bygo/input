import { invoke } from '@tauri-apps/api/core'
import type { Analysis } from '../atoms'

export const analyze = (text: string, into?: string, select?: boolean) =>
  invoke<Analysis>('analyze', { text, into, select })
