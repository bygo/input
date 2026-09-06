#!/usr/bin/env bun
/** 启发式打分：标出必 ✗ 模式 + 输出待人工扫的可疑项。用法：bun score-literal.mjs report.json */
import { readFileSync } from 'node:fs'

const path = process.argv[2]
if (!path) {
  console.error('usage: bun score-literal.mjs <report.json>')
  process.exit(1)
}
const { rows } = JSON.parse(readFileSync(path, 'utf8'))
const labels = ['搁置', '决定权', '打住', '观望', '程度', '社交', '责问', '办事', '职场', '情感']

function grade(original, rewrite) {
  const r = rewrite.toLowerCase()
  // 必 ✗ 模式（原则违规，非单条 case）
  if (/talk about|speak about|say something|discuss it/.test(r) && /再说|回头聊|聊/.test(original)) {
    return 'x'
  }
  if (/make it up/.test(r)) return 'x'
  if (/you know it|you know how it is/.test(r) && /别提/.test(original)) return 'x'
  if (/^watch this\.?$/.test(r.trim())) return 'x'
  if (/^say it\.?$/.test(r.trim()) && /说一声/.test(original)) return 'x'
  if (/love who you want/.test(r)) return 'x'
  if (/let's see\.?$/.test(r.trim()) && /定了|确认了/.test(original)) return 'x'
  if (/to say\.?$/.test(r.trim()) && /拉个群/.test(original)) return 'x'
  // 可疑 △
  if (/talk about|speak about/.test(r)) return 't'
  if (/stumbled into a pit|hit another pit/.test(r)) return 't'
  if (/i'll see what i can do/.test(r) && /看看再说/.test(original)) return 't'
  return 'v'
}

const scores = rows.map(({ original, rewrite }) => grade(original, rewrite || ''))
const count = { v: 0, t: 0, x: 0 }
scores.forEach(s => count[s]++)
console.log(`heuristic ✓=${count.v} △=${count.t} ✗=${count.x}  ✓%≈${count.v}`)
for (let c = 0; c < 10; c++) {
  const chunk = scores.slice(c * 10, c * 10 + 10)
  const v = chunk.filter(s => s === 'v').length
  const bad = []
  chunk.forEach((s, j) => {
    if (s !== 'v') {
      const row = rows[c * 10 + j]
      bad.push(`${s}:${row.original}→${row.rewrite}`)
    }
  })
  console.log(`${labels[c]} ✓${v}/10 ${bad.join(' | ')}`)
}
