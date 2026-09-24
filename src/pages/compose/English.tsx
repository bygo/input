import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import { Loader2, Volume2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Separator } from '@/components/ui/separator'
import { inTauri } from '@/lib/dom'
import { grammarAsk, sentenceGrammar } from '../../api/analyze'
import { dictLookup, dictSpeak, dictSpeaking, dictStop } from '../../api/dict'
import {
  autoGrammarAtom,
  botElAtom,
  dictAtom,
  diffPartsAtom,
  englishAtom,
  grammarAtom,
  grammarBusyAtom,
  languageAtom,
  loadBotAtom,
  nativeAtom,
  paneLockAtom,
  showNativeAtom,
} from '../../atoms'
import { EnglishText } from './EnglishText'
import { paneStyle } from './style'

/** 翻译面板：英语（点词出词卡）+ 解析/追问 + 可选中文对照。 */
export function English() {
  const english = useAtomValue(englishAtom)
  const [grammar, setGrammar] = useAtom(grammarAtom)
  const [grammarBusy, setGrammarBusy] = useAtom(grammarBusyAtom)
  const native = useAtomValue(nativeAtom)
  const showNative = useAtomValue(showNativeAtom)
  const autoGrammar = useAtomValue(autoGrammarAtom)
  const loadBot = useAtomValue(loadBotAtom)
  const diffParts = useAtomValue(diffPartsAtom)
  const language = useAtomValue(languageAtom)
  const paneLock = useAtomValue(paneLockAtom)
  const setBotEl = useSetAtom(botElAtom)
  const setDict = useSetAtom(dictAtom)
  const [askText, setAskText] = useState('')
  const [askThread, setAskThread] = useState<string[]>([])
  const [askBusy, setAskBusy] = useState(false)

  const showNativeLine = showNative && !!native.trim()

  useEffect(() => {
    setAskText('')
    setAskThread([])
    setAskBusy(false)
  }, [english, grammar])

  useEffect(() => {
    if (!autoGrammar || !inTauri()) {
      return
    }
    const sentence = english.trim()
    if (!sentence) {
      return
    }
    let alive = true
    setGrammar('')
    setGrammarBusy(true)
    void sentenceGrammar(sentence)
      .then((s) => {
        if (!alive) {
          return
        }
        const t = s.trim()
        setGrammar(t && !t.startsWith('{') ? t : '')
      })
      .catch(() => {
        if (alive) {
          setGrammar('')
        }
      })
      .finally(() => {
        if (alive) {
          setGrammarBusy(false)
        }
      })
    return () => {
      alive = false
    }
  }, [autoGrammar, english, setGrammar, setGrammarBusy])

  if (!english.trim() && !grammar.trim() && !showNativeLine && !loadBot && !grammarBusy) {
    return null
  }

  const botStyle
    = paneLock && paneLock.bot > 0
      ? { height: paneLock.bot, minHeight: paneLock.bot, maxHeight: paneLock.bot }
      : paneStyle

  return (
    <>
      <Separator className="shrink-0 bg-foreground/10" />
      <div
        ref={setBotEl}
        className="flex shrink-0 flex-col gap-1 overflow-y-auto px-4 py-2"
        style={botStyle}
      >
        <div className="flex items-start gap-1">
          <div className="min-w-0 flex-1">
            {english.trim() && (
              <p className="text-[16px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text">
                <EnglishText
                  text={english}
                  parts={diffParts}
                  mode="add"
                  onWord={(part) => {
                    if (!inTauri()) {
                      return
                    }
                    void dictLookup(part)
                      .then(entry => setDict(entry ?? { word: part, senses: [] }))
                      .catch(() => {})
                  }}
                />
              </p>
            )}
            {loadBot && (
              <div className="flex h-6 items-center">
                <Loader2
                  className="size-4 animate-spin text-muted-foreground"
                  aria-label="处理中"
                />
              </div>
            )}
          </div>
          {english.trim() && !autoGrammar && (
            <button
              type="button"
              disabled={grammarBusy}
              className="mt-0.5 inline-flex h-6 shrink-0 items-center justify-center rounded px-1.5 text-[13px] text-muted-foreground hover:text-foreground disabled:opacity-50"
              aria-label="解析"
              onClick={(ev) => {
                ev.stopPropagation()
                if (!inTauri() || grammarBusy) {
                  return
                }
                const sentence = english.trim()
                if (!sentence) {
                  return
                }
                setGrammar('')
                setGrammarBusy(true)
                void sentenceGrammar(sentence)
                  .then((s) => {
                    const t = s.trim()
                    setGrammar(t && !t.startsWith('{') ? t : '')
                  })
                  .catch(() => setGrammar(''))
                  .finally(() => setGrammarBusy(false))
              }}
            >
              析
            </button>
          )}
          {english.trim() && (
            <button
              type="button"
              className="mt-0.5 inline-flex size-6 shrink-0 items-center justify-center rounded text-muted-foreground hover:text-foreground"
              aria-label="朗读"
              onClick={(ev) => {
                ev.stopPropagation()
                if (!inTauri()) {
                  return
                }
                void dictSpeaking()
                  .catch(() => false)
                  .then(on => (on ? dictStop() : dictSpeak(english, language)))
                  .catch(() => {})
              }}
            >
              <Volume2 className="size-3.5" />
            </button>
          )}
        </div>
        {grammarBusy && (
          <div className="flex h-5 items-center">
            <Loader2
              className="size-3.5 animate-spin text-muted-foreground"
              aria-label="解析中"
            />
          </div>
        )}
        {grammar.trim() && (
          <div>
            <p className="text-[14px] leading-5 break-words whitespace-pre-wrap [overflow-wrap:anywhere] text-muted-foreground select-text">
              {grammar}
            </p>
            {askThread.map((r, i) => (
              <p
                key={i}
                className="mt-0.5 text-[14px] leading-5 break-words whitespace-pre-wrap [overflow-wrap:anywhere] text-muted-foreground select-text"
              >
                {r}
              </p>
            ))}
            <input
              value={askText}
              disabled={askBusy}
              placeholder="继续追问"
              className="mt-1 block w-full border-0 bg-transparent px-0 py-0.5 text-[14px] leading-5 outline-none placeholder:text-muted-foreground/50 disabled:opacity-50"
              onChange={e => setAskText(e.target.value)}
              onKeyDown={(e) => {
                if (e.nativeEvent.isComposing || e.keyCode === 229) {
                  return
                }
                if (e.key !== 'Enter') {
                  return
                }
                e.preventDefault()
                e.stopPropagation()
                const q = askText.trim()
                if (!q || askBusy || !inTauri()) {
                  return
                }
                const sentence = english.trim()
                if (!sentence) {
                  return
                }
                setAskBusy(true)
                void grammarAsk(sentence, grammar, q)
                  .then((s) => {
                    const t = s.trim()
                    if (t && !t.startsWith('{')) {
                      setAskThread(prev => [...prev, t])
                    }
                    setAskText('')
                  })
                  .catch(() => {})
                  .finally(() => setAskBusy(false))
              }}
            />
          </div>
        )}
        {showNativeLine && (
          <p className="text-[16px] leading-5 break-words whitespace-pre-wrap [overflow-wrap:anywhere] text-muted-foreground select-text">
            {native}
          </p>
        )}
      </div>
    </>
  )
}
