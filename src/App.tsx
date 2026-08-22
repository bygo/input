import { useAtom } from 'jotai'
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import { invoke } from '@tauri-apps/api/core'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { loadSettings } from './api/settings'
import {
  composeShortcutAtom,
  languageAtom,
  nativeLanguageAtom,
  ocrShortcutAtom,
  selectShortcutAtom,
  viewAtom,
} from './atoms'
import { cn } from '@/lib/utils'
import { Compose } from './Compose'
import { OcrShell } from './OcrShell'
import { Settings } from './Settings'

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export default function App() {
  const [view, setView] = useAtom(viewAtom)
  const [, setLanguage] = useAtom(languageAtom)
  const [, setNative] = useAtom(nativeLanguageAtom)
  const [, setComposeShortcut] = useAtom(composeShortcutAtom)
  const [, setSelectShortcut] = useAtom(selectShortcutAtom)
  const [, setOcrShortcut] = useAtom(ocrShortcutAtom)

  const cardRef = useRef<HTMLDivElement>(null)
  const sizeFloorRef = useRef<{ w: number; h: number } | null>(null)
  const blockWindowResize = useRef(false)
  const [ocrPickMode, setOcrPickMode] = useState(false)
  const [sizeEpoch, setSizeEpoch] = useState(0)

  const onBlockResize = useCallback((block: boolean) => {
    blockWindowResize.current = block
  }, [])

  useEffect(() => {
    if (!inTauri()) return
    loadSettings()
      .then((s) => {
        setLanguage('英语')
        if (s.nativeLanguage) setNative(s.nativeLanguage)
        if (s.composeShortcut) setComposeShortcut(s.composeShortcut)
        if (s.selectShortcut) setSelectShortcut(s.selectShortcut)
        if (s.ocrShortcut) setOcrShortcut(s.ocrShortcut)
      })
      .catch(() => {})

    const unShown = listen<{ selected?: string }>('overlay-shown', (e) => {
      // OCR 识别文本由 OcrShell 处理；此处只处理 compose/select 弹窗。
      if (e.payload?.selected?.trim()) return
      blockWindowResize.current = false
      sizeFloorRef.current = null
      flushSync(() => {
        setView('compose')
        setSizeEpoch((n) => n + 1)
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
      unShown.then((f) => f())
      unSettings.then((f) => f())
      unHidden.then((f) => f())
    }
  }, [setView, setLanguage, setNative, setComposeShortcut, setSelectShortcut, setOcrShortcut])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // 选区阶段 Esc 由 OcrPick 独占（ocr_cancel_pick），避免误 dismiss 成小窗。
      if (e.key === 'Escape' && inTauri()) {
        if (blockWindowResize.current || ocrPickMode) return
        invoke('dismiss')
        return
      }
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault()
        setView((v) => (v === 'settings' ? 'compose' : 'settings'))
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [setView, ocrPickMode])

  useLayoutEffect(() => {
    if (ocrPickMode || blockWindowResize.current) return
    const el = cardRef.current
    if (!el || !inTauri()) return
    let gen = 0
    let dead = false
    const apply = () => {
      if (dead || blockWindowResize.current || ocrPickMode) return
      const id = ++gen
      const floor = sizeFloorRef.current
      const w = Math.max(Math.ceil(el.scrollWidth) || 0, floor?.w ?? 0) || 440
      const h = Math.max(Math.ceil(Math.max(el.scrollHeight, el.offsetHeight)) || 0, floor?.h ?? 0)
      if (h < 8) return
      const win = getCurrentWindow()
      Promise.all([win.innerSize(), win.scaleFactor()]).then(([s, sf]) => {
        if (dead || id !== gen || blockWindowResize.current || ocrPickMode) return
        if (Math.round(s.width / sf) === w && Math.round(s.height / sf) === h) return
        return win.setSize(new LogicalSize(w, h)).then(() => invoke('reposition'))
      })
    }
    apply()
    const raf = requestAnimationFrame(() => requestAnimationFrame(apply))
    const ro = new ResizeObserver(apply)
    ro.observe(el)
    return () => {
      dead = true
      cancelAnimationFrame(raf)
      ro.disconnect()
    }
  }, [view, ocrPickMode, sizeEpoch])

  return (
    <div
      ref={cardRef}
      className={ocrPickMode ? 'h-screen w-screen bg-transparent' : 'w-max p-3'}
    >
      <OcrShell
        onBlockResize={onBlockResize}
        onPickMode={setOcrPickMode}
        sizeFloorRef={sizeFloorRef}
      />
      {view !== 'ocr' && !ocrPickMode && (
        <div
          className={cn(
            'min-w-[440px] rounded-lg bg-card text-card-foreground ring-1 ring-foreground/10',
            'shadow-[0_1px_2px_rgb(0_0_0/0.16)] dark:shadow-[0_1px_2px_rgb(0_0_0/0.55)]',
          )}
        >
          {view === 'settings' ? <Settings /> : <Compose />}
        </div>
      )}
    </div>
  )
}
