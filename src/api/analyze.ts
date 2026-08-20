import { invoke } from '@tauri-apps/api/core'
import type { Analysis } from '../atoms'

export const analyze = (text: string) => invoke<Analysis>('analyze', { text })
