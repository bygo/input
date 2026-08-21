const isMac =
  typeof navigator !== 'undefined' && /mac/i.test(navigator.platform || navigator.userAgent)

const MOD_LABEL: Record<string, string> = isMac
  ? {
      commandorcontrol: '⌘',
      commandorctrl: '⌘',
      cmdorctrl: '⌘',
      cmdorcontrol: '⌘',
      command: '⌘',
      cmd: '⌘',
      super: '⌘',
      control: '⌃',
      ctrl: '⌃',
      alt: '⌥',
      option: '⌥',
      shift: '⇧',
    }
  : {
      commandorcontrol: 'Ctrl',
      commandorctrl: 'Ctrl',
      cmdorctrl: 'Ctrl',
      cmdorcontrol: 'Ctrl',
      command: 'Win',
      cmd: 'Win',
      super: 'Win',
      control: 'Ctrl',
      ctrl: 'Ctrl',
      alt: 'Alt',
      option: 'Alt',
      shift: 'Shift',
    }

const MOD_CODES = new Set([
  'MetaLeft',
  'MetaRight',
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'ShiftLeft',
  'ShiftRight',
])

function keyLabel(token: string) {
  const t = token.replace(/^(Key|Digit)/i, '')
  if (/^arrow(up|down|left|right)$/i.test(t)) {
    return { up: '↑', down: '↓', left: '←', right: '→' }[t.slice(5).toLowerCase()] ?? t
  }
  if (/^[a-z]$/i.test(t)) return t.toUpperCase()
  if (t.toLowerCase() === 'space') return 'Space'
  if (t.toLowerCase() === 'escape') return 'Esc'
  if (t.toLowerCase() === 'enter') return 'Enter'
  return t.length === 1 ? t.toUpperCase() : t
}

export function formatShortcut(raw: string) {
  const parts = raw
    .split('+')
    .map((p) => p.trim())
    .filter(Boolean)
  if (!parts.length) return ''
  const shown = parts.map((p) => MOD_LABEL[p.toLowerCase()] ?? keyLabel(p))
  return isMac ? shown.join('') : shown.join('+')
}

export function codeToken(code: string): string | null {
  if (!code || MOD_CODES.has(code)) return null
  if (/^Key[A-Z]$/.test(code)) return code.slice(3)
  if (/^Digit[0-9]$/.test(code)) return code.slice(5)
  return code
}

function hasAlt(e: {
  altKey: boolean
  metaKey: boolean
  ctrlKey: boolean
  key: string
  code: string
  getModifierState?: (key: string) => boolean
}) {
  if (e.altKey) return true
  if (e.getModifierState?.('Alt') || e.getModifierState?.('AltGraph')) return true
  if (e.metaKey || e.ctrlKey) return false
  const token = codeToken(e.code)
  if (!token || !/^[A-Z]$/i.test(token)) return false
  if (e.key === 'Dead' || e.key === 'Process') return true
  return e.key.length === 1 && e.key.toLowerCase() !== token.toLowerCase()
}

export function shortcutFromEvent(e: {
  code: string
  key: string
  metaKey: boolean
  ctrlKey: boolean
  altKey: boolean
  shiftKey: boolean
  getModifierState?: (key: string) => boolean
}): string | null {
  const token = codeToken(e.code)
  if (!token) return null
  const fn = /^F([1-9]|1[0-9]|2[0-4])$/.test(token)
  const parts: string[] = []
  if (e.metaKey) parts.push('Command')
  if (e.ctrlKey) parts.push('Control')
  if (hasAlt(e)) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  if (!parts.length && !fn) return null
  parts.push(token)
  return parts.join('+')
}
