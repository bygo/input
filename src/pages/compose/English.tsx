import { useAtomValue, useSetAtom } from 'jotai'
import { Loader2, Volume2 } from 'lucide-react'
import { Separator } from '@/components/ui/separator'
import { inTauri } from '@/lib/dom'
import { dictLookup, dictSpeak, dictSpeaking, dictStop } from '../../api/dict'
import {
  botElAtom,
  dictAtom,
  diffPartsAtom,
  englishAtom,
  languageAtom,
  loadBotAtom,
  nativeAtom,
  paneLockAtom,
} from '../../atoms'
import { EnglishText } from './EnglishText'
import { paneStyle } from './style'

/** 翻译面板：英语 + 中文（改写一次附带 native）。 */
export function English() {
  const english = useAtomValue(englishAtom)
  const native = useAtomValue(nativeAtom)
  const loadBot = useAtomValue(loadBotAtom)
  const diffParts = useAtomValue(diffPartsAtom)
  const language = useAtomValue(languageAtom)
  const paneLock = useAtomValue(paneLockAtom)
  const setBotEl = useSetAtom(botElAtom)
  const setDict = useSetAtom(dictAtom)

  if (!english.trim() && !native.trim() && !loadBot) {
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
              <p className="text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text">
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
        {native.trim() && (
          <p className="text-[14px] leading-5 break-words whitespace-pre-wrap [overflow-wrap:anywhere] text-muted-foreground select-text">
            {native}
          </p>
        )}
      </div>
    </>
  )
}
