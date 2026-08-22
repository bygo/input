import { invoke } from '@tauri-apps/api/core'

export type DictSense = { pos: string; gloss: string }
export type DictEntry = {
  word: string
  phonetic?: string | null
  senses: DictSense[]
}

export const dictLookup = (word: string) =>
  invoke<DictEntry | null>('dict_lookup', { word })

export const dictSpeak = (word: string, lang?: string) =>
  invoke<void>('dict_speak', { word, lang })
