/** R7: R6 去 drop it 误伤；条件搁置强制 once/when；商量说≠标记说 */
export function selectPromptEn() {
  const mix = `
- 整句纯英文：禁止任何汉字残留
- 「再说」结构（「说」≠ talk/say）：
  ①时间/条件+再说或「再说吧」=搁置到该点 → leave it until/for… / after… / once… / when…；必须带上原时间或条件；禁止 talk about；禁止光秃 Let's see
  ②动作+再说=先做再定 → first / after doing…
  ③句首「再说」+人称/评价=递进 → Besides / Anyway
- 观望结局：wait and see，禁止 Watch this
- 知会：「说一声」= let me know when…，须带时间条件
- 打住提及：never mind，禁止 You know it
- 交权：up to you / you decide，禁止 make it up
- 收尾再聊（回头聊等）：chat / catch up later；与搁置「有空再说」不同——后者是 leave it until free，不是 chat
- 商量/拉群：start a group chat / discuss in the group，句末「说」不是 say it
- 职场隐喻意译（坑→snag）；短、地道`
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
      '\n注意：搁置必须带时间/条件（leave it until/for… / once… / when…）；禁 talk about；禁光秃 Let\'s see。动作+再说=… first；句首再说=Besides / Anyway。有空再说≠chat later。'
  } else if (/回头聊/.test(text)) {
    hint = '\n注意：收尾再聊用 chat/catch up later，禁止 talk about it。'
  }
  return `目标语言：英语。把【原文】译成地道英语（口语习语用意译，禁止字面直译）。
禁止添加原文未写的对象/关系。
多行须全部翻译并保留换行。${hint}
【原文】
${text}`
}
