import { useAtomValue, useSetAtom, useStore } from 'jotai'
import { useLayoutEffect } from 'react'
import { autosize } from '@/lib/dom'
import {
  botElAtom,
  dictAtom,
  englishAtom,
  inputElAtom,
  ocrPreviewAtom,
  paneLockAtom,
  sendingAtom,
  textAtom,
} from '../../atoms'

/** textarea autosize + 词卡弹出时钉死栏高 → paneLockAtom。 */
export function usePaneLayout() {
  const store = useStore()
  const dict = useAtomValue(dictAtom)
  const text = useAtomValue(textAtom)
  const english = useAtomValue(englishAtom)
  const sending = useAtomValue(sendingAtom)
  const ocrPreview = useAtomValue(ocrPreviewAtom)
  const setPaneLock = useSetAtom(paneLockAtom)

  useLayoutEffect(() => {
    autosize(store.get(inputElAtom))
  }, [store, text, sending, english, ocrPreview])

  useLayoutEffect(() => {
    if (!dict) {
      setPaneLock(null)
      return
    }
    if (store.get(paneLockAtom)) {
      return
    }
    const topEl = store.get(inputElAtom)
    const botEl = store.get(botElAtom)
    if (!topEl) {
      return
    }
    const lock = { top: topEl.offsetHeight, bot: botEl?.offsetHeight ?? 0 }
    if (lock.top < 1) {
      return
    }
    setPaneLock(lock)
  }, [dict, text, english, sending, store, setPaneLock])
}
