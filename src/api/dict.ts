import { invoke } from '@tauri-apps/api/core'

export interface DictSense {
  pos: string
  gloss: string
}

export interface DictEntry {
  word: string
  phonetic?: string | null
  senses: DictSense[]
}

export interface DictPackRow {
  id: string
  label: string
  detail: string
  installed: boolean
}

export interface DictStatus {
  pack: string
  packs: DictPackRow[]
  busy: boolean
  message: string
  percent: number
}

export function dictLookup(word: string) {
  return invoke<DictEntry | null>('dict_lookup', { word })
}

export function dictSpeak(word: string, lang?: string) {
  return invoke<void>('dict_speak', { word, lang })
}

export const dictStop = () => invoke<void>('dict_stop')

export const dictSpeaking = () => invoke<boolean>('dict_speaking')

export const dictStatus = () => invoke<DictStatus>('dict_status')

export const pullDictPack = (id: string) => invoke<DictStatus>('pull_dict_pack', { id })

export const setDictPack = (id: string) => invoke<DictStatus>('set_dict_pack', { id })

export const deleteDictPack = (id: string) => invoke<DictStatus>('delete_dict_pack', { id })
