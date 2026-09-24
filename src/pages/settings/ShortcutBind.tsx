import { listen } from '@tauri-apps/api/event'
import { useEffect, useRef, useState } from 'react'
import { cn } from '@/lib/utils'
import { beginShortcutCapture, endShortcutCapture } from '../../api/settings'
import { formatShortcut, shortcutFromEvent } from '../../shortcut'

/** 单条快捷键录制按钮。 */
export function ShortcutBind({
  label,
  value,
  onBound,
  onError,
}: {
  label: string
  value: string
  onBound: (shortcut: string) => Promise<void>
  onError: (message: string) => void
}) {
  const [recording, setRecording] = useState(false)
  const recordingRef = useRef(false)
  const btnRef = useRef<HTMLButtonElement>(null)
  const onBoundRef = useRef(onBound)
  const onErrorRef = useRef(onError)
  onBoundRef.current = onBound
  onErrorRef.current = onError

  const stop = () => {
    if (!recordingRef.current) {
      return
    }
    recordingRef.current = false
    setRecording(false)
    void endShortcutCapture().catch(() => {})
  }

  useEffect(() => {
    return () => {
      void endShortcutCapture().catch(() => {})
    }
  }, [])

  useEffect(() => {
    if (!recording) {
      return
    }
    const bind = (shortcut: string) => {
      if (!recordingRef.current) {
        return
      }
      void onBoundRef
        .current(shortcut)
        .then(() => stop())
        .catch((err) => {
          stop()
          onErrorRef.current(String(err))
        })
    }
    const unCap = listen<string>('shortcut-captured', (e) => {
      if (e.payload) {
        bind(e.payload)
      }
    })
    // Esc：清空该快捷键（原生捕获与页面 keydown 两路）
    const unCancel = listen('shortcut-capture-cancel', () => bind(''))
    const onKeyDown = (e: KeyboardEvent) => {
      if (!recordingRef.current) {
        return
      }
      if (
        ['Meta', 'Control', 'Alt', 'Shift', 'MetaLeft', 'MetaRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight'].includes(
          e.key,
        )
        || ['MetaLeft', 'MetaRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight'].includes(
          e.code,
        )
      ) {
        return
      }
      e.preventDefault()
      e.stopPropagation()
      if (e.repeat) {
        return
      }
      const plainEsc
        = e.key === 'Escape' && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey
      if (plainEsc) {
        bind('')
        return
      }
      const next = shortcutFromEvent(e)
      if (next) {
        bind(next)
      }
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => {
      unCap.then(f => f())
      unCancel.then(f => f())
      window.removeEventListener('keydown', onKeyDown, true)
    }
  }, [recording])

  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-[16px]">{label}</span>
      <button
        ref={btnRef}
        type="button"
        aria-label={label}
        className={cn(
          'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-[16px] tabular-nums',
          recording ? 'border-foreground/40 bg-muted' : 'border-border hover:bg-muted/50',
        )}
        onClick={() => {
          onError('')
          recordingRef.current = true
          setRecording(true)
          void beginShortcutCapture().catch(() => {})
          requestAnimationFrame(() => btnRef.current?.focus())
        }}
      >
        {recording ? '按下组合键' : formatShortcut(value) || '未设置'}
      </button>
    </div>
  )
}
