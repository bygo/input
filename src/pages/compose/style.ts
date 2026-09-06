import { INPUT_MAX_H, LINE_H } from '@/lib/dom'

export const areaClass
  = 'block min-h-10 w-full max-h-60 resize-none overflow-y-auto border-0 bg-transparent px-4 py-2 text-[15px] leading-6 break-words whitespace-pre-wrap [overflow-wrap:anywhere] outline-none disabled:cursor-not-allowed disabled:opacity-50'

export const paneStyle = { minHeight: LINE_H, maxHeight: INPUT_MAX_H } as const
