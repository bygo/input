import { useAtom } from 'jotai'
import { useRef, useState } from 'react'
import { ocrPreviewAtom, viewAtom } from '../atoms'
import { Compose } from './compose'
import { OcrSession } from './ocr/Session'
import { Settings } from './settings'
import { Boot } from './shell/Boot'
import { Card } from './shell/Card'
import { Hotkeys } from './shell/Hotkeys'
import { Window } from './shell/Window'

/** 根编排：常挂副作用 + Compose | Settings。 */
export function Root() {
  const [view] = useAtom(viewAtom)
  const [ocrPreview] = useAtom(ocrPreviewAtom)
  const [ocrPickMode, setOcrPickMode] = useState(false)
  const [sizeEpoch, setSizeEpoch] = useState(0)
  const cardRef = useRef<HTMLDivElement>(null)

  return (
    <div
      ref={cardRef}
      className={ocrPickMode ? 'h-screen w-screen bg-transparent' : 'w-max p-3'}
    >
      <Boot setSizeEpoch={setSizeEpoch} />
      <Hotkeys ocrPickMode={ocrPickMode} />
      <Window
        cardRef={cardRef}
        ocrPickMode={ocrPickMode}
        sizeEpoch={sizeEpoch}
        view={view}
        ocrPreviewSrc={ocrPreview?.src}
      />
      <OcrSession onPickMode={setOcrPickMode} />
      {view !== 'ocr' && !ocrPickMode && (
        <Card>{view === 'settings' ? <Settings /> : <Compose />}</Card>
      )}
    </div>
  )
}
