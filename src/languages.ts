export const LANGUAGES = [
  '英语',
  '中文',
  '日语',
  '韩语',
  '法语',
  '德语',
  '西班牙语',
  '葡萄牙语',
  '意大利语',
  '俄语',
  '阿拉伯语',
  '越南语',
]

const LETTER = /\p{L}/u
const HAN = /\p{Script=Han}/u
const KANA = /\p{Script=Hiragana}|\p{Script=Katakana}/u
const HANGUL = /\p{Script=Hangul}/u
const CYRILLIC = /\p{Script=Cyrillic}/u
const ARABIC = /\p{Script=Arabic}/u
const LATIN = /\p{Script=Latin}/u
const VIET = /[ăâêôơưđáàảãạắằẳẵặấầẩẫậéèẻẽẹếềểễệíìỉĩịóòỏõọốồổỗộớờởỡợúùủũụứừửữựýỳỷỹỵ]/i
const FR_MARK = /[àâæçéèêëîïôœùûüÿ]/i
const DE_MARK = /[äöüß]/i
const ES_MARK = /[ñ¿¡áéíóúü]/i
const PT_MARK = /[ãõáàâéêíóôúç]/i

const WORDS: Record<string, string[]> = {
  英语: ['the', 'and', 'of', 'to', 'in', 'is', 'it', 'you', 'that', 'for', 'on', 'are', 'was', 'this', 'with', 'be', 'as', 'have', 'not', 'but', 'they', 'from', 'we', 'or', 'an'],
  法语: ['le', 'la', 'les', 'des', 'une', 'est', 'que', 'qui', 'dans', 'pour', 'pas', 'avec', 'sur', 'je', 'tu', 'nous', 'vous', 'bonjour', 'merci', 'oui', 'non'],
  德语: ['der', 'die', 'das', 'und', 'ist', 'nicht', 'ein', 'eine', 'ich', 'sie', 'den', 'dem', 'auf', 'mit', 'zu', 'von', 'ein', 'auch', 'und'],
  西班牙语: ['el', 'los', 'las', 'una', 'que', 'de', 'en', 'es', 'por', 'con', 'para', 'como', 'no', 'un', 'se', 'del', 'hola', 'gracias'],
  葡萄牙语: ['uma', 'não', 'para', 'com', 'os', 'as', 'que', 'de', 'em', 'por', 'uma', 'é', 'do', 'da', 'não', 'olá', 'obrigado'],
  意大利语: ['il', 'lo', 'la', 'gli', 'che', 'non', 'per', 'una', 'con', 'sono', 'del', 'della', 'è', 'di', 'ciao', 'grazie'],
}

function letters(text: string) {
  return [...text].filter((ch) => LETTER.test(ch))
}

function ratio(chars: string[], re: RegExp) {
  if (!chars.length) return 0
  let n = 0
  for (const ch of chars) if (re.test(ch)) n += 1
  return n / chars.length
}

function hits(text: string, words: string[]) {
  const tokens = text.toLowerCase().match(/[\p{L}’']+/gu) ?? []
  const set = new Set(words)
  let n = 0
  for (const t of tokens) if (set.has(t)) n += 1
  return n
}

function guessLanguage(text: string): string | null {
  const chars = letters(text)
  if (!chars.length) return null
  const han = ratio(chars, HAN)
  const kana = ratio(chars, KANA)
  const hangul = ratio(chars, HANGUL)
  const cyr = ratio(chars, CYRILLIC)
  const ar = ratio(chars, ARABIC)
  const latin = ratio(chars, LATIN)
  if (hangul >= 0.3) return '韩语'
  if (kana >= 0.08) return '日语'
  if (han >= 0.35 && kana < 0.08 && hangul < 0.08) return '中文'
  if (ar >= 0.35) return '阿拉伯语'
  if (cyr >= 0.35) return '俄语'
  if (latin < 0.5) return null
  if (VIET.test(text)) return '越南语'
  const scores: [string, number][] = [
    ['英语', hits(text, WORDS.英语) * 2 + (FR_MARK.test(text) || DE_MARK.test(text) || ES_MARK.test(text) || PT_MARK.test(text) ? 0 : 1)],
    ['法语', hits(text, WORDS.法语) * 2 + (FR_MARK.test(text) ? 2 : 0)],
    ['德语', hits(text, WORDS.德语) * 2 + (DE_MARK.test(text) ? 2 : 0)],
    ['西班牙语', hits(text, WORDS.西班牙语) * 2 + (ES_MARK.test(text) ? 2 : 0)],
    ['葡萄牙语', hits(text, WORDS.葡萄牙语) * 2 + (PT_MARK.test(text) ? 2 : 0)],
    ['意大利语', hits(text, WORDS.意大利语) * 2],
  ]
  scores.sort((a, b) => b[1] - a[1])
  const [best, score] = scores[0]
  const second = scores[1][1]
  if (score >= 2 && score > second) return best
  if (latin >= 0.7 && score >= 1 && !FR_MARK.test(text) && !DE_MARK.test(text) && !ES_MARK.test(text) && !PT_MARK.test(text)) {
    return '英语'
  }
  return null
}

export function looksLikeLanguage(text: string, lang: string) {
  const g = guessLanguage(text)
  return g != null && g === lang
}

/** 与 Rust is_pure_english 一致：纯英文→母语，其它→英语。 */
export function isPureEnglish(text: string) {
  const letters = text.replace(
    /[^A-Za-z\u4e00-\u9fff\u3040-\u30ff\uac00-\ud7af\u0400-\u04ff\u0600-\u06ff]/g,
    '',
  )
  if (!letters) return false
  return /^[A-Za-z]+$/.test(letters)
}

export function selectTranslateInto(text: string, native: string, target: string) {
  return isPureEnglish(text) ? native : target
}
