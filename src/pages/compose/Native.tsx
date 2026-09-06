import type { ChangeEvent, KeyboardEvent } from 'react'
import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import { Loader2 } from 'lucide-react'
import { autosize } from '@/lib/dom'
import { cn } from '@/lib/utils'
import {
  dictAtom,
  diffPartsAtom,
  englishAtom,
  inputElAtom,
  loadTopAtom,
  nativeAtom,
  paneLockAtom,
  resultAtom,
  sendingAtom,
  textAtom,
} from '../../atoms'
import { EnglishText } from './EnglishText'
import { areaClass, paneStyle } from './style'
import { composeRunAtom } from './usePipeline'

/** 上栏：用户输入。 */
export function Native() {
  const [text, setText] = useAtom(textAtom)
  const [english, setEnglish] = useAtom(englishAtom)
  const setNative = useSetAtom(nativeAtom)
  const sending = useAtomValue(sendingAtom)
  const loadTop = useAtomValue(loadTopAtom)
  const diffParts = useAtomValue(diffPartsAtom)
  const paneLock = useAtomValue(paneLockAtom)
  const setInputEl = useSetAtom(inputElAtom)
  const setResult = useSetAtom(resultAtom)
  const setDict = useSetAtom(dictAtom)
  const run = useSetAtom(composeRunAtom)

  const topStyle = paneLock
    ? { height: paneLock.top, minHeight: paneLock.top, maxHeight: paneLock.top }
    : paneStyle

  if (loadTop) {
    return (
      <div className="shrink-0 overflow-y-auto px-4 py-2" style={topStyle}>
        {text.trim() && (
          <p className="text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text">
            {text}
          </p>
        )}
        <div className="flex h-6 items-center">
          <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label="处理中" />
        </div>
      </div>
    )
  }

  const showDiff = !!diffParts && !!text.trim() && !!english.trim()

  return (
    <div className="relative shrink-0 overflow-hidden" style={paneLock ? topStyle : undefined}>
      {showDiff && (
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 overflow-hidden px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere]"
        >
          <EnglishText text={text} parts={diffParts} mode="del" />
        </div>
      )}
      <textarea
        ref={setInputEl}
        value={text}
        rows={1}
        disabled={sending}
        className={cn(
          areaClass,
          'relative z-[1] shrink-0',
          showDiff && 'bg-transparent text-transparent caret-foreground',
        )}
        onChange={(e: ChangeEvent<HTMLTextAreaElement>) => {
          setText(e.target.value)
          autosize(e.currentTarget)
          setEnglish('')
          setNative('')
          setResult(null)
          setDict(null)
        }}
        onKeyDown={(e: KeyboardEvent<HTMLTextAreaElement>) => {
          if (e.nativeEvent.isComposing || e.keyCode === 229) {
            return
          }
          if (e.key !== 'Enter') {
            return
          }
          e.preventDefault()
          if (english.trim()) {
            e.currentTarget.blur()
            return
          }
          e.currentTarget.blur()
          void run()
        }}
        onScroll={(e) => {
          if (!showDiff) {
            return
          }
          const bg = e.currentTarget.previousElementSibling as HTMLElement | null
          if (bg) {
            bg.scrollTop = e.currentTarget.scrollTop
          }
        }}
      />
    </div>
  )
}
