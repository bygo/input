import { useAtom } from 'jotai'
import { Loader2 } from 'lucide-react'
import { useEffect, useLayoutEffect, useRef } from 'react'
import { listen } from '@tauri-apps/api/event'
import { analyze } from './api/analyze'
import { dictLookup } from './api/dict'
import { pasteBack } from './api/paste'
import {
  type Analysis,
  dictAtom,
  errorAtom,
  languageAtom,
  resultAtom,
  sendingAtom,
  textAtom,
  viewAtom,
} from './atoms'
import { DictCard } from './DictCard'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'

const INPUT_MAX_H = 160

function autosize(el: HTMLTextAreaElement | null) {
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, INPUT_MAX_H)}px`
}

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** 单个英文单词：只查词典。中文无空格，不能按「一词」拦截。 */
function isEnglishWord(raw: string) {
  const t = raw.trim()
  if (!t || t.length > 64) return false
  return /^[A-Za-z]+(?:['’-][A-Za-z]+)*$/.test(t)
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    const el = document.createElement('textarea')
    el.value = text
    el.style.position = 'fixed'
    el.style.left = '-9999px'
    document.body.appendChild(el)
    el.select()
    document.execCommand('copy')
    el.remove()
  }
}

export function Compose() {
  const [text, setText] = useAtom(textAtom)
  const [sending, setSending] = useAtom(sendingAtom)
  const [result, setResult] = useAtom(resultAtom)
  const [dict, setDict] = useAtom(dictAtom)
  const [error, setError] = useAtom(errorAtom)
  const [language] = useAtom(languageAtom)
  const [view] = useAtom(viewAtom)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const req = useRef(0)
  const out = result?.sentences[0]?.trim() || ''
  const insertRef = useRef<(sentence: string) => Promise<void>>(async () => {})

  useEffect(() => {
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [])

  const insert = async (sentence: string) => {
    if (!sentence) return
    if (inTauri()) {
      try {
        await pasteBack(sentence)
      } catch (e) {
        setError(String(e))
      }
    } else {
      await copyText(sentence)
    }
  }
  insertRef.current = insert

  const run = async (src = text, select = false) => {
    if (!src.trim()) return
    if (sending && !select) return
    const id = ++req.current
    setSending(true)
    setError('')
    setResult(null)
    setDict(null)
    // 英文单词：只词典；未命中再走翻译。中文等一律翻译/改写。
    if (isEnglishWord(src) && inTauri()) {
      try {
        const e = await dictLookup(src)
        if (id !== req.current) return
        if (e) {
          setDict(e)
          setSending(false)
          return
        }
      } catch {
        /* fall through to analyze */
      }
    }
    try {
      const r = await analyze(src, undefined, select, id)
      if (id !== req.current) return
      setResult(r)
    } catch (e) {
      if (id !== req.current) return
      setResult(null)
      setError(String(e))
    } finally {
      if (id === req.current) setSending(false)
    }
  }
  const runRef = useRef(run)
  runRef.current = run

  useEffect(() => {
    if (!inTauri()) return
    let alive = true
    let offHidden: (() => void) | undefined
    let offShown: (() => void) | undefined
    let offPartial: (() => void) | undefined
    listen('overlay-hidden', () => {
      if (!alive) return
      req.current += 1
      setText('')
      setResult(null)
      setDict(null)
      setError('')
      setSending(false)
    }).then((f) => {
      if (!alive) f()
      else offHidden = f
    })
    listen<{ selected?: string }>('overlay-shown', (e) => {
      // OCR 复用同事件；卸载后 unlisten 可能尚未完成，靠 alive 忽略。
      if (!alive) return
      setResult(null)
      setDict(null)
      setError('')
      const selected = e.payload?.selected?.trim() ?? ''
      if (selected) {
        setText(selected)
        requestAnimationFrame(() => inputRef.current?.focus())
        void runRef.current(selected, true)
        return
      }
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        inputRef.current?.select()
      })
    }).then((f) => {
      if (!alive) f()
      else offShown = f
    })
    listen<{ reqId?: number; partial?: string; source?: string }>('analyze-partial', (e) => {
      if (!alive) return
      const id = e.payload?.reqId
      if (id != null && id !== req.current) return
      const partial = e.payload?.partial ?? ''
      if (!partial) return
      setResult({
        sentences: [partial],
        source: (e.payload?.source as Analysis['source']) || 'local-mt',
      })
    }).then((f) => {
      if (!alive) f()
      else offPartial = f
    })
    return () => {
      alive = false
      offHidden?.()
      offShown?.()
      offPartial?.()
    }
  }, [setDict, setError, setResult, setSending, setText])
  useLayoutEffect(() => {
    autosize(inputRef.current)
  }, [text, sending, dict])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (view !== 'compose') return
      if (e.isComposing || e.keyCode === 229) return
      if (!out) return
      if (e.key === 'Enter') {
        e.preventDefault()
        e.stopPropagation()
        void insertRef.current(out)
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [out, view])

  const hasBody = !!(error || dict || out || sending)

  return (
    <div className="flex w-[440px] min-w-[440px] flex-col" aria-busy={sending || undefined}>
      <textarea
        ref={inputRef}
        value={text}
        rows={1}
        autoFocus
        disabled={sending}
        className={cn(
          'block min-h-10 w-full max-h-40 resize-none overflow-y-auto border-0 bg-transparent px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] outline-none',
          'disabled:cursor-not-allowed disabled:opacity-50',
        )}
        onChange={(e) => {
          setText(e.target.value)
          autosize(e.currentTarget)
          if (result) setResult(null)
          if (dict) setDict(null)
        }}
        onKeyDown={(e) => {
          if (e.nativeEvent.isComposing) return
          if (out && e.key === 'Enter') {
            e.preventDefault()
            return
          }
          if (e.key !== 'Enter') return
          e.preventDefault()
          void run()
        }}
      />
      <Separator />
      <div className="min-h-10 rounded-b-lg">
        {hasBody ? (
          <div className="relative max-h-[360px] overflow-auto">
            {dict ? (
              <>
                <DictCard entry={dict} speakLang={language} />
                {out || sending || error ? <Separator /> : null}
              </>
            ) : null}
            {error ? (
              <p className="px-4 py-2 text-sm break-words [overflow-wrap:anywhere] text-destructive">
                {error}
              </p>
            ) : null}
            {out ? (
              <div className="px-4 py-2">
                <p className="text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text">
                  {out}
                </p>
              </div>
            ) : null}
            {sending ? (
              <div className={cn('flex items-center px-4', out || dict || error ? 'pb-2' : 'min-h-10 py-2')}>
                <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label="处理中" />
              </div>
            ) : null}
          </div>
        ) : (
          <p className="px-4 py-2 text-[15px] leading-6">&nbsp;</p>
        )}
      </div>
    </div>
  )
}
