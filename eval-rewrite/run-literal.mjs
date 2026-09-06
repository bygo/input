#!/usr/bin/env bun
/**
 * 中→英口语直译测评：50 条易直译 case，对齐 app select_prompt。
 * 用法：bun run-literal.mjs
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { cpus } from 'node:os'
import { FORMAT, parseSentences, selectPromptEn, selectUserEn } from './select-prompt.mjs'

const OLLAMA = 'http://127.0.0.1:11434/api/chat'
const MODEL = 'qwen3.5:4b'
// 约一半 CPU，避免整机卡死（可用 NUM_THREAD 覆盖）
const NUM_THREAD = Number(process.env.NUM_THREAD) || Math.max(1, Math.floor((cpus().length || 4) / 2))
const cases = JSON.parse(readFileSync(new URL('./public/cases-literal-zh.json', import.meta.url), 'utf8'))

async function chat(system, user) {
  const resp = await fetch(OLLAMA, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: MODEL,
      stream: false,
      think: false,
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
  const raw = (v.message?.content || '').trim()
  if (!raw) throw new Error('empty')
  return parseSentences(raw)[0]
}

console.log(`model=${MODEL} num_thread=${NUM_THREAD}/${cpus().length}`)
const system = selectPromptEn()
const rows = []
for (let i = 0; i < cases.length; i++) {
  const original = cases[i]
  process.stdout.write(`[${i + 1}/${cases.length}] ${original} … `)
  try {
    const rewrite = await chat(system, selectUserEn(original))
    rows.push({ original, rewrite, error: null })
    console.log(rewrite)
  } catch (e) {
    rows.push({ original, rewrite: '', error: String(e) })
    console.log('ERR', e)
  }
}

const outPath = new URL(`./report-literal-${Date.now()}.json`, import.meta.url)
writeFileSync(outPath, JSON.stringify({ model: MODEL, into: '英语', rows }, null, 2))
console.log(`\n写入 ${outPath.pathname}`)
