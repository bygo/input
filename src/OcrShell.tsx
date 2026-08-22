import { useAtom } from 'jotai'
import { useEffect, useRef, useState, type MutableRefObject } from 'react'
import { flushSync } from 'react-dom'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  errorAtom,
  ocrPreviewAtom,
  resultAtom,
  sendingAtom,
  textAtom,
  viewAtom,
} from './atoms'
import { cn } from '@/lib/utils'
import { OcrPick } from './OcrPick'
import { OcrTranslate } from './OcrTranslate'

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** OCR 会话沙盒：始终挂载以接收事件；选区 → 识别 → 结果。 */
export function OcrShell({
  onBlockResize,
  onPickMode,
  sizeFloorRef,
}: {
  onBlockResize: (block: boolean) => void
  onPickMode: (pick: boolean) => void
  sizeFloorRef: MutableRefObject<{ w: number; h: number } | null>
}) {
  const [view, setView] = useAtom(viewAtom)
  const [ocrPreview, setOcrPreview] = useAtom(ocrPreviewAtom)
  const [, setSending] = useAtom(sendingAtom)
  const [, setText] = useAtom(textAtom)
  const [, setError] = useAtom(errorAtom)
  const [, setResult] = useAtom(resultAtom)

  const [ocrPick, setOcrPick] = useState(false)
  const [ocrFreeze, setOcrFreeze] = useState('')
  const [ocrSeedText, setOcrSeedText] = useState('')

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
          setOcrPreview(null)
          setOcrSeedText('')
          setResult(null)
          setError('')
          setText('')
          setSending(false)
          setOcrFreeze(url)
          setView('ocr')
          setOcrPick(true)
        })
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            if (!alive || sid !== session.current) return
            void invoke('ocr_show_pick').catch(() => {})
          })
        })
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
        setOcrSeedText('')
      })
    })

    const unBusy = listen<{ preview?: string }>('ocr-recognizing', (e) => {
      if (!alive) return
      const src = e.payload?.preview?.trim() ?? ''
      if (!src) return
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
        setSending(true)
        setView('ocr')
      })
      void invoke('reveal').catch(() => {})
    })

    const unShown = listen<{ selected?: string }>('overlay-shown', (e) => {
      if (!alive) return
      const selected = e.payload?.selected?.trim() ?? ''
      if (!flowActive.current || !selected) return
      flushSync(() => setOcrSeedText(selected))
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
        setOcrSeedText('')
      })
    })

    return () => {
      alive = false
      unStart.then((f) => f())
      unEnd.then((f) => f())
      unBusy.then((f) => f())
      unShown.then((f) => f())
      unHidden.then((f) => f())
    }
  }, [
    onBlockResize,
    sizeFloorRef,
    setOcrPreview,
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
          setOcrPick(false)
          setOcrFreeze('')
          setOcrPreview(null)
          setOcrSeedText('')
        }}
        onFinishSelect={(crop) => {
          const sid = session.current
          const maxImgH = Math.max(
            1,
            Math.floor((window.screen.height || window.innerHeight || 800) * 0.5),
          )
          let imgW = crop.w
          let imgH = crop.h
          if (imgH > maxImgH && imgH > 0) {
            const s = maxImgH / imgH
            imgH = maxImgH
            imgW = Math.max(1, Math.round(crop.w * s))
          }
          const w = Math.max(440, imgW) + 24
          const h = imgH + 40 + 40 + 24
          sizeFloorRef.current = { w, h }
          onBlockResize(true)
          flowActive.current = true
          void (async () => {
            try {
              const win = getCurrentWindow()
              await win.hide()
              await win.setSize(new LogicalSize(w, h))
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
              setOcrSeedText('')
              setText('')
              setResult(null)
              setError('')
              setSending(true)
              setView('ocr')
            })
            onBlockResize(false)
          })()
        }}
      />
    )
  }

  if (view !== 'ocr') return null
  if (!ocrPreview?.src) {
    return <div className="h-0 w-0 overflow-hidden" aria-hidden />
  }

  return (
    <div
      className={cn(
        'min-w-[440px] rounded-lg bg-card text-card-foreground ring-1 ring-foreground/10',
        'shadow-[0_1px_2px_rgb(0_0_0/0.16)] dark:shadow-[0_1px_2px_rgb(0_0_0/0.55)]',
      )}
    >
      <OcrTranslate seedText={ocrSeedText} />
    </div>
  )
}
