import { useAtomValue } from 'jotai'
import { Volume2 } from 'lucide-react'
import { useEffect } from 'react'
import { cn } from '@/lib/utils'
import { dictSpeak } from '../../api/dict'
import { dictAtom, languageAtom } from '../../atoms'

/** 词卡：读 dictAtom。 */
export function DictCard() {
  const entry = useAtomValue(dictAtom)
  const speakLang = useAtomValue(languageAtom)

  useEffect(() => {
    const w = entry?.word.trim() ?? ''
    if (!w) {
      return
    }
    const t = window.setTimeout(() => {
      void dictSpeak(w, speakLang).catch(() => {})
    }, 0)
    return () => clearTimeout(t)
  }, [entry?.word, speakLang])

  if (!entry) {
    return null
  }

  return (
    <div className="px-4 py-2">
      <div className="flex items-center gap-2">
        <span className="text-[15px] font-medium leading-6">{entry.word}</span>
        {entry.phonetic && (
          <span className="text-[13px] leading-5 text-muted-foreground">{entry.phonetic}</span>
        )}
        <button
          type="button"
          className="inline-flex size-6 shrink-0 items-center justify-center rounded text-muted-foreground hover:text-foreground"
          aria-label="发音"
          onClick={() => void dictSpeak(entry.word, speakLang).catch(() => {})}
        >
          <Volume2 className="size-3.5" />
        </button>
      </div>
      {entry.senses.length > 0 && (
        <ul className="mt-1 space-y-0.5">
          {entry.senses.map((s, i) => (
            <li
              key={`${s.pos}-${i}`}
              className="text-[14px] leading-5 break-words [overflow-wrap:anywhere] text-foreground/90"
            >
              {s.pos && (
                <span className={cn('mr-1.5 text-[12px] text-muted-foreground')}>
                  {s.pos}
                  .
                </span>
              )}
              {s.gloss}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
