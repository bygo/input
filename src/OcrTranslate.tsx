import { useAtom } from 'jotai'
import { Loader2 } from 'lucide-react'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { analyze } from './api/analyze'
import { pasteBack } from './api/paste'
import { errorAtom, ocrPreviewAtom, resultAtom, sendingAtom } from './atoms'
import { cn } from '@/lib/utils'

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const AREA_MAX_H = 360

function autosize(el: HTMLTextAreaElement | null) {
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, AREA_MAX_H)}px`
}

const areaClass =
  'block min-h-10 w-full resize-none overflow-y-auto border-0 bg-transparent px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] outline-none disabled:cursor-not-allowed disabled:opacity-50'

/** 截图翻译沙盒：上图 / 中原文 / 下译文（原文与译文均可编辑）。 */

export function OcrTranslate({ seedText = '' }: { seedText?: string }) {
  const [preview, setPreview] = useAtom(ocrPreviewAtom)
  const [, setSending] = useAtom(sendingAtom)
  const [, setResult] = useAtom(resultAtom)
  const [error, setError] = useAtom(errorAtom)
  const [ocrText, setOcrText] = useState('')
  const [outText, setOutText] = useState('')
  const [ocrLoading, setOcrLoading] = useState(true)
  const [translating, setTranslating] = useState(false)
  const ocrRef = useRef<HTMLTextAreaElement>(null)
  const outRef = useRef<HTMLTextAreaElement>(null)
  const req = useRef(0)
  const seedDone = useRef('')

  useLayoutEffect(() => {
    autosize(ocrRef.current)
    autosize(outRef.current)
  }, [ocrText, outText, ocrLoading, translating, error])

  // App 暂存的识别文本（可能早于本组件 listen 就绪）。
  useEffect(() => {
    const selected = seedText.trim()
    if (!selected || seedDone.current === selected) return
    seedDone.current = selected
    const id = ++req.current
    setOcrText(selected)
    setOcrLoading(false)
    setTranslating(true)
    setSending(true)
    setError('')
    setOutText('')
    setResult(null)
    analyze(selected, undefined, true, id)
      .then((r) => {
        if (id !== req.current) return
        setResult(r)
        setOutText(r.sentences[0]?.trim() || '')
      })
      .catch((err) => {
        if (id !== req.current) return
        setResult(null)
        setError(String(err))
      })
      .finally(() => {
        if (id !== req.current) return
        setTranslating(false)
        setSending(false)
      })
  }, [seedText, setSending, setResult, setError])

  useEffect(() => {
    if (!inTauri()) return
    let alive = true
    let offHidden: (() => void) | undefined
    let offBusy: (() => void) | undefined
    let offShown: (() => void) | undefined
    let offErr: (() => void) | undefined
    listen('overlay-hidden', () => {
      if (!alive) return
      req.current += 1
      seedDone.current = ''
      setPreview(null)
      setOcrText('')
      setOutText('')
      setOcrLoading(true)
      setTranslating(false)
      setResult(null)
      setError('')
      setSending(false)
    }).then((f) => {
      if (!alive) f()
      else offHidden = f
    })
    listen<{ preview?: string }>('ocr-recognizing', (e) => {
      if (!alive) return
      const src = e.payload?.preview?.trim() ?? ''
      if (src) setPreview((p) => (p ? { ...p, src } : { src, w: 0, h: 0 }))
      req.current += 1
      seedDone.current = ''
      setOcrLoading(true)
      setTranslating(false)
      setError('')
      setOutText('')
      setResult(null)
    }).then((f) => {
      if (!alive) f()
      else offBusy = f
    })
    listen<{ selected?: string }>('overlay-shown', (e) => {
      if (!alive) return
      const selected = e.payload?.selected?.trim() ?? ''
      if (!selected) return
      if (seedDone.current === selected) return
      seedDone.current = selected
      const id = ++req.current
      setOcrText(selected)
      setOcrLoading(false)
      setTranslating(true)
      setSending(true)
      setError('')
      setOutText('')
      setResult(null)
      analyze(selected, undefined, true, id)
        .then((r) => {
          if (id !== req.current) return
          setResult(r)
          setOutText(r.sentences[0]?.trim() || '')
        })
        .catch((err) => {
          if (id !== req.current) return
          setResult(null)
          setError(String(err))
        })
        .finally(() => {
          if (id !== req.current) return
          setTranslating(false)
          setSending(false)
        })
    }).then((f) => {
      if (!alive) f()
      else offShown = f
    })
    listen<string>('ocr-error', (e) => {
      if (!alive) return
      req.current += 1
      setOcrLoading(false)
      setTranslating(false)
      setSending(false)
      setError(e.payload || '截屏取词失败')
    }).then((f) => {
      if (!alive) f()
      else offErr = f
    })
    let offPartial: (() => void) | undefined
    listen<{ reqId?: number; partial?: string; source?: string }>('analyze-partial', (e) => {
      if (!alive) return
      const id = e.payload?.reqId
      if (id != null && id !== req.current) return
      const partial = e.payload?.partial ?? ''
      if (!partial) return
      setOutText(partial)
      setResult({
        sentences: [partial],
        source: e.payload?.source || 'local-mt',
      })
    }).then((f) => {
      if (!alive) f()
      else offPartial = f
    })
    return () => {
      alive = false
      offHidden?.()
      offBusy?.()
      offShown?.()
      offErr?.()
      offPartial?.()
    }
  }, [setPreview, setResult, setError, setSending])
  const boxW = preview?.w && preview.w > 0 ? Math.max(440, preview.w) : 440

  return (
    <div
      className="flex min-w-[440px] flex-col"
      style={{ width: boxW }}
      aria-busy={ocrLoading || translating || undefined}
    >
      <div className="flex shrink-0 justify-center overflow-hidden rounded-t-lg border-b border-foreground/10">
        {preview?.src ? (
          <img
            src={preview.src}
            alt="截屏选区"
            className="block max-w-none bg-muted object-contain"
            style={
              preview.w > 0 && preview.h > 0
                ? { width: preview.w, height: preview.h }
                : undefined
            }
            draggable={false}
          />
        ) : (
          <div
            className="bg-muted"
            style={{ width: preview?.w || 1, height: preview?.h || 40 }}
            aria-hidden
          />
        )}
      </div>

      <div className="min-h-10 border-b border-foreground/10">
        {ocrLoading ? (
          <div className="flex min-h-10 items-center px-4 py-2">
            <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label="识别中" />
          </div>
        ) : (
          <textarea
            ref={ocrRef}
            value={ocrText}
            rows={1}
            className={cn(areaClass)}
            onChange={(e) => {
              setOcrText(e.target.value)
              autosize(e.currentTarget)
            }}
            onKeyDown={(e) => {
              if (e.nativeEvent.isComposing) return
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                const src = ocrText.trim()
                if (!src || translating) return
                const id = ++req.current
                setTranslating(true)
                setSending(true)
                setError('')
                setOutText('')
                analyze(src, undefined, true, id)
                  .then((r) => {
                    if (id !== req.current) return
                    setResult(r)
                    setOutText(r.sentences[0]?.trim() || '')
                  })
                  .catch((err) => {
                    if (id !== req.current) return
                    setResult(null)
                    setError(String(err))
                  })
                  .finally(() => {
                    if (id !== req.current) return
                    setTranslating(false)
                    setSending(false)
                  })
              }
            }}
          />
        )}
      </div>

      <div className="min-h-10 rounded-b-lg">
        {error ? (
          <p className="px-4 py-2 text-sm break-words [overflow-wrap:anywhere] text-destructive">{error}</p>
        ) : !outText && (translating || ocrLoading) ? (
          <div className="flex min-h-10 items-center px-4 py-2">
            <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label="处理中" />
          </div>
        ) : (
          <div>
            <textarea
              ref={outRef}
              value={outText}
              rows={1}
              disabled={translating}
              className={cn(areaClass)}
              onChange={(e) => {
                setOutText(e.target.value)
                autosize(e.currentTarget)
              }}
              onKeyDown={(e) => {
                if (e.nativeEvent.isComposing) return
                if (translating) return
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault()
                  const t = outText.trim()
                  if (!t) return
                  void pasteBack(t).catch((err) => setError(String(err)))
                }
              }}
            />
            {translating || ocrLoading ? (
              <div className="flex items-center px-4 pb-2">
                <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label="处理中" />
              </div>
            ) : null}
          </div>
        )}
      </div>
    </div>
  )
}
