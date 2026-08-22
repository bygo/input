import { atom } from 'jotai'
import type { DictEntry } from './api/dict'

export type Analysis = {
  sentences: string[]
  source: 'local' | 'local-mt' | 'cloud' | string
}

export type { DictEntry }

export const textAtom = atom('')
export const sendingAtom = atom(false)
export const resultAtom = atom<Analysis | null>(null)
export const dictAtom = atom<DictEntry | null>(null)
export const errorAtom = atom('')
/** OCR crop preview: data-URL + natural pixel size for 1:1 layout without reflow */
export type OcrPreview = { src: string; w: number; h: number }
export const ocrPreviewAtom = atom<OcrPreview | null>(null)
export const viewAtom = atom<'compose' | 'settings' | 'ocr'>('compose')
export const languageAtom = atom('英语')
export const nativeLanguageAtom = atom('中文')
export const composeShortcutAtom = atom('CommandOrControl+G')
export const selectShortcutAtom = atom('CommandOrControl+Shift+G')
export const ocrShortcutAtom = atom('CommandOrControl+Shift+O')