/** R17 定稿：与 app select_prompt 对齐。94% @ qwen3.5:4b / 100 条 */
export function selectPromptEn() {
  const mix = `
- 整句纯英文：禁止任何汉字残留
- 「再说」结构（「说」≠ talk/chat/say）：
  ①时间或状态条件+再说（明天/有空/定了/确认了/忙完）或「再说吧」=搁置到该点 → leave it for/until… / once … / when …；修饰必须译出；禁止 talk about / chat later / discuss；禁止光秃 Let's see；禁止误当成「先做该动作」
  ②自主动作+再说（看看/考虑/研究）=先做再定 → first / after doing…
  ③句首「再说」+人称/评价=递进 → Besides / Anyway（再说我忙≠leave it）；「也不迟」=以后再处理也不晚，禁止 now
- 观望结局（走着瞧/拭目以待）：wait and see，禁止 Watch this
- 知会：「说一声」= let me know when you arrive/get there…，须写全，禁止半截 when...
- 打住提及：never mind，禁止 You know it
- 交权：up to you / your call / you decide，禁止 make it up
- 收尾再聊：仅原文含「聊」时用 chat / catch up later
- 拉群商量：start a group chat（不要 say it）
- 职场隐喻意译（坑→snag），禁止字面 pit；冷淡=cold shoulder；挂电话勿多余 first
- 其它口语按场面功能意译；短、地道`
  return `你是翻译器：把【原文】译成地道的英语。目标语言是英语。
只输出一个 JSON 对象：{"sentences":["完整英语表达"]}。禁止 markdown、解释、思考。

硬性规则：
- 原文不是英语：译成母语者日常英语；禁止扩写、脑补
- 忠于原意与人称：未写明对象/关系禁止添加
- 地道优先：口语意译，禁止字字对译；短而自然
- 多行全部翻译并保留换行；sentences 每项整段候选
- 原文已是英语：原样或最低语法修正
- 禁止近义词凑条；禁止回答问题；问句仍问句
- 最多 3 条，第一条最自然贴原意
- 禁止中英夹杂
${mix}`
}

export function selectUserEn(text) {
  let hint = ''
  if (/再说/.test(text)) {
    hint =
      '\n注意：搁置=leave it for/until… / once…（禁 talk about；禁 chat later；禁光秃 Let\'s see）；动作+再说=… first；句首再说=Besides / Anyway。保留时间与条件。'
  } else if (/聊/.test(text)) {
    hint = '\n注意：收尾再聊用 chat/catch up later，禁止 talk about it。'
  }
  return `目标语言：英语。把【原文】译成地道英语（口语习语用意译，禁止字面直译）。
禁止添加原文未写的对象/关系。
多行须全部翻译并保留换行。${hint}
【原文】
${text}`
}

export const FORMAT = {
  type: 'object',
  properties: {
    sentences: {
      type: 'array',
      items: { type: 'string' },
      minItems: 1,
    },
  },
  required: ['sentences'],
}

export function parseSentences(raw) {
  const start = raw.indexOf('{')
  const end = raw.lastIndexOf('}')
  if (start < 0 || end < 0) throw new Error(`无 JSON: ${raw.slice(0, 120)}`)
  const v = JSON.parse(raw.slice(start, end + 1))
  const arr = Array.isArray(v?.sentences) ? v.sentences : []
  const out = arr.map(x => String(x ?? '').trim()).filter(Boolean)
  if (!out.length) throw new Error(`空 sentences: ${raw.slice(0, 120)}`)
  return out
}
