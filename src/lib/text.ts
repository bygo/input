export interface DiffPart { t: 'eq' | 'del' | 'add', s: string }

export function isEnglishWord(raw: string) {
  const t = raw.trim()
  if (!t || t.length > 64) {
    return false
  }
  return /^[A-Z]+(?:['’-][A-Z]+)*$/i.test(t)
}

/** 几乎全是拉丁字母才算纯英文；夹汉字等一律算非英文。 */
export function looksEnglish(raw: string) {
  const letters = raw.replace(
    /[^A-Za-z\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF\u0400-\u04FF\u0600-\u06FF]/g,
    '',
  )
  if (!letters) {
    return false
  }
  return /^[A-Z]+$/i.test(letters)
}

/** 词级 LCS diff：eq / del / add */
export function wordDiff(a: string, b: string): DiffPart[] {
  const A = a.split(/(\s+)/).filter(x => x.length > 0)
  const B = b.split(/(\s+)/).filter(x => x.length > 0)
  const n = A.length
  const m = B.length
  const dp: number[][] = Array.from({ length: n + 1 }, () => Array.from({ length: m + 1 }).fill(0) as number[])
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1])
    }
  }
  const out: DiffPart[] = []
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
