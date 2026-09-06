import { useAtom } from 'jotai'
import { cn } from '@/lib/utils'
import { setSelectSpeak, setShortcut } from '../../api/settings'
import {
  composeShortcutAtom,
  ocrShortcutAtom,
  selectShortcutAtom,
  selectSpeakAtom,
} from '../../atoms'
import { ShortcutBind } from './ShortcutBind'

/** 快捷键三行 + 翻译后朗读。 */
export function Shortcuts({ onError }: { onError: (message: string) => void }) {
  const [composeShortcut, setComposeShortcut] = useAtom(composeShortcutAtom)
  const [selectShortcut, setSelectShortcut] = useAtom(selectShortcutAtom)
  const [ocrShortcut, setOcrShortcut] = useAtom(ocrShortcutAtom)
  const [selectSpeak, setSelectSpeakAtom] = useAtom(selectSpeakAtom)

  return (
    <div className="flex flex-col gap-2">
      <ShortcutBind
        label="输入"
        value={composeShortcut}
        onError={onError}
        onBound={async (shortcut) => {
          const s = await setShortcut('compose', shortcut)
          setComposeShortcut(s.composeShortcut)
        }}
      />
      <ShortcutBind
        label="选中翻译"
        value={selectShortcut}
        onError={onError}
        onBound={async (shortcut) => {
          const s = await setShortcut('select', shortcut)
          setSelectShortcut(s.selectShortcut)
        }}
      />
      <ShortcutBind
        label="截屏取词"
        value={ocrShortcut}
        onError={onError}
        onBound={async (shortcut) => {
          const s = await setShortcut('ocr', shortcut)
          setOcrShortcut(s.ocrShortcut)
        }}
      />
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm">翻译后朗读</span>
        <button
          type="button"
          aria-label="翻译后朗读"
          className={cn(
            'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-sm',
            selectSpeak
              ? 'border-foreground/40 bg-muted'
              : 'border-border hover:bg-muted/50',
          )}
          onClick={() => {
            onError('')
            const next = !selectSpeak
            setSelectSpeakAtom(next)
            void setSelectSpeak(next)
              .then(s => setSelectSpeakAtom(s.selectSpeak))
              .catch(e => onError(String(e)))
          }}
        >
          {selectSpeak ? '开' : '关'}
        </button>
      </div>
    </div>
  )
}
