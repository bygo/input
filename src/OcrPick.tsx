import { useCallback, useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from 'react'
import { invoke } from '@tauri-apps/api/core'

type Pt = { x: number; y: number }

export function OcrPick({
  freeze,
  onCancel,
  onFinishSelect,
}: {
  /** WeChat-style frozen screenshot (asset URL). */
  freeze: string
  onCancel: () => void
  /** Leave freeze mask and reserve final (crop+input) size. */
  onFinishSelect: (crop: { w: number; h: number }) => void
}) {
  const [origin, setOrigin] = useState<Pt | null>(null)
  const [curr, setCurr] = useState<Pt | null>(null)
  const dragging = useRef(false)
  const latest = useRef<{ origin: Pt | null; curr: Pt | null }>({ origin: null, curr: null })
  const imgSize = useRef({ w: 0, h: 0 })
  const finishing = useRef(false)
  const imgRef = useRef<HTMLImageElement>(null)

  const finish = useCallback(
    (a: Pt, b: Pt) => {
      if (finishing.current) return
      const vw = window.innerWidth || 1
      const vh = window.innerHeight || 1
      const iw = imgSize.current.w || vw
      const ih = imgSize.current.h || vh
      const sx = iw / vw
      const sy = ih / vh
      const x = Math.min(a.x, b.x) * sx
      const y = Math.min(a.y, b.y) * sy
      const w = Math.abs(a.x - b.x) * sx
      const h = Math.abs(a.y - b.y) * sy
      if (w < 4 || h < 4) {
        onCancel()
        return
      }
      finishing.current = true
      // Drop freeze mask + reserve crop size; preview arrives via ocr-recognizing.
      onFinishSelect({ w: Math.max(1, Math.round(w)), h: Math.max(1, Math.round(h)) })
      void invoke('ocr_finish_pick', { x, y, w, h }).catch((e) => console.error(e))
    },
    [onCancel, onFinishSelect],
  )

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        e.stopPropagation()
        void invoke('ocr_cancel_pick').finally(onCancel)
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [onCancel])

  useEffect(() => {
    const el = imgRef.current
    if (!el) return
    if (el.complete && el.naturalWidth > 0) {
      imgSize.current = { w: el.naturalWidth, h: el.naturalHeight }
    }
  }, [freeze])

  const draw =
    origin && curr
      ? {
          x: Math.min(origin.x, curr.x),
          y: Math.min(origin.y, curr.y),
          w: Math.abs(curr.x - origin.x),
          h: Math.abs(curr.y - origin.y),
        }
      : null

  const point = (e: ReactMouseEvent): Pt => ({ x: e.clientX, y: e.clientY })

  return (
    <div className="fixed inset-0 z-[100] cursor-crosshair select-none overflow-hidden bg-black">
      {/* 蒙层始终盖在图上，亮窗时不会先闪一张未遮罩截图 */}
      {freeze ? (
        <img
          ref={imgRef}
          src={freeze}
          alt=""
          draggable={false}
          className="pointer-events-none absolute inset-0 h-full w-full max-w-none object-fill"
          onLoad={(e) => {
            imgSize.current = {
              w: e.currentTarget.naturalWidth,
              h: e.currentTarget.naturalHeight,
            }
          }}
        />
      ) : null}
      <div
        className="absolute inset-0"
        onMouseDown={(e) => {
          if (finishing.current || e.button !== 0) return
          dragging.current = true
          const p = point(e)
          latest.current = { origin: p, curr: p }
          setOrigin(p)
          setCurr(p)
        }}
        onMouseMove={(e) => {
          if (!dragging.current || finishing.current) return
          const p = point(e)
          latest.current.curr = p
          setCurr(p)
        }}
        onMouseUp={() => {
          if (!dragging.current || finishing.current) return
          dragging.current = false
          const { origin: a, curr: b } = latest.current
          if (a && b) finish(a, b)
          else onCancel()
        }}
      >
        {draw ? (
          <div
            className="pointer-events-none absolute border border-white/90 bg-transparent shadow-[0_0_0_9999px_rgba(0,0,0,0.45)]"
            style={{ left: draw.x, top: draw.y, width: draw.w, height: draw.h }}
          />
        ) : (
          <div className="pointer-events-none absolute inset-0 bg-black/35" />
        )}
      </div>
      <div className="pointer-events-none absolute top-3 left-1/2 -translate-x-1/2 rounded-md bg-black/70 px-3 py-1.5 text-xs text-white">
        拖选文字区域 · Esc 取消
      </div>
    </div>
  )
}
