import { useAtom } from 'jotai'
import { useEffect, useLayoutEffect, useRef } from 'react'
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
  selectShortcutAtom,
  sendingAtom,
  viewAtom,
} from './atoms'
import { cn } from '@/lib/utils'
import { Compose } from './Compose'
import { Settings } from './Settings'

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export default function App() {
  const [view, setView] = useAtom(viewAtom)
  const [sending] = useAtom(sendingAtom)
  const [, setLanguage] = useAtom(languageAtom)
  const [, setNative] = useAtom(nativeLanguageAtom)
  const [, setComposeShortcut] = useAtom(composeShortcutAtom)
  const [, setSelectShortcut] = useAtom(selectShortcutAtom)
  const cardRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!inTauri()) return
    loadSettings()
      .then((s) => {
        if (s.targetLanguage) setLanguage(s.targetLanguage)
        if (s.nativeLanguage) setNative(s.nativeLanguage)
        if (s.composeShortcut) setComposeShortcut(s.composeShortcut)
        if (s.selectShortcut) setSelectShortcut(s.selectShortcut)
      })
      .catch(() => {})
    const unShown = listen('overlay-shown', () => {
      flushSync(() => setView('compose'))
    })
    const unSettings = listen('open-settings', () => {
      flushSync(() => setView('settings'))
    })
    const unHidden = listen('overlay-hidden', () => {
      flushSync(() => setView('compose'))
    })
    return () => {
      unShown.then((f) => f())
      unSettings.then((f) => f())
      unHidden.then((f) => f())
    }
  }, [setView, setLanguage, setNative, setComposeShortcut, setSelectShortcut])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && inTauri()) {
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
  }, [setView])

  useLayoutEffect(() => {
    const el = cardRef.current
    if (!el || !inTauri()) return
    let gen = 0
    let dead = false
    const apply = () => {
      if (dead) return
      const id = ++gen
      const w = Math.ceil(el.scrollWidth) || 440
      const h = Math.ceil(Math.max(el.scrollHeight, el.offsetHeight))
      if (h < 8) return
      const win = getCurrentWindow()
      Promise.all([win.innerSize(), win.scaleFactor()]).then(([s, sf]) => {
        if (dead || id !== gen) return
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
  }, [view])

  return (
    <div
      ref={cardRef}
      className={cn(
        'w-[440px] rounded-lg bg-card text-card-foreground shadow-xl ring-1 ring-foreground/10',
        sending && view === 'compose' && 'animate-breathe-bg',
      )}
    >
      <div hidden={view !== 'compose'}>
        <Compose />
      </div>
      <div hidden={view !== 'settings'}>
        <Settings />
      </div>
    </div>
  )
}
