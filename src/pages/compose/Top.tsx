import { useAtomValue } from 'jotai'
import { Separator } from '@/components/ui/separator'
import { ocrPreviewAtom } from '../../atoms'

/** OCR 截图预览（有则置顶）。 */
export function Top() {
  const ocrPreview = useAtomValue(ocrPreviewAtom)
  if (!ocrPreview?.src) {
    return null
  }

  return (
    <>
      <div className="shrink-0 overflow-hidden bg-muted p-2">
        <img
          src={ocrPreview.src}
          alt="截屏选区"
          className="block max-w-none object-contain"
          style={
            ocrPreview.w > 0 && ocrPreview.h > 0
              ? { width: ocrPreview.w, height: ocrPreview.h }
              : undefined
          }
          draggable={false}
        />
      </div>
      <Separator className="shrink-0 bg-foreground/10" />
    </>
  )
}
