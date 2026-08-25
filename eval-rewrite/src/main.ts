import './style.css'

type Row = {
  original: string
  rewrite: string
  seed: number | null
  status: '' | 'pass' | 'fail' | 'running' | 'error'
  error: string
}

type DiffPart = { t: 'eq' | 'del' | 'add'; s: string }

const MODELS = [
  { id: 'qwen3:4b-instruct', label: '千问' },
  { id: 'translategemma:4b', label: 'TranslateGemma' },
] as const
/** 生成 case / 效果分析用 */
const GEN_MODEL = 'qwen3:4b-instruct'
const FONT_SIZE = 13
const ROW_H = Math.round(FONT_SIZE * 1.5) + 7

document.documentElement.style.setProperty('--font-size', `${FONT_SIZE}px`)
document.documentElement.style.setProperty('--row-h', `${ROW_H}px`)

const rows: Row[] = []
let busy = false
let rewriteModel: string = MODELS[0].id
let analysisText = ''
const root = document.querySelector('#app')!

function usedSeeds() {
  const s = new Set<number>()
  for (const r of rows) {
    if (r.seed != null) s.add(r.seed)
  }
  return s
}

function allocSeed(used: Set<number>) {
  let s = (Math.random() * 0x7fffffff) | 0
  for (let i = 0; i < 100000; i++) {
    const next = (s + i) | 0
    if (!used.has(next)) {
      used.add(next)
      return next
    }
  }
  throw new Error('无可用种子')
}

function wordDiff(a: string, b: string): DiffPart[] {
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

function diffHtml(a: string, b: string, side: 'left' | 'right') {
  if (!a && !b) return ''
  if (!b) return escapeHtml(a)
  if (!a) return escapeHtml(b)
  const parts = wordDiff(a, b)
  if (side === 'left') {
    return parts
      .filter((p) => p.t !== 'add')
      .map((p) =>
        p.t === 'del' ? `<span class="del">${escapeHtml(p.s)}</span>` : escapeHtml(p.s),
      )
      .join('')
  }
  return parts
    .filter((p) => p.t !== 'del')
    .map((p) =>
      p.t === 'add' ? `<span class="add">${escapeHtml(p.s)}</span>` : escapeHtml(p.s),
    )
    .join('')
}

function stats() {
  const judged = rows.filter((r) => r.status === 'pass' || r.status === 'fail')
  const pass = rows.filter((r) => r.status === 'pass').length
  const seeded = rows.filter((r) => r.seed != null).length
  const base = judged.length
    ? `已评 ${judged.length}/${rows.length} · 通过 ${pass} · ${(
        (pass / judged.length) *
        100
      ).toFixed(0)}%`
    : `共 ${rows.length} 条`
  return `${base} · 种子 ${seeded}`
}

async function ollamaChat(
  model: string,
  prompt: string,
  options: Record<string, unknown>,
) {
  const resp = await fetch('/ollama/api/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model,
      stream: false,
      options,
      messages: [{ role: 'user', content: prompt }],
    }),
  })
  const body = await resp.text()
  if (!resp.ok) throw new Error(`${resp.status}: ${body.slice(0, 200)}`)
  const v = JSON.parse(body) as { message?: { content?: string } }
  const out = (v.message?.content || '').trim()
  if (!out) throw new Error('模型返回为空')
  return out
}

async function rewriteOne(text: string, into: string) {
  const toEn = into.includes('英语') || into.includes('英文')
  const prompt = toEn
    ? `You are fixing an on-screen English draft.\nFix only clear grammar mistakes. Keep the same meaning, person (I/you/me/my/your), tense, and structure.\nDo NOT swap I/you or me/you. Do NOT answer, chat, or paraphrase for style.\nIf already fine, return it unchanged (fix capitalization/punctuation only if needed).\nOutput only the corrected English, nothing else.\n\n【原文】\n${text}`
    : `把下面原文改成正确的${into}。只修语法，保留原意与人称，禁止对调 I/you。不要回答或扩写。只输出一句${into}，不要解释。\n\n【原文】\n${text}`
  return ollamaChat(rewriteModel, prompt, { temperature: 0.1 })
}

