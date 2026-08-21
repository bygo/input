import { atom } from 'jotai'

export type Analysis = {
  sentences: string[]
  source: 'local' | 'cloud' | string
}

export const textAtom = atom('')
export const sendingAtom = atom(false)
export const resultAtom = atom<Analysis | null>(null)
export const errorAtom = atom('')
export const viewAtom = atom<'compose' | 'settings'>('compose')
export const languageAtom = atom('英语')
export const nativeLanguageAtom = atom('中文')
export const composeShortcutAtom = atom('Alt+A')
export const selectShortcutAtom = atom('Alt+D')
