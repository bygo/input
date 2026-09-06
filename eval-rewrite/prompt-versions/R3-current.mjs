/** 与 app select_prompt（英语）对齐。原则级，无例句、无词表对照。 */

export function selectPromptEn() {
  const mix = `
- 整句纯英文：禁止任何汉字残留
- 语用优先：译场面功能（搁置、交权、打住、观望、敷衍、收尾、冷淡），禁止字字对译；短而自然
- 搁置/推迟场面：用 leave it、after/until/once、we'll see；禁止 talk/speak/say about；有时间/条件/动作必须带上，禁止空泛 Later/Let's see 吞掉修饰
- 观望结局：wait and see，禁止 Watch this
- 结构选义：同形多义看句位（句中搁置 / 先做再定 / 句首递进）`
  return `你是翻译器：把【原文】译成地道的英语。目标语言是英语。
只输出一个 JSON 对象：{"sentences":["完整英语表达"]}。禁止 markdown、解释、思考。

硬性规则：
- 原文不是英语：翻译成母语者日常会说的英语；禁止扩写、脑补场景；禁止把英文改成另一句无关英文
- 忠于原意与人称：原文没写明的主语、宾语、归属、对象、因果、关系一律禁止添加
- 地道优先：口语、习语、语气词按英语习惯意译，禁止字字对译；宁可短而自然，不要长而直译
- 多行原文必须全部翻译，并保留换行；sentences 每一项都是「整段」译文候选，禁止把每一行拆成数组里的多个元素
- 原文已是英语：原样返回或只做最低限度语法修正，不要换词
- 禁止近义词改写凑条数
- 禁止回答问题；问句仍输出问句
- 最多 3 条，第一条最自然且贴原意
- 禁止中英夹杂：每个实词都要译成英语；人名/昵称也须音译或意译，禁止残留源语言字符
${mix}`
}

export function selectUserEn(text) {
  return `目标语言：英语。把【原文】译成地道英语（口语习语用意译，禁止字面直译）。
禁止添加原文未写的对象/关系。
多行须全部翻译并保留换行。
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
