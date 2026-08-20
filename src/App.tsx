import { useAtom } from 'jotai'
import { Loader2 } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { analyze } from './api/analyze'
import { load, save, testProvider } from './api/settings'
import {
  defaultSettings,
  errorAtom,
  resultAtom,
  sendingAtom,
  settingsAtom,
  textAtom,
  viewAtom,
  type Settings,
} from './atoms'
import { PROVIDERS, ProviderIcon, providerById, type ProviderId } from './providers'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'

const VERDICT: Record<string, string> = {
  clean: '无明显问题',
  needs_work: '有语法问题',
  broken: '较难理解',
}

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export default function App() {
  const [text, setText] = useAtom(textAtom)
  const [sending, setSending] = useAtom(sendingAtom)
  const [result, setResult] = useAtom(resultAtom)
  const [error, setError] = useAtom(errorAtom)
  const [view, setView] = useAtom(viewAtom)
  const [settings, setSettings] = useAtom(settingsAtom)
  const inputRef = useRef<HTMLInputElement>(null)
  const cardRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!inTauri()) return
    load()
      .then((s) =>
        setSettings({
          ...defaultSettings(),
          ...s,
          keys: s.keys ?? {},
          lastModels: s.lastModels ?? {},
        }),
      )
      .catch((e) => setError(String(e)))
    const unShown = listen('overlay-shown', () => {
      setView('compose')
      setResult(null)
      setError('')
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        inputRef.current?.select()
      })
    })
    const unSettings = listen('open-settings', () => setView('settings'))
    return () => {
      unShown.then((f) => f())
      unSettings.then((f) => f())
    }
  }, [setSettings, setView, setResult, setError])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (inTauri()) getCurrentWindow().hide()
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

  useEffect(() => {
    const el = cardRef.current
    if (!el || !inTauri()) return
    const apply = () => {
      const r = el.getBoundingClientRect()
      getCurrentWindow()
        .setSize(new LogicalSize(Math.ceil(r.width), Math.ceil(r.height)))
        .then(() => invoke('reposition'))
    }
    apply()
    const ro = new ResizeObserver(apply)
    ro.observe(el)
    return () => ro.disconnect()
  }, [view, result, error])

  const run = async () => {
    if (!text.trim() || sending) return
    setSending(true)
    setError('')
    try {
      setResult(await analyze(text))
    } catch (e) {
      setResult(null)
      setError(String(e))
    } finally {
      setSending(false)
    }
  }

  return (
    <div
      ref={cardRef}
      className="w-[440px] rounded-2xl bg-card text-card-foreground shadow-xl ring-1 ring-foreground/10"
    >
      {view === 'settings' ? (
        <SettingsPanel
          settings={settings}
          setSettings={setSettings}
          error={error}
          setError={setError}
          onBack={() => setView('compose')}
        />
      ) : (
        <div className="flex flex-col">
          <div className="relative">
            <Input
              ref={inputRef}
              value={text}
              autoFocus
              disabled={sending}
              placeholder="检查英语语法"
              className="h-14 border-0 bg-transparent px-4 text-[15px] shadow-none focus-visible:border-transparent focus-visible:ring-0 md:text-[15px]"
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  run()
                }
              }}
            />
            {sending && (
              <Loader2 className="absolute top-1/2 right-4 size-4 -translate-y-1/2 animate-spin text-muted-foreground" />
            )}
          </div>
          {(error || result) && <Separator />}
          {error && view === 'compose' && (
            <p className="px-4 py-3 text-sm text-destructive">{error}</p>
          )}
          {result && (
            <section className="flex max-h-[360px] flex-col gap-3 overflow-auto px-4 py-3">
              <div className="text-xs text-muted-foreground">{VERDICT[result.verdict] ?? result.verdict}</div>
              {result.corrected && <p className="text-sm leading-relaxed">{result.corrected}</p>}
              {result.issues.map((issue, i) => (
                <div key={i} className="flex flex-col gap-1">
                  <div className="flex items-center gap-1.5">
                    <Badge variant="outline">{issue.category}</Badge>
                    {issue.span && <span className="text-xs text-destructive">「{issue.span}」</span>}
                  </div>
                  <p className="text-sm">{issue.explanation}</p>
                  {issue.suggestion && (
                    <p className="text-sm text-muted-foreground">建议：{issue.suggestion}</p>
                  )}
                </div>
              ))}
            </section>
          )}
        </div>
      )}
    </div>
  )
}

