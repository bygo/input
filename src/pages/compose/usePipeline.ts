import type { DictEntry } from '../../api/dict'
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
  grammarAtom,
  grammarBusyAtom,
  inputElAtom,
  languageAtom,
  nativeAtom,
  resultAtom,
  selectSpeakAtom,
  sendingAtom,
  showNativeAtom,
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
    const showNative = get(showNativeAtom)
    const enSrc = looksEnglish(src)
    // 关中文译文时英源划词不上栏，避免 loadTop 闪空白行
    set(toNativeAtom, select && enSrc && showNative)
    set(errorAtom, '')
    set(resultAtom, null)
    set(dictAtom, null)
    set(nativeAtom, '')
    set(grammarAtom, '')
    set(grammarBusyAtom, false)

    // 输入仅为一个英文单词：只出词卡，不走改写/翻译
    if (isEnglishWord(src) && inTauri()) {
      let entry: DictEntry | null = null
      try {
        entry = await dictLookup(src)
      } catch {
        /* 无词条也只出卡 */
      }
      if (id !== req) {
        return
      }
      set(toNativeAtom, false)
      set(englishAtom, '')
      set(nativeAtom, '')
      set(grammarAtom, '')
      set(grammarBusyAtom, false)
      set(textAtom, src)
      set(dictAtom, entry ?? { word: src, senses: [] })
      set(sendingAtom, false)
      return
    }

    // 英源划词且关中文译文：只落英语栏，不上栏、不转圈
    if (select && enSrc && !showNative) {
      set(textAtom, '')
      set(englishAtom, src)
      set(resultAtom, { sentences: [src], source: 'passthrough' })
      set(sendingAtom, false)
      if (selectSpeak && inTauri()) {
        void dictSpeak(src, language).catch(() => {})
      }
      return
    }

    set(sendingAtom, true)

    try {
      // 输入改写：英语；中文对照按设置
      if (!select) {
        set(textAtom, src)
        set(englishAtom, '')
        set(nativeAtom, '')
        set(grammarAtom, '')
        set(grammarBusyAtom, false)
        const r = await analyze(src, language, false, id, showNative)
        if (id !== req) {
          return
        }
        set(resultAtom, r)
        const out = r.sentences[0]?.trim() || ''
        if (!out) {
          return
        }
        set(englishAtom, out)
        set(nativeAtom, showNative ? (r.native ?? '').trim() : '')
        if (selectSpeak && inTauri() && r.source !== 'passthrough') {
          void dictSpeak(out, language).catch(() => {})
        }
        return
      }

      // 划词 / OCR：英源→上栏中文；其它→下栏英语
      if (enSrc) {
        set(englishAtom, src)
        set(textAtom, '')
      } else {
        set(textAtom, src)
        set(englishAtom, '')
      }
      set(nativeAtom, '')
      set(grammarAtom, '')
      set(grammarBusyAtom, false)
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
      // 主输入栏 / 词卡追问等：回车留给字段自己，不回填
      const ae = document.activeElement
      if (
        ae === store.get(inputElAtom)
        || ae instanceof HTMLInputElement
        || ae instanceof HTMLTextAreaElement
      ) {
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
