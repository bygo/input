import type { DictEntry } from './api/dict'
import { atom } from 'jotai'
import { looksEnglish, wordDiff } from '@/lib/text'

export interface Analysis {
  sentences: string[]
  /** 改写一次附带的中文；翻译路径可空 */
  native?: string | null
  source: 'local' | 'passthrough' | string
}

export type { DictEntry }

export const textAtom = atom('')
/** 翻译面板：英语 */
export const englishAtom = atom('')
/** 翻译面板：中文（改写一次附带；或划词译中时的结果另走 text） */
export const nativeAtom = atom('')
/** 点词触发：各语法块的抽象语义 */
export const grammarAtom = atom('')
export const grammarBusyAtom = atom(false)
export const sendingAtom = atom(false)
/** 划词/OCR：英文源 → 译文进上栏 */
export const toNativeAtom = atom(false)
export const resultAtom = atom<Analysis | null>(null)
export const dictAtom = atom<DictEntry | null>(null)
export const errorAtom = atom('')
/** OCR crop preview: data-URL + natural pixel size for 1:1 layout without reflow */
export interface OcrPreview { src: string, w: number, h: number }
export const ocrPreviewAtom = atom<OcrPreview | null>(null)
export const viewAtom = atom<'compose' | 'settings' | 'ocr'>('compose')
export const languageAtom = atom('英语')
export const nativeLanguageAtom = atom('中文')
export const composeShortcutAtom = atom('CommandOrControl+G')
export const selectShortcutAtom = atom('CommandOrControl+2')
export const ocrShortcutAtom = atom('CommandOrControl+Shift+O')
/** 翻译完成后是否朗读英语（输入 / 选中 / OCR） */
export const selectSpeakAtom = atom(true)
/** 是否显示中文对照 */
export const showNativeAtom = atom(true)
/** 译出后是否自动解析；关则点词才解析 */
export const autoGrammarAtom = atom(false)

export const loadTopAtom = atom(get => get(sendingAtom) && get(toNativeAtom))
export const loadBotAtom = atom(get => get(sendingAtom) && !get(toNativeAtom))

/** 英文→英文改写词级 diff；跨语种为 null */
export const diffPartsAtom = atom((get) => {
  const src = get(textAtom).trim()
  const out = get(englishAtom).trim()
  if (!src || !out || !looksEnglish(src) || !looksEnglish(out)) {
    return null
  }
  return wordDiff(src, out)
})

/** 词卡弹出时钉死的栏高 */
export const paneLockAtom = atom<{ top: number, bot: number } | null>(null)

export const inputElAtom = atom<HTMLTextAreaElement | null>(null)
export const botElAtom = atom<HTMLDivElement | null>(null)
