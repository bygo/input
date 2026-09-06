/** R8: 合并 R6/R7 优点；再说严禁 chat；条件→once；拉群/邮件已修好的构式保留 */
export function selectPromptEn() {
  const mix = `
- 整句纯英文：禁止任何汉字残留
- 「再说」=搁置/先做再定/递进，永远不是 chat/talk/say：
  ①时间/条件+再说或「再说吧」→ leave it until/for [时间] / once [条件] / after…；必须保留修饰；禁止 chat later；禁止 talk about；禁止光秃 Let's see
  ②动作+再说 → … first / after doing…
  ③句首「再说」+人称/评价 → Besides / Anyway
- 仅当原文含「聊」时，收尾才用 chat / catch up later
- 观望：wait and see，禁止 Watch this
- 知会：「说一声」→ let me know when…（带条件）
- 打住提及：never mind，禁止 You know it
- 交权：up to you / you decide，禁止 make it up
- 拉群商量：start a group chat（不要句末 say it）
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
      '\n注意：「再说」是搁置/递进，禁止 chat/talk about；须 leave it until/for… 或 once… 并保留时间条件；动作+再说=… first；句首再说=Besides/Anyway。'
  } else if (/聊/.test(text)) {
    hint = '\n注意：含「聊」的收尾用 chat/catch up later。'
  }
  return `目标语言：英语。把【原文】译成地道英语（口语习语用意译，禁止字面直译）。
禁止添加原文未写的对象/关系。
多行须全部翻译并保留换行。${hint}
【原文】
${text}`
}