function localMetrics() {
  const paired = rows.filter((r) => r.original.trim() && r.rewrite.trim())
  const judged = rows.filter((r) => r.status === 'pass' || r.status === 'fail')
  const pass = rows.filter((r) => r.status === 'pass').length
  const fail = rows.filter((r) => r.status === 'fail').length
  const unchanged = paired.filter((r) => r.original.trim() === r.rewrite.trim()).length
  const pronounFlip = paired.filter((r) => {
    const a = r.original.toLowerCase()
    const b = r.rewrite.toLowerCase()
    const hasI = /\bi\b/.test(a)
    const hasYou = /\byou\b/.test(a)
    if (hasI && hasYou) return false
    if (hasI && !/\bi\b/.test(b) && /\byou\b/.test(b)) return true
    if (hasYou && !/\byou\b/.test(b) && /\bi\b/.test(b)) return true
    if (/\bme\b/.test(a) && !/\bme\b/.test(b) && /\byou\b/.test(b)) return true
    return false
  }).length
  return {
    model: rewriteModel,
    total: rows.length,
    paired: paired.length,
    judged: judged.length,
    pass,
    fail,
    passRate: judged.length ? pass / judged.length : null,
    unchanged,
    pronounFlip,
  }
}

async function analyzeModel() {
  const m = localMetrics()
  const samples = rows
    .filter((r) => r.original.trim() && r.rewrite.trim())
    .slice(0, 40)
    .map((r, i) => ({
      i: i + 1,
      seed: r.seed,
      status: r.status || null,
      original: r.original,
      rewrite: r.rewrite,
    }))
  if (samples.length === 0) throw new Error('请先跑出至少一条「原文+改写」')

  const prompt = `你是本地改写模型评测分析师。根据指标与样本，用中文写一份简洁分析报告。
改写模型: ${m.model}
指标 JSON: ${JSON.stringify(m)}
样本(最多40条) JSON: ${JSON.stringify(samples)}

请输出：
1) 总体结论（是否可用、主要风险）
2) 量化解读（通过率、未改动比例、疑似人称对调等）
3) 典型好例 / 坏例各 1～3 条（引用原文→改写）
4) 错误模式归类（语法微修、语义漂移、人称对调、过度改写等）
5) 可执行改进建议（prompt / 温度 / 后处理校验）
不要编造样本里没有的句子。`

  return ollamaChat(GEN_MODEL, prompt, {
    temperature: 0.3,
    num_predict: 2048,
  })
}

