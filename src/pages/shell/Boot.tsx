import type { Dispatch, SetStateAction } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useAtom } from 'jotai'
import { useEffect } from 'react'
import { flushSync } from 'react-dom'
import { inTauri } from '@/lib/dom'
import { loadSettings } from '../../api/settings'
import {
  autoGrammarAtom,
  composeShortcutAtom,
  languageAtom,
  nativeLanguageAtom,
  ocrShortcutAtom,
  selectShortcutAtom,
  selectSpeakAtom,
  showNativeAtom,
  viewAtom,
} from '../../atoms'
import { blockWindowResize, sizeFloorRef } from './geometry'

/** 启动读设置 + overlay 显隐 / 打开设置 → 切 view。 */
export function Boot({
  setSizeEpoch,
}: {
  setSizeEpoch: Dispatch<SetStateAction<number>>
}) {
  const [, setView] = useAtom(viewAtom)
  const [, setLanguage] = useAtom(languageAtom)
  const [, setNative] = useAtom(nativeLanguageAtom)
  const [, setComposeShortcut] = useAtom(composeShortcutAtom)
  const [, setSelectShortcut] = useAtom(selectShortcutAtom)
  const [, setOcrShortcut] = useAtom(ocrShortcutAtom)
  const [, setSelectSpeak] = useAtom(selectSpeakAtom)
  const [, setShowNative] = useAtom(showNativeAtom)
  const [, setAutoGrammar] = useAtom(autoGrammarAtom)

  useEffect(() => {
    if (!inTauri()) {
      return
    }
    loadSettings()
      .then((s) => {
        setLanguage('英语')
        setNative('中文')
        if (s.composeShortcut) {
          setComposeShortcut(s.composeShortcut)
        }
        if (s.selectShortcut) {
          setSelectShortcut(s.selectShortcut)
        }
        if (s.ocrShortcut) {
          setOcrShortcut(s.ocrShortcut)
        }
        if (typeof s.selectSpeak === 'boolean') {
          setSelectSpeak(s.selectSpeak)
        }
        if (typeof s.showNative === 'boolean') {
          setShowNative(s.showNative)
        }
        if (typeof s.autoGrammar === 'boolean') {
          setAutoGrammar(s.autoGrammar)
        }
      })
      .catch(() => {})

    const unShown = listen('overlay-shown', () => {
      // OCR 文本 / 选中译文由 ocr/Session、Compose 处理；此处解除跟窗封锁并催尺寸。
      // 选中翻译也必须 bump：Rust 先把窗设成 OVERLAY_H，译文出现后要重测。
      blockWindowResize.current = false
      sizeFloorRef.current = null
      flushSync(() => {
        setView('compose')
        setSizeEpoch(n => n + 1)
      })
    })
    const unSettings = listen('open-settings', () => {
      flushSync(() => setView('settings'))
    })
    const unHidden = listen('overlay-hidden', () => {
      blockWindowResize.current = false
      sizeFloorRef.current = null
      flushSync(() => setView('compose'))
    })

    return () => {
      unShown.then(f => f())
      unSettings.then(f => f())
      unHidden.then(f => f())
    }
  }, [setView, setLanguage, setNative, setComposeShortcut, setSelectShortcut, setOcrShortcut, setSelectSpeak, setShowNative, setAutoGrammar, setSizeEpoch])

  return null
}