function SettingsPanel({
  settings,
  setSettings,
  error,
  setError,
  onBack,
}: {
  settings: Settings
  setSettings: (s: Settings) => void
  error: string
  setError: (s: string) => void
  onBack: () => void
}) {
  const current = providerById(settings.provider)
  const models = current.models.some((m) => m.id === settings.model)
    ? current.models
    : [{ id: settings.model, label: settings.model }, ...current.models]

  const pickProvider = (id: ProviderId) => {
    const next = providerById(id)
    const keys = { ...settings.keys, [settings.provider]: settings.apiKey }
    const lastModels = { ...settings.lastModels, [settings.provider]: settings.model }
    setSettings({
      ...settings,
      provider: id,
      baseUrl: next.baseUrl,
      apiKey: keys[id] ?? '',
      model: lastModels[id] ?? next.models[0].id,
      keys,
      lastModels,
    })
  }

  return (
    <div className="flex flex-col gap-4 p-4">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">设置</span>
        <Button variant="ghost" size="sm" onClick={onBack}>
          返回
        </Button>
      </div>

      <div className="flex flex-col gap-2">
        <Label className="text-xs text-muted-foreground">服务</Label>
        <div className="grid grid-cols-3 gap-2">
          {PROVIDERS.map((p) => {
            const selected = settings.provider === p.id
            return (
              <button
                key={p.id}
                type="button"
                onClick={() => pickProvider(p.id)}
                className={cn(
                  'flex flex-col items-center gap-1.5 rounded-xl border px-2 py-2.5 text-xs font-medium',
                  selected ? 'border-foreground/25 bg-muted' : 'border-border hover:bg-muted/50',
                )}
              >
                <ProviderIcon id={p.id} />
                {p.name}
              </button>
            )
          })}
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Label className="text-xs text-muted-foreground">模型</Label>
        <div className="overflow-hidden rounded-xl border border-border">
          {models.map((m, i) => {
            const selected = settings.model === m.id
            return (
              <button
                key={m.id}
                type="button"
                onClick={() => setSettings({ ...settings, model: m.id })}
                className={cn(
                  'flex w-full items-center justify-between px-3 py-2 text-left text-sm',
                  i > 0 && 'border-t border-border',
                  selected ? 'bg-muted' : 'hover:bg-muted/40',
                )}
              >
                <span>{m.label}</span>
                {selected && <span className="text-xs text-muted-foreground">使用中</span>}
              </button>
            )
          })}
        </div>
      </div>

      <div className="flex flex-col gap-1.5">
        <Label className="text-xs text-muted-foreground">API Key</Label>
        <Input
          type="password"
          placeholder={current.keyHint}
          value={settings.apiKey}
          onChange={(e) =>
            setSettings({
              ...settings,
              apiKey: e.target.value,
              keys: { ...settings.keys, [settings.provider]: e.target.value },
            })
          }
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <Label className="text-xs text-muted-foreground">快捷键</Label>
        <Input
          value={settings.shortcut}
          onChange={(e) => setSettings({ ...settings, shortcut: e.target.value })}
        />
      </div>

      {settings.provider === 'ollama' && (
        <p className="text-xs text-muted-foreground">本地需先 ollama serve，并 ollama pull 对应模型。</p>
      )}

      <div className="flex gap-2">
        <Button
          className="flex-1"
          onClick={async () => {
            setError('')
            try {
              await save(settings)
              setError('已保存')
            } catch (e) {
              setError(String(e))
            }
          }}
        >
          保存
        </Button>
        <Button
          variant="outline"
          onClick={async () => {
            setError('')
            try {
              setError(await testProvider(settings))
            } catch (e) {
              setError(String(e))
            }
          }}
        >
          测试
        </Button>
      </div>

      {error && (
        <p className={error === '已保存' || error === '连接成功' ? 'text-sm text-muted-foreground' : 'text-sm text-destructive'}>
          {error}
        </p>
      )}
    </div>
  )
}
