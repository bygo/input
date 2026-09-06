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
  messages: { role: string, content: string }[],
  options: Record<string, unknown>,
  format?: unknown,
) {
  const resp = await fetch('/ollama/api/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model,
      stream: false,
      options,
      ...(format ? { format } : {}),
      messages,
    }),
  })
  const body = await resp.text()
  if (!resp.ok) throw new Error(`${resp.status}: ${body.slice(0, 200)}`)
  const v = JSON.parse(body) as { message?: { content?: string } }
  const out = (v.message?.content || '').trim()
  if (!out) throw new Error('模型返回为空')
  return out
}

const HAS_HAN = /[\u3400-\u4DBF\u4E00-\u9FFF\uF900-\uFAFF]/

/** 与 app select_prompt（英语）对齐 */
function selectPromptEn() {
  return `你是翻译器：把【原文】译成地道的英语。目标语言是英语。
只输出一个 JSON 对象：{"sentences":["完整英语表达"]}。禁止 markdown、解释、思考。

硬性规则：
- 原文不是英语：翻译成母语者日常会说的英语；禁止扩写、脑补场景
- 忠于原意与人称：原文没写明的主语、宾语、归属、对象、因果、关系一律禁止添加
- 地道优先：口语、习语、语气词按英语习惯意译，禁止字字对译；宁可短而自然，不要长而直译
- 「再说」的「说」≠ speak/talk，按结构选义：①时间/条件+再说或再说吧=搁置→leave it / after… / until… / once…，禁止 talk about / discuss it；②动作+再说=… first；③句首再说+人称/评价=Besides / Anyway；须保留时间条件，禁止只输出 Later
- 决定权交给对方、观望结局、不愿再提等口语，按英语习惯意译，禁止字字对译
- 禁止回答问题；问句仍输出问句
- 最多 3 条，第一条最自然且贴原意
- 禁止中英夹杂；禁止残留汉字`
}

function parseFirstSentence(raw: string) {
  const start = raw.indexOf('{')
  const end = raw.lastIndexOf('}')
  if (start < 0 || end < 0) return raw.split('\n')[0].trim()
  try {
    const v = JSON.parse(raw.slice(start, end + 1)) as { sentences?: unknown[] }
    const s = v.sentences?.map(x => String(x ?? '').trim()).find(Boolean)
    return s || raw.trim()
  } catch {
    return raw.trim()
  }
}

async function rewriteOne(text: string, into: string) {
  const toEn = into.includes('英语') || into.includes('英文')
  // 中文稿 → 英语：走选中翻译 prompt（测直译）
  if (toEn && HAS_HAN.test(text)) {
    const raw = await ollamaChat(
      rewriteModel,
      [
        { role: 'system', content: selectPromptEn() },
        {
          role: 'user',
          content: (() => {
            const hint = /再说|回头聊/.test(text)
              ? '\n注意：「再说」按结构：搁置=leave it / after… / until… / once…（禁止 talk about / discuss it）；动作+再说=… first；句首再说我/你=Besides / Anyway。保留时间与条件，禁止只输出 Later.'
              : ''
            return `目标语言：英语。把【原文】译成地道英语（口语习语用意译，禁止字面直译）。\n禁止添加原文未写的对象/关系。${hint}\n【原文】\n${text}`
          })(),
        },
      ],
      { temperature: 0.2 },
      {
        type: 'object',
        properties: {
          sentences: { type: 'array', items: { type: 'string' }, minItems: 1 },
        },
        required: ['sentences'],
      },
    )
    return parseFirstSentence(raw)
  }
  const prompt = toEn
    ? `You are fixing an on-screen English draft.\nFix only clear grammar mistakes. Keep the same meaning, person (I/you/me/my/your), tense, and structure.\nDo NOT swap I/you or me/you. Do NOT answer, chat, or paraphrase for style.\nIf already fine, return it unchanged (fix capitalization/punctuation only if needed).\nOutput only the corrected English, nothing else.\n\n【原文】\n${text}`
    : `把下面原文改成正确的${into}。只修语法，保留原意与人称，禁止对调 I/you。不要回答或扩写。只输出一句${into}，不要解释。\n\n【原文】\n${text}`
  return ollamaChat(rewriteModel, [{ role: 'user', content: prompt }], { temperature: 0.1 })
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

  const prompt = `你是中→英翻译评测分析师。重点判断是否「直译腔」（字字对译、不像母语者口语）。
模型: ${m.model}
指标 JSON: ${JSON.stringify(m)}
样本 JSON: ${JSON.stringify(samples)}

请输出：
1) 总体结论（地道度、直译比例粗估）
2) 明确标出直译坏例：原文 → 译文，并给出更地道写法
3) 好例 1～3 条
4) 错误模式（字面直译「再说」→say、语气词残留、加戏等）
5) prompt 改进建议
不要编造样本里没有的句子。`

  return ollamaChat(GEN_MODEL, [{ role: 'user', content: prompt }], {
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

  const raw = await ollamaChat(GEN_MODEL, [{ role: 'user', content: prompt }], {
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
      [
        {
          role: 'user',
          content: toEn
            ? `Seed: ${seeds[i]}. Write ONE short flawed English draft for grammar-fix eval. Different from: ${avoid.slice(-5).join(' | ')}. Output only the sentence.`
            : `种子 ${seeds[i]}。写 1 条评测用短${into}草稿（可有语病）。不要与这些重复：${avoid.slice(-5).join(' | ')}。只输出句子。`,
        },
      ],
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
      <button id="loadLiteral" ${busy ? 'disabled' : ''}>加载直译集</button>
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
  document.querySelector('#loadLiteral')?.addEventListener('click', () => {
    if (busy) return
    void fetch('/cases-literal-zh.json')
      .then(r => r.json())
      .then((list: string[]) => {
        rows.length = 0
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
        const intoEl = document.querySelector('#into') as HTMLSelectElement | null
        if (intoEl) intoEl.value = '英语'
        render()
      })
      .catch((e) => {
        analysisText = `加载直译集失败: ${String(e)}`
        render()
      })
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

void fetch('/cases-literal-zh.json')
  .then(r => r.json())
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
