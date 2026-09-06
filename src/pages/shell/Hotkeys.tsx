import { invoke } from '@tauri-apps/api/core'
import { useAtom } from 'jotai'
import { useEffect } from 'react'
import { inTauri } from '@/lib/dom'
import { viewAtom } from '../../atoms'
import { blockWindowResize } from './geometry'

/** Esc dismiss、⌘/Ctrl+, 切设置。 */
export function Hotkeys({ ocrPickMode }: { ocrPickMode: boolean }) {
  const [, setView] = useAtom(viewAtom)

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // 选区阶段 Esc 由 OcrPick 独占（ocr_cancel_pick），避免误 dismiss 成小窗。
      if (e.key === 'Escape' && inTauri()) {
        if (blockWindowResize.current || ocrPickMode) {
          return
        }
        invoke('dismiss')
        return
      }
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault()
        setView(v => (v === 'settings' ? 'compose' : 'settings'))
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [setView, ocrPickMode])

  return null
}
