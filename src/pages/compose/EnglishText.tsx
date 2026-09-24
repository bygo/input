import type { DiffPart } from '@/lib/text'
import { isEnglishWord } from '@/lib/text'

const WORD_RE = /([A-Z]+(?:['’-][A-Z]+)*)/i

function Words({
  text,
  onWord,
  hover,
}: {
  text: string
  onWord?: (word: string) => void
  hover: 'opacity' | 'foreground'
}) {
  const cls
    = hover === 'opacity'
      ? 'cursor-pointer hover:opacity-70'
      : 'cursor-pointer hover:text-foreground/70'
  // hover 中划线时整词一起亮
  const compoundCls
    = hover === 'opacity'
      ? '[&:has([data-hyphen]:hover)>[data-bit]]:opacity-70'
      : '[&:has([data-hyphen]:hover)>[data-bit]]:text-foreground/70'
  return (
    <>
      {text.split(WORD_RE).map((part, i) => {
        if (!onWord || !isEnglishWord(part)) {
          return <span key={i}>{part}</span>
        }
        // 含中划线：点词段查词段，点 "-" 查整词（如 plague-carrying）
        if (!part.includes('-')) {
          return (
            <span
              key={i}
              className={cls}
              onClick={(ev) => {
                ev.stopPropagation()
                onWord(part)
              }}
            >
              {part}
            </span>
          )
        }
        return (
          <span key={i} className={compoundCls}>
            {part.split(/(-)/).map((bit, j) => {
              if (!bit) {
                return null
              }
              const lookup = bit === '-' ? part : bit
              return (
                <span
                  key={j}
                  data-bit=""
                  {...(bit === '-' ? { 'data-hyphen': '' } : {})}
                  className={cls}
                  onClick={(ev) => {
                    ev.stopPropagation()
                    onWord(lookup)
                  }}
                >
                  {bit}
                </span>
              )
            })}
          </span>
        )
      })}
    </>
  )
}

/** mode=del：原文顶区（滤 add，标删）；mode=add：英语栏（滤 del，标增）。 */
export function EnglishText({
  text,
  parts,
  mode,
  onWord,
}: {
  text?: string
  parts?: DiffPart[] | null
  mode: 'del' | 'add'
  onWord?: (word: string) => void
}) {
  if (parts) {
    const drop = mode === 'del' ? 'add' : 'del'
    return (
      <>
        {parts
          .filter(p => p.t !== drop)
          .map((p, i) => {
            if (mode === 'del') {
              return p.t === 'del'
                ? (
                    <span
                      key={i}
                      className="rounded-sm bg-red-500/20 text-red-700 dark:text-red-400"
                    >
                      {p.s}
                    </span>
                  )
                : (
                    <span key={i} className="text-muted-foreground">
                      {p.s}
                    </span>
                  )
            }
            const body = <Words text={p.s} onWord={onWord} hover="opacity" />
            return p.t === 'add'
              ? (
                  <span
                    key={i}
                    className="rounded-sm bg-green-500/20 text-green-800 dark:text-green-400"
                  >
                    {body}
                  </span>
                )
              : (
                  <span key={i}>{body}</span>
                )
          })}
      </>
    )
  }

  if (mode === 'del') {
    return <span className="text-muted-foreground">{text}</span>
  }

  return (
    <>
      {(text ?? '').split('\n').map((line, li) => (
        <span key={li}>
          {li > 0 && '\n'}
          {line.length === 0
            ? (
                '\u00A0'
              )
            : (
                <Words text={line} onWord={onWord} hover="foreground" />
              )}
        </span>
      ))}
    </>
  )
}
