export const INPUT_MAX_H = 240
export const LINE_H = 40

export function autosize(el: HTMLTextAreaElement | null, minH = 0, maxH = INPUT_MAX_H) {
  if (!el) {
    return
  }
  el.style.height = 'auto'
  const h = Math.max(minH, el.scrollHeight)
  el.style.height = `${maxH < Number.POSITIVE_INFINITY ? Math.min(h, maxH) : h}px`
}

export const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export async function copyText(text: string) {
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
