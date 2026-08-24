import { useAtom } from 'jotai'
import { Loader2, Volume2 } from 'lucide-react'
import { useEffect, useLayoutEffect, useRef, useState, type ChangeEvent, type KeyboardEvent } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { analyze } from './api/analyze'
import { dictLookup, dictSpeak, dictSpeaking, dictStop } from './api/dict'
import { pasteBack } from './api/paste'
import {
  type Analysis,
  composePunchAtom,
  dictAtom,
  errorAtom,
  languageAtom,
  nativeLanguageAtom,
  ocrPreviewAtom,
  resultAtom,
  sendingAtom,
  textAtom,
  viewAtom,
} from './atoms'
import { DictCard } from './DictCard'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'

const INPUT_MAX_H = 240
const LINE_H = 40 // 一行 min-h-10
const SEP_H = 1 // Separator h-px

function autosize(el: HTMLTextAreaElement | null, minH = 0, maxH = INPUT_MAX_H) {
  if (!el) return
  el.style.height = 'auto'
  const h = Math.max(minH, el.scrollHeight)
  el.style.height = `${maxH < Number.POSITIVE_INFINITY ? Math.min(h, maxH) : h}px`
}

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

function isEnglishWord(raw: string) {
  const t = raw.trim()
  if (!t || t.length > 64) return false
  return /^[A-Za-z]+(?:['’-][A-Za-z]+)*$/.test(t)
}

/** 几乎全是拉丁字母才算纯英文；夹汉字等一律算非英文。 */
function looksEnglish(raw: string) {
  const letters = raw.replace(
    /[^A-Za-z\u4e00-\u9fff\u3040-\u30ff\uac00-\ud7af\u0400-\u04ff\u0600-\u06ff]/g,
    '',
  )
  if (!letters) return false
  return /^[A-Za-z]+$/.test(letters)
}

/** 词级 LCS diff：eq / del / add */
function wordDiff(a: string, b: string): { t: 'eq' | 'del' | 'add'; s: string }[] {
  const A = a.split(/(\s+)/).filter((x) => x.length > 0)
  const B = b.split(/(\s+)/).filter((x) => x.length > 0)
  const n = A.length
  const m = B.length
  const dp: number[][] = Array.from({ length: n + 1 }, () => Array(m + 1).fill(0))
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1])
    }
  }
  const out: { t: 'eq' | 'del' | 'add'; s: string }[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (A[i] === B[j]) {
      out.push({ t: 'eq', s: A[i] })
      i += 1
      j += 1
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      out.push({ t: 'del', s: A[i] })
      i += 1
    } else {
      out.push({ t: 'add', s: B[j] })
      j += 1
    }
  }
  while (i < n) {
    out.push({ t: 'del', s: A[i] })
    i += 1
  }
  while (j < m) {
    out.push({ t: 'add', s: B[j] })
    j += 1
  }
  return out
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    const el = document.createElement('textarea')
    el.value = text
    el.style.position = 'fixed'
    el.style.left = '-9999px'
    document.body.appendChild(el)
    el.select()
    document.execCommand('copy')
    el.remove()
  }
}

const areaClass =
  'block min-h-10 w-full max-h-60 resize-none overflow-y-auto border-0 bg-transparent px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] outline-none disabled:cursor-not-allowed disabled:opacity-50'

/**
 * 结构固定：
 *   [OCR? | 原文?] 左全列 | [输入栏 → 英语栏 → 词卡]
 *
 * 手动输入英文：原文整列在左（同 OCR）；右上母语、右下英文
 */
