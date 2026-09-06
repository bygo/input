import { atom, useSetAtom, useStore } from 'jotai'
import { useEffect } from 'react'
import { copyText, inTauri } from '@/lib/dom'
import { isEnglishWord, looksEnglish } from '@/lib/text'
import { analyze } from '../../api/analyze'
import { dictLookup, dictSpeak } from '../../api/dict'
import { pasteBack } from '../../api/paste'
import {
  dictAtom,
  englishAtom,
  errorAtom,
  inputElAtom,
  languageAtom,
  nativeAtom,
  resultAtom,
  selectSpeakAtom,
  sendingAtom,
  textAtom,
  toNativeAtom,
  viewAtom,
} from '../../atoms'

let req = 0

/** 改写 / 翻译入口（业务组件 useSetAtom 调用，禁止 props 下发）。 */
export const composeRunAtom = atom(
  null,
  async (get, set, payload?: { src?: string, select?: boolean }) => {
    const select = payload?.select ?? false
    const src = (payload?.src ?? get(textAtom)).trim()
    if (!src) {
      return
    }
    if (get(sendingAtom) && !select) {
      return
    }
    const id = ++req
    const language = get(languageAtom)
    const selectSpeak = get(selectSpeakAtom)
    const enSrc = looksEnglish(src)
    set(toNativeAtom, select && enSrc)
    set(sendingAtom, true)
    set(errorAtom, '')
    set(resultAtom, null)
    set(dictAtom, null)
    set(nativeAtom, '')

    if (isEnglishWord(src) && inTauri()) {
      try {
        const e = await dictLookup(src)
        if (id !== req) {
          return
        }
        if (e) {
          set(toNativeAtom, false)
          set(englishAtom, '')
          set(nativeAtom, '')
          set(textAtom, src)
          set(dictAtom, e)
          set(sendingAtom, false)
          return
        }
      } catch {
        /* fall through */
      }
    }

    try {
      // 输入改写：一次出英语 + 中文；上栏保留原文，翻译面板落英/中
      if (!select) {
        set(textAtom, src)
        set(englishAtom, '')
        set(nativeAtom, '')
        const r = await analyze(src, language, false, id)
        if (id !== req) {
          return
        }
        set(resultAtom, r)
        const out = r.sentences[0]?.trim() || ''
        if (!out) {
          return
        }
        set(englishAtom, out)
        set(nativeAtom, (r.native ?? '').trim())
        if (selectSpeak && inTauri() && r.source !== 'passthrough') {
          void dictSpeak(out, language).catch(() => {})
        }
        return
      }

      // 划词 / OCR：翻译落栏（英源→上栏中文，其它→下栏英语）
      if (enSrc) {
        set(englishAtom, src)
        set(textAtom, '')
      } else {
        set(textAtom, src)
        set(englishAtom, '')
      }
      set(nativeAtom, '')
      const r = await analyze(src, undefined, true, id)
      if (id !== req) {
        return
      }
      set(resultAtom, r)
      const out = r.sentences[0]?.trim() || src
      if (!out) {
        return
      }
      if (get(toNativeAtom)) {
        set(textAtom, out)
      } else {
        set(englishAtom, out)
      }
      if (selectSpeak && inTauri() && r.source !== 'passthrough') {
        const spoken = (get(toNativeAtom) ? src : out).trim()
        if (spoken) {
          void dictSpeak(spoken, language).catch(() => {})
        }
      }
    } catch (e) {
      if (id !== req) {
        return
      }
      const msg = String(e)
      if (msg.includes('未就绪')) {
        set(resultAtom, null)
        set(errorAtom, msg)
      } else {
        set(errorAtom, '')
        set(resultAtom, { sentences: [src], source: 'passthrough' })
        if (get(toNativeAtom)) {
          set(textAtom, src)
        } else {
          set(englishAtom, src)
        }
      }
    } finally {
      if (id === req) {
        set(sendingAtom, false)
      }
    }
  },
)

export function bumpComposeReq() {
  req += 1
}

export function composeReqId() {
  return req
}

/** 全局快捷键回填、首焦。 */
export function usePipeline() {
  const store = useStore()
  const setError = useSetAtom(errorAtom)

  useEffect(() => {
    if (store.get(englishAtom).trim() || (store.get(sendingAtom) && store.get(toNativeAtom))) {
      return
    }
    const el = store.get(inputElAtom)
    el?.focus()
    el?.select()
  }, [store])

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (store.get(viewAtom) !== 'compose') {
        return
      }
      if (e.isComposing || e.keyCode === 229) {
        return
      }
      const en = store.get(englishAtom).trim()
      if (!en) {
        return
      }
      if (e.key !== 'Enter') {
        return
      }
      if (document.activeElement === store.get(inputElAtom)) {
        return
      }
      e.preventDefault()
      e.stopPropagation()
      void (async () => {
        if (inTauri()) {
          try {
            await pasteBack(en)
          } catch (err) {
            setError(String(err))
          }
        } else {
          await copyText(en)
        }
      })()
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [store, setError])
}