async function generateCases(n: number, into: string) {
  const used = usedSeeds()
  const seeds: number[] = []
  for (let i = 0; i < n; i++) seeds.push(allocSeed(used))

  const toEn = into.includes('英语') || into.includes('英文')
  const existing = rows
    .map((r) => r.original.trim())
    .filter(Boolean)
    .slice(-12)
  const avoidLine = existing.length ? existing.join(' | ') : '(none)'
  const prompt = toEn
    ? `You generate eval cases for an English grammar-fix IME.\nBatch seeds: [${seeds.join(', ')}]. Use them only as diversity entropy; each sentence should feel different.\nProduce EXACTLY ${n} short English drafts a non-native might type (grammar errors welcome: agreement, tense, articles, I/you mix-ups, missing words).\nVary hard across the batch: statements / questions / requests / offers; topics (work email, PR comment, chat, travel, food, school); mistake types.\nDo NOT resemble existing cases: ${avoidLine}\nOutput ONLY a JSON array of ${n} strings. No markdown, no keys, no commentary.`
    : `你在为「改写成${into}」评测造用例。\n本批种子: [${seeds.join(', ')}]。只当多样性熵，每句要明显不同。\n生成恰好 ${n} 条短草稿（可含语病）。\n强制拉开：陈述/疑问/请求/提议；话题与错误类型都要多样。\n不要与已有用例相似：${avoidLine}\n只输出 JSON 字符串数组，长度 ${n}。不要 markdown，不要解释。`

  const raw = await ollamaChat(GEN_MODEL, prompt, {
    temperature: 1.1,
    seed: seeds[0],
    num_predict: Math.min(4096, 100 * n + 300),
  })
  const start = raw.indexOf('[')
  const end = raw.lastIndexOf(']')
  if (start < 0 || end < 0) throw new Error(`无法解析用例 JSON: ${raw.slice(0, 160)}`)
  const arr = JSON.parse(raw.slice(start, end + 1)) as unknown
  if (!Array.isArray(arr)) throw new Error('用例不是数组')
  const list = arr
    .map((x) => String(x ?? '').trim())
    .filter((x) => x.length > 0)
    .slice(0, n)
  if (list.length === 0) throw new Error('未生成到有效用例')

  const out: { original: string; seed: number }[] = []
  for (let i = 0; i < list.length; i++) {
    out.push({ original: list[i], seed: seeds[i] })
  }
  // 数量不足时，剩余已占用的种子仍逐条补齐，避免丢种子记账
  const avoid = [...existing, ...list]
  for (let i = list.length; i < n; i++) {
    const one = await ollamaChat(
      GEN_MODEL,
      toEn
        ? `Seed: ${seeds[i]}. Write ONE short flawed English draft for grammar-fix eval. Different from: ${avoid.slice(-5).join(' | ')}. Output only the sentence.`
        : `种子 ${seeds[i]}。写 1 条评测用短${into}草稿（可有语病）。不要与这些重复：${avoid.slice(-5).join(' | ')}。只输出句子。`,
      { temperature: 1.15, seed: seeds[i], num_predict: 120 },
    )
    const line = one
      .split('\n')[0]
      .replace(/^["'`\d.\)\-\s]+/, '')
      .replace(/["']$/g, '')
      .trim()
    if (!line) break
    out.push({ original: line, seed: seeds[i] })
    avoid.push(line)
  }
  return out
}

async function runAt(i: number, into: string) {
  const row = rows[i]
  row.status = 'running'
  row.error = ''
  row.rewrite = ''
  render()
  try {
    row.rewrite = await rewriteOne(row.original, into)
    row.status = ''
  } catch (e) {
    row.status = 'error'
    row.error = String(e)
  }
  render()
}

async function runAll(into: string) {
  if (busy) return
  busy = true
  render()
  for (let i = 0; i < rows.length; i++) await runAt(i, into)
  busy = false
  render()
}

function render() {
  const into =
    (document.querySelector('#into') as HTMLSelectElement | null)?.value || '英语'
  const addN =
    (document.querySelector('#addN') as HTMLInputElement | null)?.value || '10'
  root.innerHTML = `
    <div class="bar">
      <label>改写模型
        <select id="model">
          ${MODELS.map(
            (m) =>
              `<option value="${m.id}" ${rewriteModel === m.id ? 'selected' : ''}>${m.label}</option>`,
          ).join('')}
        </select>
      </label>
      <label>目标
        <select id="into">
          <option value="英语" ${into === '英语' ? 'selected' : ''}>英语</option>
          <option value="中文" ${into === '中文' ? 'selected' : ''}>中文</option>
        </select>
      </label>
      <button class="primary" id="runAll" ${busy ? 'disabled' : ''}>全部跑一遍</button>
      <button id="add" ${busy ? 'disabled' : ''}>加一行</button>
      <label>数量 <input type="number" id="addN" min="1" max="200" value="${escapeAttr(addN)}" style="width:56px" /></label>
      <button id="addMany" ${busy ? 'disabled' : ''}>生成 N 条</button>
      <button id="export" ${busy ? 'disabled' : ''}>导出 JSON</button>
      <button id="analyze" ${busy ? 'disabled' : ''}>分析效果</button>
      <span class="meta">${stats()}${busy ? ' · 运行中…' : ''}</span>
    </div>
    ${
      analysisText
        ? `<div class="report"><div class="report-bar"><b>效果分析</b> <button id="closeReport">关闭</button></div><pre>${escapeHtml(analysisText)}</pre></div>`
        : ''
    }
    <div class="table">
      <div class="head">
        <div class="rn">#</div>
        <div class="seedh">种子</div>
        <div class="hcell left">原文</div>
        <div class="hcell">改写</div>
        <div class="act hcell" style="flex:none;width:var(--act-w)">判定</div>
      </div>
      ${rows
        .map((r, i) => {
          const left =
            r.status === 'running'
              ? escapeHtml(r.original)
              : diffHtml(r.original, r.rewrite, 'left')
          const right =
            r.status === 'running'
              ? '<span class="running">…</span>'
              : r.error
                ? `<span class="err">${escapeHtml(r.error)}</span>`
                : r.rewrite
                  ? diffHtml(r.original, r.rewrite, 'right')
                  : ''
          const cls =
            r.status === 'pass' ? 'pass' : r.status === 'fail' ? 'fail' : ''
          return `
        <div class="row ${cls}">
          <div class="rn">${i + 1}</div>
          <div class="seedc" title="seed">${r.seed ?? '—'}</div>
          <div class="cell left">
            <div class="inner" contenteditable="true" data-k="original" data-i="${i}" title="${escapeAttr(r.original)}">${left}</div>
          </div>
          <div class="cell">
            <div class="inner" contenteditable="true" data-k="rewrite" data-i="${i}" title="${escapeAttr(r.rewrite)}">${right}</div>
          </div>
          <div class="act">
            <button data-run="${i}" ${busy ? 'disabled' : ''}>跑</button>
            <button data-pass="${i}" class="ok${r.status === 'pass' ? ' on' : ''}" ${busy ? 'disabled' : ''}>✓</button>
            <button data-fail="${i}" class="bad${r.status === 'fail' ? ' on' : ''}" ${busy ? 'disabled' : ''}>✗</button>
          </div>
        </div>`
        })
        .join('')}
    </div>
  `

  document.querySelector('#model')?.addEventListener('change', () => {
    rewriteModel = (document.querySelector('#model') as HTMLSelectElement).value
  })
  document.querySelector('#runAll')?.addEventListener('click', () => {
    void runAll((document.querySelector('#into') as HTMLSelectElement).value)
  })
  document.querySelector('#add')?.addEventListener('click', () => {
    rows.push({ original: '', rewrite: '', seed: null, status: '', error: '' })
    render()
  })
  document.querySelector('#closeReport')?.addEventListener('click', () => {
    analysisText = ''
    render()
  })
  document.querySelector('#analyze')?.addEventListener('click', () => {
    if (busy) return
    busy = true
    analysisText = '分析中…'
    render()
    void analyzeModel()
      .then((text) => {
        analysisText = text
      })
      .catch((e) => {
        analysisText = `分析失败: ${String(e)}`
      })
      .finally(() => {
        busy = false
        render()
      })
  })
  document.querySelector('#addMany')?.addEventListener('click', () => {
    if (busy) return
    const n = Math.min(
      200,
      Math.max(1, Math.floor(Number((document.querySelector('#addN') as HTMLInputElement).value) || 1)),
    )
    const target = (document.querySelector('#into') as HTMLSelectElement).value
    busy = true
    render()
    void generateCases(n, target)
      .then((list) => {
        for (const { original, seed } of list) {
          rows.push({ original, rewrite: '', seed, status: '', error: '' })
        }
      })
      .catch((e) => {
        rows.push({
          original: '',
          rewrite: '',
          seed: null,
          status: 'error',
          error: `生成失败: ${String(e)}`,
        })
      })
      .finally(() => {
        busy = false
        render()
      })
  })
  document.querySelector('#export')?.addEventListener('click', () => {
    const data = rows
      .filter((r) => r.original.trim() || r.rewrite.trim())
      .map((r) => ({
        original: r.original,
        rewrite: r.rewrite,
        seed: r.seed,
        status: r.status || undefined,
      }))
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `eval-rewrite-${Date.now()}.json`
    a.click()
    URL.revokeObjectURL(url)
  })
  document.querySelector('#into')?.addEventListener('change', () => render())

  root.querySelectorAll('.inner').forEach((el) => {
    const d = el as HTMLElement
    d.addEventListener('focus', () => {
      const i = Number(d.dataset.i)
      const k = d.dataset.k as 'original' | 'rewrite'
      d.textContent = rows[i][k]
    })
    d.addEventListener('blur', () => {
      const i = Number(d.dataset.i)
      const k = d.dataset.k as 'original' | 'rewrite'
      const next = (d.innerText || '').replace(/\n$/, '')
      if (rows[i][k] === next) {
        render()
        return
      }
      rows[i][k] = next
      render()
    })
  })
  root.querySelectorAll('[data-run]').forEach((el) => {
    el.addEventListener('click', () => {
      void runAt(
        Number((el as HTMLElement).dataset.run),
        (document.querySelector('#into') as HTMLSelectElement).value,
      )
    })
  })
  root.querySelectorAll('[data-pass]').forEach((el) => {
    el.addEventListener('click', () => {
      rows[Number((el as HTMLElement).dataset.pass)].status = 'pass'
      render()
    })
  })
  root.querySelectorAll('[data-fail]').forEach((el) => {
    el.addEventListener('click', () => {
      rows[Number((el as HTMLElement).dataset.fail)].status = 'fail'
      render()
    })
  })
}

function escapeHtml(s: string) {
  return s
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function escapeAttr(s: string) {
  return escapeHtml(s).replaceAll('\n', ' ')
}

void fetch('/cases.json')
  .then((r) => r.json())
  .then((list: string[]) => {
    const used = new Set<number>()
    for (const original of list) {
      rows.push({
        original,
        rewrite: '',
        seed: allocSeed(used),
        status: '',
        error: '',
      })
    }
    render()
  })
  .catch(() => {
    render()
  })
