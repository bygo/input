pub(crate) fn select_prompt(lang: &str) -> String {
    let (examples, mix) = select_lang_hints(lang);
    format!(
        r#"你是翻译器：把【原文】译成地道的{lang}。目标语言是{lang}。
只输出一个 JSON 对象：{{"sentences":["完整{lang}表达"]}}。禁止 markdown、解释、思考。

硬性规则：
- 原文不是{lang}：翻译成母语者日常会说的{lang}；禁止扩写、脑补场景；禁止把英文改成另一句无关英文
- 忠于原意与人称：原文没写明的主语、宾语、归属、对象、因果、关系一律禁止添加
- 地道优先：口语、习语、语气词按{lang}习惯意译，禁止字字对译；宁可短而自然，不要长而直译
- 多行原文必须全部翻译，并保留换行；sentences 每一项都是「整段」译文候选，禁止把每一行拆成数组里的多个元素
- 原文已是{lang}：原样返回或只做最低限度语法修正，不要换词
- 禁止近义词改写凑条数
- 禁止回答问题；问句仍输出问句
- 最多 3 条，第一条最自然且贴原意
- 禁止中英夹杂：每个实词都要译成{lang}；人名/昵称也须音译或意译，禁止残留源语言字符
{mix}
{examples}"#
    )
}

pub(crate) fn select_lang_hints(lang: &str) -> (&'static str, &'static str) {
    if lang.contains("中文") {
        (
            r#"例：原文「Work on it for 14 hours.」→ {"sentences":["连续做十四个小时。"]}
例：原文「Have you eaten yet today?」→ {"sentences":["你今天吃饭了吗？"]}
例：原文「I'll get back to you.」→ {"sentences":["我稍后回复你。"]}
反例（禁止）：{"sentences":["overnight没有重大新冲击，但crude仍在反应。"]}
反例（禁止直译腔）：{"sentences":["我将回到你。"]}"#,
            r#"
- 整句纯中文；overnight/crude 等必须中文化；英文习语用意译
- 专有名词：US→美国、Federal Reserve→美联储。极短缩写 OK/AI/GDP 可保留"#,
        )
    } else {
        (
            "",
            r#"
- 整句纯英文：禁止任何汉字残留
- 「再说」结构（「说」≠ talk/chat/say）：
  ①时间或状态条件+再说（明天/有空/定了/确认了/忙完）或「再说吧」=搁置 → leave it for/until… / once… / when…；修饰必须译出；禁止 talk about / chat later / discuss；禁止光秃 Let's see；禁止误当成先做该动作
  ②自主动作+再说（看看/考虑/研究）=先做再定 → first / after doing…
  ③句首「再说」+人称/评价=递进 → Besides / Anyway（再说我忙≠leave it）；「也不迟」=以后再处理也不晚，禁止 now
- 观望结局（走着瞧/拭目以待）：wait and see，禁止 Watch this
- 知会：「说一声」= let me know when you arrive/get there…，须写全
- 打住提及：never mind，禁止 You know it
- 交权：up to you / you decide，禁止 make it up
- 收尾再聊：仅原文含「聊」时用 chat / catch up later
- 拉群商量：start a group chat（不要 say it）
- 职场隐喻意译（坑→snag）；冷淡=cold shoulder；短、地道"#,
        )
    }
}
