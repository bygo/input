import { useAtom, useSetAtom } from 'jotai'
import { cn } from '@/lib/utils'
import { setAutoGrammar, setSelectSpeak, setShortcut, setShowNative } from '../../api/settings'
import {
  autoGrammarAtom,
  composeShortcutAtom,
  grammarAtom,
  nativeAtom,
  ocrShortcutAtom,
  selectShortcutAtom,
  selectSpeakAtom,
  showNativeAtom,
} from '../../atoms'
import { ShortcutBind } from './ShortcutBind'

/** 快捷键三行 + 朗读 / 中文对照 / 自动解析。 */
export function Shortcuts({ onError }: { onError: (message: string) => void }) {
  const [composeShortcut, setComposeShortcut] = useAtom(composeShortcutAtom)
  const [selectShortcut, setSelectShortcut] = useAtom(selectShortcutAtom)
  const [ocrShortcut, setOcrShortcut] = useAtom(ocrShortcutAtom)
  const [selectSpeak, setSelectSpeakAtom] = useAtom(selectSpeakAtom)
  const [showNative, setShowNativeAtom] = useAtom(showNativeAtom)
  const [autoGrammar, setAutoGrammarAtom] = useAtom(autoGrammarAtom)
  const setNative = useSetAtom(nativeAtom)
  const setGrammar = useSetAtom(grammarAtom)

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
        <span className="text-[16px]">翻译后朗读</span>
        <button
          type="button"
          aria-label="翻译后朗读"
          className={cn(
            'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-[16px]',
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
      <div className="flex items-center justify-between gap-3">
        <span className="text-[16px]">中文译文</span>
        <button
          type="button"
          aria-label="中文译文"
          className={cn(
            'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-[16px]',
            showNative
              ? 'border-foreground/40 bg-muted'
              : 'border-border hover:bg-muted/50',
          )}
          onClick={() => {
            onError('')
            const next = !showNative
            setShowNativeAtom(next)
            if (!next) {
              setNative('')
            }
            void setShowNative(next)
              .then(s => setShowNativeAtom(s.showNative))
              .catch(e => onError(String(e)))
          }}
        >
          {showNative ? '开' : '关'}
        </button>
      </div>
      <div className="flex items-center justify-between gap-3">
        <span className="text-[16px]">自动解析</span>
        <button
          type="button"
          aria-label="自动解析"
          className={cn(
            'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-[16px]',
            autoGrammar
              ? 'border-foreground/40 bg-muted'
              : 'border-border hover:bg-muted/50',
          )}
          onClick={() => {
            onError('')
            const next = !autoGrammar
            setAutoGrammarAtom(next)
            if (!next) {
              setGrammar('')
            }
            void setAutoGrammar(next)
              .then(s => setAutoGrammarAtom(s.autoGrammar))
              .catch(e => onError(String(e)))
          }}
        >
          {autoGrammar ? '开' : '关'}
        </button>
      </div>
    </div>
  )
}
