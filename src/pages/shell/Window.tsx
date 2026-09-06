import type { RefObject } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAtom } from 'jotai'
import { useLayoutEffect } from 'react'
import { inTauri } from '@/lib/dom'
import { sendingAtom, textAtom } from '../../atoms'
import { blockWindowResize, fitOcrPin, ocrPinRef, sizeFloorRef } from './geometry'

/** 按 card 内容同步 Tauri 窗口尺寸；OCR 钉点时禁止跟鼠标 reposition。 */
export function Window({
  cardRef,
  ocrPickMode,
  sizeEpoch,
  view,
  ocrPreviewSrc,
}: {
  cardRef: RefObject<HTMLDivElement | null>
  ocrPickMode: boolean
  sizeEpoch: number
  view: string
  ocrPreviewSrc?: string
}) {
  const [sending] = useAtom(sendingAtom)
  const [text] = useAtom(textAtom)

  useLayoutEffect(() => {
    if (ocrPickMode) {
      return
    }
    const el = cardRef.current
    if (!el) {
      return
    }
    if (!inTauri()) {
      return
    }

    let gen = 0
    let dead = false

    const blocked = () => dead || blockWindowResize.current || ocrPickMode

    const apply = () => {
      if (blocked()) {
        return
      }

      const id = ++gen
      const floor = sizeFloorRef.current
      const w = Math.max(Math.ceil(el.scrollWidth) || 0, floor?.w ?? 0) || 360
      const h = Math.max(Math.ceil(Math.max(el.scrollHeight, el.offsetHeight)) || 0, floor?.h ?? 0)
      if (h < 8) {
        return
      }

      const win = getCurrentWindow()
      void Promise.all([win.innerSize(), win.scaleFactor()]).then(([s, sf]) => {
        if (blocked()) {
          return
        }
        if (id !== gen) {
          return
        }
        if (Math.round(s.width / sf) === w && Math.round(s.height / sf) === h) {
          return
        }

        return win.setSize(new LogicalSize(w, h)).then(async () => {
          const pin = ocrPinRef.current
          if (pin) {
            const fitted = await fitOcrPin(win, pin.x, pin.y, w, h)
            ocrPinRef.current = fitted
            await win.setPosition(new PhysicalPosition(fitted.x, fitted.y))
            return
          }
          if (ocrPreviewSrc) {
            return
          }
          return invoke('reposition')
        })
      })
    }

    apply()
    let raf2 = 0
    const raf1 = requestAnimationFrame(() => {
      apply()
      // 译文/栏高常在下一帧才稳定，再测一次
      raf2 = requestAnimationFrame(apply)
    })
    const ro = new ResizeObserver(apply)
    ro.observe(el)
    return () => {
      dead = true
      cancelAnimationFrame(raf1)
      cancelAnimationFrame(raf2)
      ro.disconnect()
    }
  }, [cardRef, view, ocrPickMode, sizeEpoch, ocrPreviewSrc, sending, text])

  return null
}
