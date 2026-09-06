import { useSetAtom, useStore } from 'jotai'
import { listen } from '@tauri-apps/api/event'
import { useEffect } from 'react'
import { inTauri } from '@/lib/dom'
import { looksEnglish } from '@/lib/text'
import { dictStop } from '../../api/dict'
import {
  dictAtom,
  englishAtom,
  errorAtom,
  inputElAtom,
  nativeAtom,
  ocrPreviewAtom,
  resultAtom,
  sendingAtom,
  textAtom,
  toNativeAtom,
  type Analysis,
} from '../../atoms'
import { bumpComposeReq, composeReqId, composeRunAtom } from './usePipeline'

/** overlay / OCR / 流式 partial → atom。 */
export function useBridge() {
  const store = useStore()
  const run = useSetAtom(composeRunAtom)

  useEffect(() => {
    if (!inTauri()) {
      return
    }
    let alive = true
    let offHidden: (() => void) | undefined
    let offShown: (() => void) | undefined
    let offPartial: (() => void) | undefined
    let offOcrText: (() => void) | undefined

    listen('overlay-hidden', () => {
      if (!alive) {
        return
      }
      bumpComposeReq()
      store.set(toNativeAtom, false)
      store.set(textAtom, '')
      store.set(englishAtom, '')
      store.set(nativeAtom, '')
      store.set(resultAtom, null)
      store.set(dictAtom, null)
      store.set(errorAtom, '')
      store.set(sendingAtom, false)
      store.set(ocrPreviewAtom, null)
      void dictStop().catch(() => {})
    }).then((f) => {
      if (!alive) {
        f()
      } else {
        offHidden = f
      }
    })

    listen<{ selected?: string }>('overlay-shown', (e) => {
      if (!alive) {
        return
      }
      store.set(resultAtom, null)
      store.set(dictAtom, null)
      store.set(errorAtom, '')
      store.set(ocrPreviewAtom, null)
      store.set(englishAtom, '')
      store.set(nativeAtom, '')
      const selected = e.payload?.selected?.trim() ?? ''
      if (selected) {
        void run({ src: selected, select: true })
        return
      }
      store.set(textAtom, '')
      requestAnimationFrame(() => {
        const el = store.get(inputElAtom)
        el?.focus()
        el?.select()
      })
    }).then((f) => {
      if (!alive) {
        f()
      } else {
        offShown = f
      }
    })

    listen<string>('ocr-text', (e) => {
      if (!alive) {
        return
      }
      const selected = (typeof e.payload === 'string' ? e.payload : '').trim()
      if (!selected) {
        return
      }
      store.set(resultAtom, null)
      store.set(dictAtom, null)
      store.set(errorAtom, '')
      void run({ src: selected, select: true })
    }).then((f) => {
      if (!alive) {
        f()
      } else {
        offOcrText = f
      }
    })

    listen<{ reqId?: number, partial?: string, source?: string }>('analyze-partial', (e) => {
      if (!alive) {
        return
      }
      const id = e.payload?.reqId
      if (id != null && id !== composeReqId()) {
        return
      }
      const partial = e.payload?.partial ?? ''
      if (!partial) {
        return
      }
      store.set(resultAtom, {
        sentences: [partial],
        source: (e.payload?.source as Analysis['source']) || 'local',
      })
      if (store.get(toNativeAtom)) {
        if (looksEnglish(partial)) {
          store.set(englishAtom, partial)
        } else {
          store.set(textAtom, partial)
        }
      } else {
        store.set(englishAtom, partial)
      }
    }).then((f) => {
      if (!alive) {
        f()
      } else {
        offPartial = f
      }
    })

    return () => {
      alive = false
      offHidden?.()
      offShown?.()
      offPartial?.()
      offOcrText?.()
    }
  }, [store, run])
}
