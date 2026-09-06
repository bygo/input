import { useAtomValue } from 'jotai'
import { Separator } from '@/components/ui/separator'
import {
  dictAtom,
  englishAtom,
  errorAtom,
  ocrPreviewAtom,
  sendingAtom,
  textAtom,
} from '../../atoms'
import { DictCard } from './DictCard'
import { English } from './English'
import { Native } from './Native'
import { paneStyle } from './style'
import { Top } from './Top'
import { useBridge } from './useBridge'
import { usePaneLayout } from './usePaneLayout'
import { usePipeline } from './usePipeline'

/**
 * 单列： [OCR?] → 输入 → 英语结果 → 词卡
 */
export function Compose() {
  usePipeline()
  useBridge()
  usePaneLayout()

  const text = useAtomValue(textAtom)
  const english = useAtomValue(englishAtom)
  const sending = useAtomValue(sendingAtom)
  const error = useAtomValue(errorAtom)
  const dict = useAtomValue(dictAtom)
  const ocrPreview = useAtomValue(ocrPreviewAtom)

  const imgW = ocrPreview?.w && ocrPreview.w > 0 ? ocrPreview.w : 0
  const ocrPad = imgW > 0 ? 8 : 0
  const boxW = Math.max(360, imgW > 0 ? imgW + ocrPad * 2 : 0)
  const ocrPending
    = !!ocrPreview?.src && !sending && !text.trim() && !english.trim() && !error
  const wordOnly = !!dict && !english.trim() && !ocrPreview?.src

  if (wordOnly) {
    return (
      <div className="flex w-[360px] min-w-[360px] flex-col rounded-lg">
        <DictCard />
      </div>
    )
  }

  return (
    <div
      className="flex min-w-[360px] flex-col"
      style={{ width: boxW }}
      aria-busy={sending || undefined}
    >
      <Top />

      {ocrPending
        ? (
            <>
              <div className="shrink-0 overflow-y-auto px-4 py-2" style={paneStyle} />
              <Separator className="shrink-0 bg-foreground/10" />
              <div className="shrink-0 overflow-y-auto px-4 py-2" style={paneStyle} />
            </>
          )
        : (
            <>
              <Native />
              <English />
            </>
          )}

      {error && (
        <>
          <Separator className="shrink-0 bg-foreground/10" />
          <p className="shrink-0 px-4 py-2 text-sm break-words [overflow-wrap:anywhere] text-destructive">
            {error}
          </p>
        </>
      )}

      {dict && (
        <>
          <Separator className="shrink-0 bg-foreground/10" />
          <div className="shrink-0">
            <DictCard />
          </div>
        </>
      )}
    </div>
  )
}
