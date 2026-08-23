import { Volume2 } from 'lucide-react'
import { useEffect } from 'react'
import { dictSpeak, type DictEntry } from './api/dict'
import { cn } from '@/lib/utils'

export function DictCard({
  entry,
  speakLang,
}: {
  entry: DictEntry
  speakLang?: string
}) {
  useEffect(() => {
    const w = entry.word.trim()
    if (!w) return
    // 延迟一拍：StrictMode 先挂再卸时 cleanup 会清掉，避免双播。
    const t = window.setTimeout(() => {
      void dictSpeak(w, speakLang).catch(() => {})
    }, 0)
    return () => clearTimeout(t)
  }, [entry.word, speakLang])

  return (
    <div className="px-4 py-2">
      <div className="flex items-center gap-2">
        <span className="text-[15px] font-medium leading-6">{entry.word}</span>
        {entry.phonetic ? (
          <span className="text-[13px] leading-5 text-muted-foreground">{entry.phonetic}</span>
        ) : null}
        <button
          type="button"
          className="inline-flex size-6 shrink-0 items-center justify-center rounded text-muted-foreground hover:text-foreground"
          aria-label="发音"
          onClick={() => void dictSpeak(entry.word, speakLang).catch(() => {})}
        >
          <Volume2 className="size-3.5" />
        </button>
      </div>
      <ul className="mt-1 space-y-0.5">
        {entry.senses.map((s, i) => (
          <li
            key={`${s.pos}-${i}`}
            className="text-[14px] leading-5 break-words [overflow-wrap:anywhere] text-foreground/90"
          >
            {s.pos ? (
              <span className={cn('mr-1.5 text-[12px] text-muted-foreground')}>{s.pos}.</span>
            ) : null}
            {s.gloss}
          </li>
        ))}
      </ul>
    </div>
  )
}
