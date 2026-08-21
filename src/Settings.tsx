import { useAtom } from 'jotai'
import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { installLocal, localStatus, setLocal, type LocalStatus } from './api/local'
import {
  beginShortcutCapture,
  endShortcutCapture,
  setNativeLanguage,
  setShortcut,
  setTargetLanguage,
} from './api/settings'
import { composeShortcutAtom, languageAtom, nativeLanguageAtom, selectShortcutAtom, viewAtom } from './atoms'
import { LANGUAGES } from './languages'
import { formatShortcut, shortcutFromEvent } from './shortcut'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

function Tip({ label, text }: { label: string; text: string }) {
  return (
    <span className="group relative w-fit text-sm">
      {label}
      <span className="pointer-events-none absolute top-full left-0 z-20 mt-1 hidden w-max max-w-[220px] rounded-md bg-foreground px-2 py-1 text-xs leading-snug text-background group-hover:block">
        {text}
      </span>
    </span>
  )
}

function LangGrid({
  value,
  onPick,
}: {
  value: string
  onPick: (lang: string) => void
}) {
  return (
    <div className="grid grid-cols-4 gap-2">
      {LANGUAGES.map((lang) => {
        const selected = value === lang
        return (
          <button
            key={lang}
            type="button"
            onClick={() => onPick(lang)}
            className={cn(
              'rounded-xl border px-2 py-2 text-xs font-medium',
              selected ? 'border-foreground/25 bg-muted' : 'border-border hover:bg-muted/50',
            )}
          >
            {lang}
          </button>
        )
      })}
    </div>
  )
}

const empty: LocalStatus = {
  enabled: false,
  installed: false,
  running: false,
  hasModel: false,
  busy: false,
  message: '',
  percent: 0,
}

function ShortcutBind({
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
    if (!recordingRef.current) return
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
    if (!recording) return
    const bind = (shortcut: string) => {
      if (!recordingRef.current) return
      void onBoundRef
        .current(shortcut)
        .then(() => stop())
        .catch((err) => {
          stop()
          onErrorRef.current(String(err))
        })
    }
    const unCap = listen<string>('shortcut-captured', (e) => {
      if (e.payload) bind(e.payload)
    })
    const unCancel = listen('shortcut-capture-cancel', () => stop())
    const onKeyDown = (e: KeyboardEvent) => {
      if (!recordingRef.current) return
      if (['Meta', 'Control', 'Alt', 'Shift', 'MetaLeft', 'MetaRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight'].includes(e.key) || ['MetaLeft', 'MetaRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight'].includes(e.code)) {
        return
      }
      e.preventDefault()
      e.stopPropagation()
      if (e.repeat) return
      const plainEsc =
        e.key === 'Escape' && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey
      if (plainEsc) {
        stop()
        return
      }
      const next = shortcutFromEvent(e)
      if (next) bind(next)
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => {
      unCap.then((f) => f())
      unCancel.then((f) => f())
      window.removeEventListener('keydown', onKeyDown, true)
    }
  }, [recording])

  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-sm">{label}</span>
      <button
        ref={btnRef}
        type="button"
        aria-label={label}
        className={cn(
          'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-sm tabular-nums',
          recording
            ? 'border-foreground/40 bg-muted'
            : 'border-border hover:bg-muted/50',
        )}
        onClick={() => {
          onError('')
          recordingRef.current = true
          setRecording(true)
          void beginShortcutCapture().catch(() => {})
          requestAnimationFrame(() => btnRef.current?.focus())
        }}
      >
        {recording ? '按下组合键' : formatShortcut(value)}
      </button>
    </div>
  )
}

