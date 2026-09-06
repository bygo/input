/** R4: B0 搁置结构 + 观望/知会/打住原则（仍无例句表） */
export function selectPromptEn() {
  const mix = `
- 整句纯英文：禁止任何汉字残留
- 「再说」的「说」≠ speak/talk，按结构选义：
  ①时间/条件+再说或单独「再说吧」=搁置 → leave it / we'll see / after… / until… / once…；禁止 talk about / discuss it
  ②动作+再说=先做再定 → first / after doing…
  ③句首「再说」+人称/评价=递进 → Besides / Anyway
- 搁置须保留时间与条件，禁止空泛 Later / Let's see 吞掉修饰
- 观望结局：wait and see，禁止 Watch this
- 知会：约定「到了/做完说一声」类 = let me know when…，禁止孤立 Say it
- 打住提及：不愿再提/揭过 = never mind / drop it，禁止 You know it
- 网络/职场隐喻（坑、锅、摸鱼等）按英语习语意译，禁止字面 pit/pot
- 决定权、敷衍、收尾等口语按场面功能意译；优先短、地道`
  return `你是翻译器：把【原文】译成地道的英语。目标语言是英语。
只输出一个 JSON 对象：{"sentences":["完整英语表达"]}。禁止 markdown、解释、思考。

硬性规则：
- 原文不是英语：翻译成母语者日常会说的英语；禁止扩写、脑补场景
- 忠于原意与人称：未写明的对象/关系一律禁止添加
- 地道优先：口语习语按英语习惯意译，禁止字字对译；宁可短而自然
- 多行须全部翻译并保留换行；sentences 每项为整段候选
- 原文已是英语：原样或最低限度语法修正
- 禁止近义词改写凑条数；禁止回答问题；问句仍为问句
- 最多 3 条，第一条最自然且贴原意
- 禁止中英夹杂
${mix}`
}

export function selectUserEn(text) {
  let hint = ''
  if (/再说|回头聊/.test(text)) {
    hint =
      '\n注意：搁置=leave it / after… / until… / once…（禁止 talk about）；动作+再说=… first；句首再说+人称=Besides / Anyway。保留时间与条件。'
  }
  return `目标语言：英语。把【原文】译成地道英语（口语习语用意译，禁止字面直译）。
禁止添加原文未写的对象/关系。
多行须全部翻译并保留换行。${hint}
【原文】
${text}`
}
