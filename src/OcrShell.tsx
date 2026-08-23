import { useAtom } from 'jotai'
import { useEffect, useRef, useState, type MutableRefObject } from 'react'
import { flushSync } from 'react-dom'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  dictAtom,
  errorAtom,
  ocrPreviewAtom,
  resultAtom,
  sendingAtom,
  textAtom,
  viewAtom,
} from './atoms'
import { OcrPick } from './OcrPick'

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** OCR 会话沙盒：始终挂载以接收事件；选区 → 识别 → Compose 左图右文。 */
export function OcrShell({
  onBlockResize,
  onPickMode,
  sizeFloorRef,
  ocrPinRef,
}: {
  onBlockResize: (block: boolean) => void
  onPickMode: (pick: boolean) => void
  sizeFloorRef: MutableRefObject<{ w: number; h: number } | null>
  ocrPinRef: MutableRefObject<{ x: number; y: number } | null>
}) {
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
  const flowActive = useRef(false)

  useEffect(() => {
    onPickMode(ocrPick)
  }, [ocrPick, onPickMode])

  useEffect(() => {
    if (!inTauri()) return
    let alive = true

    const unStart = listen<{ path?: string }>('ocr-pick-start', (e) => {
      if (!alive) return
      const path = e.payload?.path?.trim() ?? ''
      const url = path ? convertFileSrc(path) : ''
      const sid = ++session.current
      onBlockResize(true)
      flowActive.current = false
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
        if (!alive || sid !== session.current) return
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
        if (!alive || sid !== session.current) return
        void invoke('ocr_show_pick').catch(() => {})
      })()
    })

    const unEnd = listen('ocr-pick-end', () => {
      if (!alive) return
      session.current += 1
      flowActive.current = false
      onBlockResize(false)
      flushSync(() => {
        setOcrPick(false)
        setOcrFreeze('')
        setOcrPreview(null)
        ocrPinRef.current = null
      })
    })

    const unBusy = listen<{ preview?: string }>('ocr-recognizing', (e) => {
      if (!alive) return
      const raw = e.payload?.preview?.trim() ?? ''
      if (!raw) return
      const src = raw.startsWith('data:') ? raw : convertFileSrc(raw)
      onBlockResize(false)
      flowActive.current = true
      flushSync(() => {
        setOcrPreview((prev) => ({
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
          // reveal 会跟鼠标；钉回选区原位
          const p = ocrPinRef.current
          if (p) await win.setPosition(new PhysicalPosition(p.x, p.y))
          // 确保焦点在卡片上，否则点外侧不触发失焦关闭
          await win.setFocus()
        } catch {
          /* ignore */
        }
      })()
    })

    const unHidden = listen('overlay-hidden', () => {
      if (!alive) return
      session.current += 1
      flowActive.current = false
      onBlockResize(false)
      flushSync(() => {
        setOcrPick(false)
        setOcrFreeze('')
        setOcrPreview(null)
        ocrPinRef.current = null
      })
    })

    return () => {
      alive = false
      unStart.then((f) => f())
      unEnd.then((f) => f())
      unBusy.then((f) => f())
      unHidden.then((f) => f())
    }
  }, [
    onBlockResize,
    sizeFloorRef,
    ocrPinRef,
    setOcrPreview,
    setDict,
    setResult,
    setError,
    setText,
    setSending,
    setView,
  ])

  if (ocrPick) {
    return (
      <OcrPick
        freeze={ocrFreeze}
        onCancel={() => {
          session.current += 1
          flowActive.current = false
          onBlockResize(false)
          ocrPinRef.current = null
          setOcrPick(false)
          setOcrFreeze('')
          setOcrPreview(null)
        }}
        onFinishSelect={(crop) => {
          const sid = session.current
          // 1:1 CSS 尺寸才能与选区完全重叠（不缩小）
          const imgW = crop.w
          const imgH = crop.h
          const imgPad = 8 // 截图四周底衬，与 Compose 一致
          // 左图右文：宽 = 图 + 底衬 + 360 文栏；外层 p-3×2 = 24
          const w = imgW + imgPad * 2 + 360 + 24
          const h = Math.max(imgH + imgPad * 2, 80) + 24
          sizeFloorRef.current = { w, h }
          onBlockResize(true)
          flowActive.current = true
          void (async () => {
            try {
              const win = getCurrentWindow()
              const scale = await win.scaleFactor()
              const outer = await win.outerPosition()
              const pad = 12 + imgPad // App p-3 + 截图底衬
              const px = Math.round(outer.x + (crop.x - pad) * scale)
              const py = Math.round(outer.y + (crop.y - pad) * scale)
              ocrPinRef.current = { x: px, y: py }
              await win.hide()
              await win.setSize(new LogicalSize(w, h))
              await win.setPosition(new PhysicalPosition(px, py))
            } catch {
              /* ignore */
            }
            if (sid !== session.current) {
              onBlockResize(false)
              flowActive.current = false
              return
            }
            flushSync(() => {
              setOcrPick(false)
              setOcrFreeze('')
              setOcrPreview((prev) => ({
                w: imgW,
                h: imgH,
                src: prev?.src || '',
              }))
              setDict(null)
              setText('')
              setResult(null)
              setError('')
              // 识别完成（ocr-text）后再 loading
              setSending(false)
              setView('compose')
            })
            onBlockResize(false)
          })()
        }}
      />
    )
  }

  return null
}