export function Settings() {
  const [, setView] = useAtom(viewAtom)
  const [language, setLanguage] = useAtom(languageAtom)
  const [native, setNative] = useAtom(nativeLanguageAtom)
  const [composeShortcut, setComposeShortcut] = useAtom(composeShortcutAtom)
  const [selectShortcut, setSelectShortcut] = useAtom(selectShortcutAtom)
  const [st, setSt] = useState<LocalStatus>(empty)
  const [error, setError] = useState('')

  const refresh = () => {
    localStatus()
      .then(setSt)
      .catch((e) => setError(String(e)))
  }

  useEffect(() => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return
    refresh()
    const un = listen<{ stage: string; message: string; percent: number }>('local-model-progress', (e) => {
      setSt((s) => ({
        ...s,
        busy: e.payload.stage !== 'done' && e.payload.stage !== 'error',
        message: e.payload.message,
        percent: e.payload.percent,
        enabled: e.payload.stage === 'done' ? true : s.enabled,
        hasModel: e.payload.stage === 'done' ? true : s.hasModel,
        running: e.payload.stage === 'done' ? true : s.running,
        installed: e.payload.stage === 'done' ? true : s.installed,
      }))
      if (e.payload.stage === 'done') refresh()
    })
    return () => {
      un.then((f) => f())
    }
  }, [])

  const ready = st.enabled && st.hasModel && st.running

  return (
    <div className="flex flex-col gap-4 p-4">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">设置</span>
        <Button variant="ghost" size="sm" onClick={() => setView('compose')}>
          返回
        </Button>
      </div>

      <div className="flex flex-col gap-2">
        <Tip label="我现在的语言" text={`选中文字后按 ${formatShortcut(selectShortcut)}，译成这种语言。`} />
        <LangGrid
          value={native}
          onPick={async (lang) => {
            setNative(lang)
            try {
              await setNativeLanguage(lang)
            } catch (e) {
              setError(String(e))
            }
          }}
        />
      </div>

      <div className="flex flex-col gap-2">
        <Tip
          label="学习的语言"
          text={`按 ${formatShortcut(composeShortcut)} 输入时：已是该语言则纠错润色，否则翻译成该语言。`}
        />
        <LangGrid
          value={language}
          onPick={async (lang) => {
            setLanguage(lang)
            try {
              await setTargetLanguage(lang)
            } catch (e) {
              setError(String(e))
            }
          }}
        />
      </div>

      <div className="flex flex-col gap-2">
        <span className="text-sm">快捷键</span>
        <ShortcutBind
          label="输入"
          value={composeShortcut}
          onError={setError}
          onBound={async (shortcut) => {
            const s = await setShortcut('compose', shortcut)
            setComposeShortcut(s.composeShortcut)
          }}
        />
        <ShortcutBind
          label="选中翻译"
          value={selectShortcut}
          onError={setError}
          onBound={async (shortcut) => {
            const s = await setShortcut('select', shortcut)
            setSelectShortcut(s.selectShortcut)
          }}
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <span className="text-sm">本地模型增强</span>
        <p className="text-xs text-muted-foreground">
          自动安装本机模型，随应用启动。安装约 2GB，之后改写可走本地。
        </p>
      </div>

      <div className="text-xs text-muted-foreground">
        {ready ? '运行中 · Qwen2.5 3B' : st.message || (st.enabled ? '启动中…' : '未安装')}
      </div>

      {st.busy && (
        <div className="h-1.5 overflow-hidden rounded-full bg-muted">
          <div className="h-full bg-foreground/70" style={{ width: `${st.percent}%` }} />
        </div>
      )}

      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="flex gap-2">
        {!ready && (
          <Button
            className="flex-1"
            disabled={st.busy}
            onClick={async () => {
              setError('')
              try {
                await installLocal()
                refresh()
              } catch (e) {
                setError(String(e))
                refresh()
              }
            }}
          >
            {st.busy ? '安装中…' : '安装并启用'}
          </Button>
        )}
        {st.hasModel && (
          <Button
            variant="outline"
            className={ready ? 'flex-1' : ''}
            disabled={st.busy}
            onClick={async () => {
              setError('')
              try {
                await setLocal(!st.enabled)
                refresh()
              } catch (e) {
                setError(String(e))
              }
            }}
          >
            {st.enabled ? '停用' : '启用'}
          </Button>
        )}
      </div>
    </div>
  )
}
