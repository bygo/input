import { useAtom } from 'jotai'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { analyze } from './api/analyze'
import { pasteBack } from './api/paste'
import { errorAtom, languageAtom, resultAtom, sendingAtom, textAtom, viewAtom } from './atoms'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'

const INPUT_MAX_H = 160

function autosize(el: HTMLTextAreaElement | null) {
  if (!el) return
  el.style.height = '0px'
  el.style.height = `${Math.min(el.scrollHeight, INPUT_MAX_H)}px`
}

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

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
  const [error, setError] = useAtom(errorAtom)
  const [language] = useAtom(languageAtom)
  const [view] = useAtom(viewAtom)
  const [selected, setSelected] = useState(0)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const listRef = useRef<HTMLElement>(null)
  const sentences = result?.sentences ?? []
  const active = sentences.length ? Math.min(selected, sentences.length - 1) : 0
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

  const run = async (src = text, into?: string) => {
    if (!src.trim() || sending) return
    setSending(true)
    setError('')
    try {
      setResult(await analyze(src, into))
    } catch (e) {
      setResult(null)
      setError(String(e))
    } finally {
      setSending(false)
    }
  }

  useEffect(() => {
    if (!inTauri()) return
    const unHidden = listen('overlay-hidden', () => {
      setText('')
      setResult(null)
      setError('')
      setSending(false)
    })
    const un = listen<{ selected?: string }>('overlay-shown', (e) => {
      setResult(null)
      setError('')
      const selected = e.payload?.selected?.trim() ?? ''
      if (selected) {
        setText(selected)
        setSending(true)
        analyze(selected, undefined, true)
          .then(setResult)
          .catch((err) => {
            setResult(null)
            setError(String(err))
          })
          .finally(() => setSending(false))
        requestAnimationFrame(() => inputRef.current?.focus())
        return
      }
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        inputRef.current?.select()
      })
    })
    return () => {
      un.then((f) => f())
      unHidden.then((f) => f())
    }
  }, [setError, setResult, setSending, setText])

  useEffect(() => {
    setSelected(0)
  }, [result])

  useLayoutEffect(() => {
    autosize(inputRef.current)
  }, [text, sending])

  useEffect(() => {
    if (!sentences.length) return
    listRef.current
      ?.querySelector(`[data-index="${active}"]`)
      ?.scrollIntoView({ block: 'nearest' })
  }, [active, sentences.length])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (view !== 'compose') return
      if (e.isComposing || e.keyCode === 229) return
      const list = result?.sentences ?? []
      if (!list.length) return
      if (e.metaKey || e.ctrlKey) {
        const n = Number(e.key)
        if (n >= 1 && n <= 9 && list[n - 1]) {
          e.preventDefault()
          void insertRef.current(list[n - 1])
        }
        return
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelected((i) => Math.min(list.length - 1, i + 1))
        return
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelected((i) => Math.max(0, i - 1))
        return
      }
      if (e.key === 'Enter') {
        e.preventDefault()
        void insertRef.current(list[Math.min(selected, list.length - 1)])
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [result, selected, view])

  return (
    <div className="flex min-w-0 flex-col">
      <div className="relative min-w-0" aria-busy={sending || undefined}>
        <textarea
          ref={inputRef}
          value={text}
          rows={1}
          autoFocus
          disabled={sending}
          placeholder={`译入${language} / ${language}纠错`}
          className={cn(
            'block min-h-10 w-full max-h-40 resize-none overflow-y-auto border-0 bg-transparent px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] outline-none',
            'placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50',
          )}
          onChange={(e) => {
            setText(e.target.value)
            autosize(e.currentTarget)
            if (result) setResult(null)
          }}
          onKeyDown={(e) => {
            if (e.nativeEvent.isComposing) return
            if (sentences.length && (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter')) {
              e.preventDefault()
              return
            }
            if (e.key !== 'Enter') return
            e.preventDefault()
            void run()
          }}
        />
      </div>
      {(error || result) && <Separator />}
      {error && <p className="px-4 py-3 text-sm break-words [overflow-wrap:anywhere] text-destructive">{error}</p>}
      {result && (
        <section
          ref={listRef}
          className="flex max-h-[360px] min-w-0 flex-col overflow-auto"
        >
          {result.sentences.map((sentence, i) => (
            <button
              key={i}
              type="button"
              data-index={i}
              data-selected={i === active ? 'true' : undefined}
              aria-selected={i === active}
              className={cn(
                'block min-h-10 w-full min-w-0 px-4 py-2 text-left text-[15px] leading-6 last:rounded-b-lg',
                i === active
                  ? 'bg-zinc-300 text-foreground'
                  : 'hover:bg-zinc-100',
              )}
              onClick={() => void insert(sentence)}
            >
              <span className="block max-w-full break-words whitespace-pre-wrap [overflow-wrap:anywhere]">
                {sentence}
              </span>
            </button>
          ))}
        </section>
      )}
    </div>
  )
}
