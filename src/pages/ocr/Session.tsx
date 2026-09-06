import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAtom } from 'jotai'
import { useEffect, useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import { inTauri } from '@/lib/dom'
import {
  dictAtom,
  errorAtom,
  ocrPreviewAtom,
  resultAtom,
  sendingAtom,
  textAtom,
  viewAtom,
} from '../../atoms'
import { blockWindowResize, fitOcrPin, ocrPinRef, sizeFloorRef } from '../shell/geometry'
import { OcrPick } from './Pick'

/** OCR 选区会话：冻屏选区 → 钉窗 → 识别预览。 */
export function OcrSession({ onPickMode }: { onPickMode: (pick: boolean) => void }) {
  const [, setView] = useAtom(viewAtom)
  const [, setOcrPreview] = useAtom(ocrPreviewAtom)
  const [, setSending] = useAtom(sendingAtom)
  const [, setText] = useAtom(textAtom)
  const [, setError] = useAtom(errorAtom)
  const [, setResult] = useAtom(resultAtom)
  const [, setDict] = useAtom(dictAtom)

  const [ocrPick, setOcrPick] = useState(false)
  const [ocrFreeze, setOcrFreeze] = useState('')
  const session = useRef(0)

  useEffect(() => {
    onPickMode(ocrPick)
  }, [ocrPick, onPickMode])

  useEffect(() => {
    if (!inTauri()) {
      return
    }
    let alive = true

    const unStart = listen<{ path?: string }>('ocr-pick-start', (e) => {
      if (!alive) {
        return
      }
      const path = e.payload?.path?.trim() ?? ''
      const url = path ? convertFileSrc(path) : ''
      const sid = ++session.current
      blockWindowResize.current = true
      void (async () => {
        if (url) {
          try {
            const im = new Image()
            im.src = url
            await im.decode()
          } catch {
            /* still show pick UI */
          }
        }
        if (!alive || sid !== session.current) {
          return
        }
        flushSync(() => {
          sizeFloorRef.current = null
          ocrPinRef.current = null
          setOcrPreview(null)
          setResult(null)
          setError('')
          setText('')
          setSending(false)
          setOcrFreeze(url)
          setView('ocr')
          setOcrPick(true)
        })
        if (!alive || sid !== session.current) {
          return
        }
        void invoke('ocr_show_pick').catch(() => {})
      })()
    })

    const unEnd = listen('ocr-pick-end', () => {
      if (!alive) {
        return
      }
      session.current += 1
      blockWindowResize.current = false
      flushSync(() => {
        setOcrPick(false)
        setOcrFreeze('')
        setOcrPreview(null)
        ocrPinRef.current = null
      })
    })

    const unBusy = listen<{ preview?: string }>('ocr-recognizing', (e) => {
      if (!alive) {
        return
      }
      const raw = e.payload?.preview?.trim() ?? ''
      if (!raw) {
        return
      }
      const src = raw.startsWith('data:') ? raw : convertFileSrc(raw)
      blockWindowResize.current = false
      flushSync(() => {
        setOcrPreview(prev => ({
          src,
          w: prev?.w && prev.w > 0 ? prev.w : 0,
          h: prev?.h && prev.h > 0 ? prev.h : 0,
        }))
        setOcrPick(false)
        setOcrFreeze('')
        setDict(null)
        setText('')
        setResult(null)
        setError('')
        // 等 ocr-text 再 loading，否则不知落在母语栏还是英语栏
        setSending(false)
        setView('compose')
      })
      void (async () => {
        try {
          const win = getCurrentWindow()
          await invoke('reveal')
          const p = ocrPinRef.current
          if (p) {
            const size = await win.outerSize()
            const scale = await win.scaleFactor()
            const fitted = await fitOcrPin(win, p.x, p.y, size.width / scale, size.height / scale)
            ocrPinRef.current = fitted
            await win.setPosition(new PhysicalPosition(fitted.x, fitted.y))
          }
          await win.setFocus()
        } catch {
          /* ignore */
        }
      })()
    })

    const unHidden = listen('overlay-hidden', () => {
      if (!alive) {
        return
      }
      session.current += 1
      blockWindowResize.current = false
      flushSync(() => {
        setOcrPick(false)
        setOcrFreeze('')
        setOcrPreview(null)
        ocrPinRef.current = null
      })
    })

    return () => {
      alive = false
      unStart.then(f => f())
      unEnd.then(f => f())
      unBusy.then(f => f())
      unHidden.then(f => f())
    }
  }, [setOcrPreview, setDict, setResult, setError, setText, setSending, setView])

  if (!ocrPick) {
    return null
  }

  return (
    <OcrPick
      freeze={ocrFreeze}
      onCancel={() => {
        session.current += 1
        blockWindowResize.current = false
        ocrPinRef.current = null
        setOcrPick(false)
        setOcrFreeze('')
        setOcrPreview(null)
      }}
      onFinishSelect={(crop) => {
        const sid = session.current
        const imgW = crop.w
        const imgH = crop.h
        const imgPad = 8
        const w = Math.max(360, imgW + imgPad * 2) + 24
        const h = imgH + imgPad * 2 + 80 + 24
        sizeFloorRef.current = { w, h }
        blockWindowResize.current = true
        void (async () => {
          try {
            const win = getCurrentWindow()
            const scale = await win.scaleFactor()
            const outer = await win.outerPosition()
            const pad = 12 + imgPad
            const px = Math.round(outer.x + (crop.x - pad) * scale)
            const py = Math.round(outer.y + (crop.y - pad) * scale)
            const fitted = await fitOcrPin(win, px, py, w, h)
            ocrPinRef.current = fitted
            await win.hide()
            await win.setSize(new LogicalSize(w, h))
            await win.setPosition(new PhysicalPosition(fitted.x, fitted.y))
          } catch {
            /* ignore */
          }
          if (sid !== session.current) {
            blockWindowResize.current = false
            return
          }
          flushSync(() => {
            setOcrPick(false)
            setOcrFreeze('')
            setOcrPreview(prev => ({
              w: imgW,
              h: imgH,
              src: prev?.src || '',
            }))
            setDict(null)
            setText('')
            setResult(null)
            setError('')
            setSending(false)
            setView('compose')
          })
          blockWindowResize.current = false
        })()
      }}
    />
  )
}
