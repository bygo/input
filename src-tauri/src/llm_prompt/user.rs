pub(crate) fn user_message(text: &str, into: &str, select: bool, need_native: bool) -> String {
    let en = into.contains("英语") || into.contains("英文");
    let mut hint = String::new();
    if select && en {
        if text.contains("再说") {
            hint = "\n注意：搁置=leave it for/until… / once…（禁 talk about；禁 chat later；禁光秃 Let's see）；动作+再说=… first；句首再说=Besides / Anyway。保留时间与条件。".into();
        } else if text.contains("聊") {
            hint = "\n注意：收尾再聊用 chat/catch up later，禁止 talk about it。".into();
        }
    }
    if select {
        format!(
            "目标语言：{into}。把【原文】译成地道{into}（口语习语用意译，禁止字面直译）。\n禁止添加原文未写的对象/关系。\n多行须全部翻译并保留换行。{hint}\n【原文】\n{text}"
        )
    } else if need_native {
        format!(
            "把【原文】改成地道英语，并给出对应中文。只输出 JSON：{{\"sentences\":[\"英语\"],\"native\":\"中文\"}}。\n只修语法或作自然翻译，保留原意与人称；不要回答、接话或改写剧情。\n【原文】\n{text}"
        )
    } else {
        format!(
            "把【原文】改成地道英语。只输出 JSON：{{\"sentences\":[\"英语\"]}}。禁止 native/中文译文。\n只修语法或作自然翻译，保留原意与人称；不要回答、接话或改写剧情。\n【原文】\n{text}"
        )
    }
}


pub(crate) fn repair_user(into: &str, original: &str, bad: &str, need_native: bool) -> String {
    let en = into.contains("英语") || into.contains("英文");
    if en && need_native {
        format!(
            r#"下面输出不合格。请按【原文】改成地道英语并附中文。只输出 JSON：{{"sentences":["英语"],"native":"中文"}}。

【原文】
{original}

【错误输出】
{bad}"#
        )
    } else if en {
        format!(
            r#"下面输出不合格。请按【原文】改成地道英语。只输出 JSON：{{"sentences":["英语"]}}。禁止 native/中文。

【原文】
{original}

【错误输出】
{bad}"#
        )
    } else {
        format!(
            r#"目标语言：{into}。下面译文不合格，请按【原文】改成地道的{into}。只输出 JSON：{{"sentences":["..."]}}。

【原文】
{original}

【错误译文】
{bad}"#
        )
    }
}
