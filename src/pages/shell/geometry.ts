import type { Window } from '@tauri-apps/api/window'
import {
  currentMonitor,
  monitorFromPoint,
  primaryMonitor,
} from '@tauri-apps/api/window'

/** 窗口几何：OCR 选区与 ResizeObserver 共享。 */
export const sizeFloorRef = { current: null as { w: number, h: number } | null }
export const ocrPinRef = { current: null as { x: number, y: number } | null }
export const blockWindowResize = { current: false }

/** 钉点物理坐标收进所在屏，避免截图窗溢出。 */
export async function fitOcrPin(
  win: Window,
  x: number,
  y: number,
  logicalW: number,
  logicalH: number,
): Promise<{ x: number, y: number }> {
  const scale = await win.scaleFactor()
  const mon
    = (await monitorFromPoint(x, y))
      ?? (await currentMonitor())
      ?? (await primaryMonitor())
  if (!mon) {
    return { x, y }
  }
  const pad = 8
  const left = mon.position.x + pad
  const top = mon.position.y + pad
  const right = mon.position.x + mon.size.width - pad
  const bottom = mon.position.y + mon.size.height - pad
  const pw = logicalW * scale
  const ph = logicalH * scale
  const maxX = Math.max(left, right - pw)
  const maxY = Math.max(top, bottom - ph)
  return {
    x: Math.round(Math.min(Math.max(x, left), maxX)),
    y: Math.round(Math.min(Math.max(y, top), maxY)),
  }
}