export function Compose() {
  const [text, setText] = useAtom(textAtom)
  const [english, setEnglish] = useState('')
  /** 手动输入英文时的原句（左全列）；null = 不显示左列 */
  const [original, setOriginal] = useState<string | null>(null)
  const [sending, setSending] = useAtom(sendingAtom)
  const [, setResult] = useAtom(resultAtom)
  const [dict, setDict] = useAtom(dictAtom)
  const [error, setError] = useAtom(errorAtom)
  const [language] = useAtom(languageAtom)
  const [native] = useAtom(nativeLanguageAtom)
  const [view] = useAtom(viewAtom)
  const [ocrPreview, setOcrPreview] = useAtom(ocrPreviewAtom)
  const [, setComposePunch] = useAtom(composePunchAtom)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const origRef = useRef<HTMLTextAreaElement>(null)
  const rightRef = useRef<HTMLDivElement>(null)
  const topSecRef = useRef<HTMLDivElement>(null)
  const botSecRef = useRef<HTMLDivElement>(null)
  const [rightTaller, setRightTaller] = useState(false)
  /** 原文左列贴英语栏：top = 母语+分隔，height = 英语栏 */
  const [origLayout, setOrigLayout] = useState({ top: 0, height: 40 })
  /** 词卡弹出后钉死一二栏高度，避免词卡高低切换时挤动 */
  const paneLockRef = useRef<{ top: number; bot: number } | null>(null)
  const [paneLock, setPaneLock] = useState<{ top: number; bot: number } | null>(null)
  const req = useRef(0)
  const insertRef = useRef<(sentence: string) => Promise<void>>(async () => {})
  /** 英文源 → 母语：loading / 结果在输入栏 */
  const toNative = useRef(false)
  const waitInput = sending && toNative.current && !!english.trim()

  useEffect(() => {
    if (english.trim() || waitInput) return
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [])

  const insert = async (sentence: string) => {
    if (!sentence) return
    if (inTauri()) {
      try {
        await pasteBack(sentence)
      } catch (e) {
        setError(String(e))
      }
    } else {
      await copyText(sentence)
    }
  }
  insertRef.current = insert

  const run = async (src = text, select = false) => {
    if (!src.trim()) return
    if (sending && !select) return
    const id = ++req.current
    const enSrc = looksEnglish(src)
    toNative.current = enSrc
    setSending(true)
    setError('')
    setResult(null)
    setDict(null)

    if (isEnglishWord(src) && inTauri()) {
      try {
        const e = await dictLookup(src)
        if (id !== req.current) return
        if (e) {
          toNative.current = false
          setOriginal(null)
          setEnglish('')
          setText(src)
          setDict(e)
          setSending(false)
          return
        }
      } catch {
        /* fall through */
      }
    }

    // 纯英文 → 英语栏（再译母语）；非纯英文 → 母语栏（再译英语）
    if (enSrc) {
      setEnglish(src)
      setText('')
    } else {
      setOriginal(null)
      setText(src)
    }
    try {
      // 手动输入英文：原文左列 + 先纠错再译母语
      if (!select && enSrc) {
        setOriginal(src)
        const polished = await analyze(src, language, false, id)
        if (id !== req.current) return
        let en = polished.sentences[0]?.trim() || src
        if (!looksEnglish(en)) en = src
        setEnglish(en)
        setResult(polished)
        const r = await analyze(en, native, true, id)
        if (id !== req.current) return
        setResult(r)
        const out = r.sentences[0]?.trim() || ''
        if (out) setText(out)
        return
      }

      setOriginal(null)
      const r = await analyze(src, undefined, select, id)
      if (id !== req.current) return
      setResult(r)
      const out = r.sentences[0]?.trim() || ''
      if (!out) return
      // 按翻译方向落栏，不看译文「像不像英文」（混排也会进英语栏）
      if (toNative.current) setText(out)
      else setEnglish(out)
    } catch (e) {
      if (id !== req.current) return
      setResult(null)
      setError(String(e))
    } finally {
      if (id === req.current) setSending(false)
    }
  }
  const runRef = useRef(run)
  runRef.current = run

  useEffect(() => {
    if (!inTauri()) return
    let alive = true
    let offHidden: (() => void) | undefined
    let offShown: (() => void) | undefined
    let offPartial: (() => void) | undefined
    let offOcrText: (() => void) | undefined
    listen('overlay-hidden', () => {
      if (!alive) return
      req.current += 1
      toNative.current = false
      setText('')
      setEnglish('')
      setOriginal(null)
      setResult(null)
      setDict(null)
      setError('')
      setSending(false)
      setOcrPreview(null)
      void dictStop().catch(() => {})
    }).then((f) => {
      if (!alive) f()
      else offHidden = f
    })
    listen<{ selected?: string }>('overlay-shown', (e) => {
      if (!alive) return
      setResult(null)
      setDict(null)
      setError('')
      setOcrPreview(null)
      setEnglish('')
      setOriginal(null)
      const selected = e.payload?.selected?.trim() ?? ''
      if (selected) {
        void runRef.current(selected, true)
        return
      }
      setText('')
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        inputRef.current?.select()
      })
    }).then((f) => {
      if (!alive) f()
      else offShown = f
    })
    listen<string>('ocr-text', (e) => {
      if (!alive) return
      const selected = (typeof e.payload === 'string' ? e.payload : '').trim()
      if (!selected) return
      setResult(null)
      setDict(null)
      setError('')
      setOriginal(null)
      void runRef.current(selected, true)
    }).then((f) => {
      if (!alive) f()
      else offOcrText = f
    })
    listen<{ reqId?: number; partial?: string; source?: string }>('analyze-partial', (e) => {
      if (!alive) return
      const id = e.payload?.reqId
      if (id != null && id !== req.current) return
      const partial = e.payload?.partial ?? ''
      if (!partial) return
      setResult({
        sentences: [partial],
        source: (e.payload?.source as Analysis['source']) || 'local-mt',
      })
      // 与最终落栏一致：英→母语时英文 partial 仍进英语栏；母语→英一律进英语栏
      if (toNative.current) {
        if (looksEnglish(partial)) setEnglish(partial)
        else setText(partial)
      } else {
        setEnglish(partial)
      }
    }).then((f) => {
      if (!alive) f()
      else offPartial = f
    })
    return () => {
      alive = false
      offHidden?.()
      offShown?.()
      offPartial?.()
      offOcrText?.()
    }
  }, [setDict, setError, setOcrPreview, setResult, setSending, setText])

  const hasLeft = original !== null
  const showOrig = hasLeft
  const origPunch = showOrig && !ocrPreview?.src

  const ocrPadForMin = ocrPreview?.w && ocrPreview.w > 0 ? 8 : 0
  const shotHForAutosize =
    ocrPreview?.h && ocrPreview.h > 0 ? ocrPreview.h + ocrPadForMin * 2 : 0
  const twoPaneAutosize = !!(english.trim() || sending)
  // 两栏：扣 1px 分隔线后均分；单栏：整高对齐截图
  const colMinForAutosize =
    shotHForAutosize <= 0
      ? 0
      : twoPaneAutosize
        ? Math.max(40, Math.ceil((shotHForAutosize - 1) / 2))
        : shotHForAutosize
  const colMaxForAutosize =
    colMinForAutosize > 0 ? Math.max(INPUT_MAX_H, colMinForAutosize) : INPUT_MAX_H
  useLayoutEffect(() => {
    autosize(inputRef.current, colMinForAutosize, colMaxForAutosize)
    if (!origPunch) autosize(origRef.current)
  }, [text, sending, english, original, ocrPreview, colMinForAutosize, colMaxForAutosize, origPunch])

  // 有词卡时锁定母语/英语栏像素高；关掉词卡后解除
  useLayoutEffect(() => {
    if (!dict) {
      paneLockRef.current = null
      setPaneLock(null)
      return
    }
    if (paneLockRef.current) return
    const topEl = topSecRef.current ?? inputRef.current
    const botEl = botSecRef.current
    if (!topEl) return
    const lock = { top: topEl.offsetHeight, bot: botEl?.offsetHeight ?? 0 }
    if (lock.top < 1) return
    paneLockRef.current = lock
    setPaneLock(lock)
  }, [dict, text, english, sending])

  // 右栏高于截图时，露出的左下角补圆角
  useLayoutEffect(() => {
    const el = rightRef.current
    if (!el || !ocrPreview?.h) {
      setRightTaller(false)
      return
    }
    const pad = ocrPreview.w && ocrPreview.w > 0 ? 8 : 0
    const leftH = ocrPreview.h + pad * 2
    const sync = () => setRightTaller(el.clientHeight > leftH)
    sync()
    const ro = new ResizeObserver(sync)
    ro.observe(el)
    return () => ro.disconnect()
  }, [ocrPreview?.h, ocrPreview?.w, text, english, sending, original, error])

  useEffect(() => {
    setComposePunch(origPunch)
    return () => setComposePunch(false)
  }, [origPunch, setComposePunch])

  // 原文高度钉死英语栏，上方挖空（对称 OCR 图下挖空）
  useLayoutEffect(() => {
    if (!origPunch) return
    const topEl = topSecRef.current
    const botEl = botSecRef.current
    if (!topEl) return
    const sync = () => {
      const sep = botEl ? SEP_H : 0
      const top = topEl.offsetHeight + sep
      const height = botEl
        ? botEl.offsetHeight
        : Math.max(LINE_H, rightRef.current?.clientHeight ?? LINE_H)
      setOrigLayout({ top, height })
    }
    sync()
    const ro = new ResizeObserver(sync)
    ro.observe(topEl)
    if (botEl) ro.observe(botEl)
    return () => ro.disconnect()
  }, [origPunch, text, english, sending, original, error])

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (view !== 'compose') return
      if (e.isComposing || e.keyCode === 229) return
      if (!english.trim()) return
      if (e.key !== 'Enter') return
      if (document.activeElement === inputRef.current) return
      if (document.activeElement === origRef.current) return
      e.preventDefault()
      e.stopPropagation()
      void insertRef.current(english.trim())
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [english, view])

  const imgW = ocrPreview?.w && ocrPreview.w > 0 ? ocrPreview.w : 0
  const imgH = ocrPreview?.h && ocrPreview.h > 0 ? ocrPreview.h : 0
  const origW = showOrig ? 360 : 0
  // 截图四周 padding，露出底衬才像「被截下来的」
  const ocrPad = imgW > 0 ? 8 : 0
  const boxW = 360 + imgW + ocrPad * 2 + origW
  // 右栏总高对齐截图底衬；两栏扣分隔线后均分（上 ceil / 下 floor）
  const shotH = imgH > 0 ? imgH + ocrPad * 2 : 0
  const avail = shotH > 0 ? Math.max(0, shotH - SEP_H) : 0
  const colMinTop = shotH > 0 ? Math.max(LINE_H, Math.ceil(avail / 2)) : LINE_H
  const colMinBot = shotH > 0 ? Math.max(LINE_H, Math.floor(avail / 2)) : LINE_H
  const colMaxTop = Math.max(INPUT_MAX_H, colMinTop)
  const colMaxBot = Math.max(INPUT_MAX_H, colMinBot)
  // 单栏（无下栏）时整高对齐截图
  const colMinSolo = shotH > 0 ? shotH : LINE_H
  const colMaxSolo = Math.max(INPUT_MAX_H, colMinSolo)
  const topBoxStyle =
    imgH > 0
      ? ({ minHeight: colMinTop, maxHeight: colMaxTop } as const)
      : ({ minHeight: LINE_H, maxHeight: INPUT_MAX_H } as const)
  const botBoxStyle =
    imgH > 0
      ? ({ minHeight: colMinBot, maxHeight: colMaxBot } as const)
      : ({ minHeight: LINE_H, maxHeight: INPUT_MAX_H } as const)
  const soloBoxStyle =
    imgH > 0
      ? ({ minHeight: colMinSolo, maxHeight: colMaxSolo } as const)
      : ({ minHeight: LINE_H, maxHeight: INPUT_MAX_H } as const)
  const diffParts =
    hasLeft && original.trim() && english.trim()
      ? wordDiff(original.trim(), english.trim())
      : null
  // OCR 已出图、尚未出字：两栏空占位，不转圈
  const ocrPending =
    !!ocrPreview?.src && !sending && !text.trim() && !english.trim() && !error
  // 单词查中：只显示词卡。有 OCR 截图时绝不能走进这里（会拆掉截图）
  const wordOnly = !!dict && !english.trim() && !ocrPreview?.src

  if (wordOnly) {
    return (
      <div className="flex w-[360px] min-w-[360px] flex-col rounded-lg">
        <DictCard entry={dict} speakLang={language} />
      </div>
    )
  }

  const ocrPunch = !!ocrPreview?.src
  const paneChrome =
    'bg-card ring-1 ring-foreground/20 shadow-[0_1px_2px_rgb(0_0_0/0.16)] dark:shadow-[0_1px_2px_rgb(0_0_0/0.55)]'

  return (
    <div
      className="flex min-w-[360px] flex-col"
      style={{ width: boxW }}
      aria-busy={sending || undefined}
    >
    <div className={cn('flex min-w-[360px]', ocrPunch && 'items-start')}>
      {ocrPreview?.src ? (
        <div
          className="relative shrink-0 self-stretch"
          style={{ width: ocrPreview.w + ocrPad * 2 }}
        >
          <div
            className={cn(
              'overflow-hidden bg-muted p-2',
              ocrPunch
                ? cn('rounded-l-lg border-r border-foreground/20', paneChrome)
                : 'border-r border-foreground/20',
            )}
          >
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
          {/* 图下方挖空区：接住点击并正式关闭（避免点穿只失焦隐藏） */}
          {ocrPunch ? (
            <div
              role="button"
              tabIndex={-1}
              aria-label="关闭"
              className="absolute inset-x-0 bottom-0"
              style={{ top: (ocrPreview.h || 0) + ocrPad * 2 }}
              onMouseDown={(e) => {
                e.preventDefault()
                e.stopPropagation()
                void invoke('dismiss')
              }}
            />
          ) : null}
        </div>
      ) : null}

      {showOrig ? (
        <div className="relative w-[360px] shrink-0 self-stretch">
          <div
            className={cn(
              'flex overflow-hidden border-r border-foreground/20',
              origPunch
                ? cn('absolute inset-x-0', paneChrome, 'rounded-l-lg')
                : 'relative min-h-full',
              ocrPunch && paneChrome,
              ocrPunch && rightTaller && 'rounded-bl-lg',
            )}
            style={
              origPunch
                ? { top: origLayout.top, height: origLayout.height }
                : undefined
            }
          >
            {/* diff 高亮垫在底下；透明文字的 textarea 负责编辑 */}
            <div
              aria-hidden
              className="pointer-events-none absolute inset-0 overflow-hidden px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere]"
            >
              {diffParts
                ? diffParts
                    .filter((p) => p.t !== 'add')
                    .map((p, i) =>
                      p.t === 'del' ? (
                        <span
                          key={i}
                          className="rounded-sm bg-red-500/20 text-red-700 dark:text-red-400"
                        >
                          {p.s}
                        </span>
                      ) : (
                        <span key={i} className="text-muted-foreground">
                          {p.s}
                        </span>
                      ),
                    )
                : (
                  <span className="text-muted-foreground">{original}</span>
                )}
            </div>
            <textarea
              ref={origRef}
              value={original}
              rows={1}
              disabled={sending}
              className={cn(
                areaClass,
                'relative z-[1] min-h-full bg-transparent text-transparent caret-foreground',
                origPunch && 'max-h-none',
              )}
              onChange={(e: ChangeEvent<HTMLTextAreaElement>) => {
                setOriginal(e.target.value)
                if (!origPunch) autosize(e.currentTarget)
              }}
              onKeyDown={(e: KeyboardEvent<HTMLTextAreaElement>) => {
                if (e.nativeEvent.isComposing) return
                if (e.key !== 'Enter') return
                e.preventDefault()
                e.currentTarget.blur()
                void run(original, false)
              }}
              onScroll={(e) => {
                const bg = e.currentTarget.previousElementSibling as HTMLElement | null
                if (bg) bg.scrollTop = e.currentTarget.scrollTop
              }}
            />
          </div>
          {/* 原文上方挖空（对称 OCR 图下挖空） */}
          {origPunch && origLayout.top > 0 ? (
            <div
              role="button"
              tabIndex={-1}
              aria-label="关闭"
              className="absolute inset-x-0 top-0"
              style={{ height: origLayout.top }}
              onMouseDown={(e) => {
                e.preventDefault()
                e.stopPropagation()
                void invoke('dismiss')
              }}
            />
          ) : null}
          {origPunch ? (
            <div
              role="button"
              tabIndex={-1}
              aria-label="关闭"
              className="absolute inset-x-0 bottom-0"
              style={{ top: origLayout.top + origLayout.height }}
              onMouseDown={(e) => {
                e.preventDefault()
                e.stopPropagation()
                void invoke('dismiss')
              }}
            />
          ) : null}
        </div>
      ) : null}

      <div
        ref={rightRef}
        className={cn(
          'flex min-w-[360px] flex-1 flex-col',
          ocrPunch && cn(paneChrome, 'rounded-r-lg'),
          ocrPunch && rightTaller && !showOrig && 'rounded-bl-lg',
          origPunch && cn(paneChrome, 'rounded-r-lg rounded-tl-lg'),
        )}
        style={ocrPunch && shotH > 0 ? { minHeight: shotH } : undefined}
      >
        {ocrPending ? (
          <>
            <div className="shrink-0 overflow-y-auto px-4 py-2" style={topBoxStyle} />
            <Separator className="shrink-0 bg-foreground/20" />
            <div className="shrink-0 overflow-y-auto px-4 py-2" style={botBoxStyle} />
          </>
        ) : (
          <>
            {waitInput ? (
              <div
                ref={topSecRef}
                className="shrink-0 overflow-y-auto px-4 py-2"
                style={
                  paneLock
                    ? { height: paneLock.top, minHeight: paneLock.top, maxHeight: paneLock.top }
                    : topBoxStyle
                }
              >
                {text.trim() ? (
                  <p className="text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text">
                    {text}
                  </p>
                ) : null}
                <div className="flex h-6 items-center">
                  <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label="处理中" />
                </div>
              </div>
            ) : hasLeft ? (
              <p
                ref={topSecRef}
                className="shrink-0 overflow-y-auto px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text"
                style={
                  paneLock
                    ? { height: paneLock.top, minHeight: paneLock.top, maxHeight: paneLock.top }
                    : english.trim() || (sending && !waitInput)
                      ? topBoxStyle
                      : soloBoxStyle
                }
              >
                {text.trim() ? text : '\u00a0'}
              </p>
            ) : (
              <textarea
                ref={inputRef}
                value={text}
                rows={1}
                disabled={sending}
                className={cn(
                  areaClass,
                  'shrink-0',
                  ocrPreview?.src ? 'rounded-tr-lg' : 'rounded-t-lg',
                  ocrPunch && 'max-h-none',
                )}
                style={
                  paneLock
                    ? { height: paneLock.top, minHeight: paneLock.top, maxHeight: paneLock.top }
                    : ocrPunch
                      ? english.trim() || sending
                        ? { minHeight: colMinTop, maxHeight: colMaxTop }
                        : { minHeight: colMinSolo, maxHeight: colMaxSolo }
                      : undefined
                }
                onChange={(e: ChangeEvent<HTMLTextAreaElement>) => {
                  setText(e.target.value)
                  const two = !!(english.trim() || sending)
                  autosize(
                    e.currentTarget,
                    ocrPunch ? (two ? colMinTop : colMinSolo) : 0,
                    ocrPunch ? (two ? colMaxTop : colMaxSolo) : INPUT_MAX_H,
                  )
                  setEnglish('')
                  setOriginal(null)
                  setResult(null)
                  setDict(null)
                }}
                onKeyDown={(e: KeyboardEvent<HTMLTextAreaElement>) => {
                  if (e.nativeEvent.isComposing) return
                  if (e.key !== 'Enter') return
                  e.preventDefault()
                  // 已有英语结果：失焦后由全局 Enter 回填，避免再次执行
                  if (english.trim()) {
                    e.currentTarget.blur()
                    return
                  }
                  e.currentTarget.blur()
                  void run()
                }}
              />
            )}

            {english.trim() || (sending && !waitInput) ? (
              <>
                <Separator className="shrink-0 bg-foreground/20" />
                <div
                  ref={botSecRef}
                  className="flex shrink-0 items-start gap-1 overflow-y-auto px-4 py-2"
                  style={
                    paneLock && paneLock.bot > 0
                      ? { height: paneLock.bot, minHeight: paneLock.bot, maxHeight: paneLock.bot }
                      : botBoxStyle
                  }
                >
                  <div className="min-w-0 flex-1">
                    {english.trim() ? (
                      <p className="text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] select-text">
                        {diffParts
                          ? diffParts
                              .filter((p) => p.t !== 'del')
                              .map((p, i) => {
                                const body = p.s.split(/([A-Za-z]+(?:['’-][A-Za-z]+)*)/).map((part, j) =>
                                  isEnglishWord(part) ? (
                                    <span
                                      key={j}
                                      className="cursor-pointer hover:opacity-70"
                                      onClick={(ev) => {
                                        ev.stopPropagation()
                                        if (!inTauri()) return
                                        void dictLookup(part)
                                          .then((entry) =>
                                            setDict(entry ?? { word: part, senses: [] }),
                                          )
                                          .catch(() => {})
                                      }}
                                    >
                                      {part}
                                    </span>
                                  ) : (
                                    <span key={j}>{part}</span>
                                  ),
                                )
                                return p.t === 'add' ? (
                                  <span
                                    key={i}
                                    className="rounded-sm bg-green-500/20 text-green-800 dark:text-green-400"
                                  >
                                    {body}
                                  </span>
                                ) : (
                                  <span key={i}>{body}</span>
                                )
                              })
                          : english.split('\n').map((line, li) => (
                              <span key={li}>
                                {li > 0 ? '\n' : null}
                                {line.length === 0
                                  ? '\u00a0'
                                  : line.split(/([A-Za-z]+(?:['’-][A-Za-z]+)*)/).map((part, i) =>
                                      isEnglishWord(part) ? (
                                        <span
                                          key={i}
                                          className="cursor-pointer hover:text-foreground/70"
                                          onClick={(ev) => {
                                            ev.stopPropagation()
                                            if (!inTauri()) return
                                            void dictLookup(part)
                                              .then((entry) =>
                                                setDict(entry ?? { word: part, senses: [] }),
                                              )
                                              .catch(() => {})
                                          }}
                                        >
                                          {part}
                                        </span>
                                      ) : (
                                        <span key={i}>{part}</span>
                                      ),
                                    )}
                              </span>
                            ))}
                      </p>
                    ) : null}
                    {sending && !waitInput ? (
                      <div className="flex h-6 items-center">
                        <Loader2
                          className="size-4 animate-spin text-muted-foreground"
                          aria-label="处理中"
                        />
                      </div>
                    ) : null}
                  </div>
                  {english.trim() ? (
                    <button
                      type="button"
                      className="mt-0.5 inline-flex size-6 shrink-0 items-center justify-center rounded text-muted-foreground hover:text-foreground"
                      aria-label="朗读"
                      onClick={(ev) => {
                        ev.stopPropagation()
                        if (!inTauri()) return
                        void dictSpeaking()
                          .catch(() => false)
                          .then((on) => (on ? dictStop() : dictSpeak(english, language)))
                          .catch(() => {})
                      }}
                    >
                      <Volume2 className="size-3.5" />
                    </button>
                  ) : null}
                </div>
              </>
            ) : null}
          </>
        )}

        {error ? (
          <>
            <Separator className="shrink-0 bg-foreground/20" />
            <p className="shrink-0 px-4 py-2 text-sm break-words [overflow-wrap:anywhere] text-destructive">
              {error}
            </p>
          </>
        ) : null}

        {dict ? (
          <>
            <Separator className="shrink-0 bg-foreground/20" />
            <div className="shrink-0">
              <DictCard entry={dict} speakLang={language} />
            </div>
          </>
        ) : null}
      </div>
    </div>
    </div>
  )
}
