#!/usr/bin/env bun
/** 复测易直译失败子集 */
import { writeFileSync } from 'node:fs'
import { cpus } from 'node:os'
import { FORMAT, parseSentences, selectPromptEn, selectUserEn } from './select-prompt.mjs'

const OLLAMA = 'http://127.0.0.1:11434/api/chat'
const MODEL = 'qwen3:4b-instruct'
const NUM_THREAD = Number(process.env.NUM_THREAD) || Math.max(1, Math.floor((cpus().length || 4) / 2))
const cases = [
  '再说吧',
  '回头再说',
  '有空再说',
  '到时候再说',
  '以后再说',
  '改天再说',
  '下次再说',
  '等会儿再说',
  '忙完再说',
  '回头聊',
  '定了再说',
  '再说也不迟',
  '再说我走了',
  '吃完再说',
  '回去再说',
  '上车再说',
  '先这样，回头再说',
  '我再想想，明天再说',
]

async function chat(system, user) {
  const resp = await fetch(OLLAMA, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: MODEL,
      stream: false,
      format: FORMAT,
      options: { temperature: 0.2, num_thread: NUM_THREAD },
      messages: [
        { role: 'system', content: system },
        { role: 'user', content: user },
      ],
    }),
  })
  const body = await resp.text()
  if (!resp.ok) throw new Error(`${resp.status}: ${body.slice(0, 200)}`)
  const v = JSON.parse(body)
  return parseSentences((v.message?.content || '').trim())[0]
}

const system = selectPromptEn()
const rows = []
for (const original of cases) {
  process.stdout.write(`${original} … `)
  try {
    const rewrite = await chat(system, selectUserEn(original))
    rows.push({ original, rewrite })
    console.log(rewrite)
  } catch (e) {
    rows.push({ original, rewrite: '', error: String(e) })
    console.log('ERR', e)
  }
}
const out = new URL(`./report-literal-rerun-${Date.now()}.json`, import.meta.url)
writeFileSync(out, JSON.stringify({ model: MODEL, rows }, null, 2))
console.log(`\n写入 ${out.pathname}`)
