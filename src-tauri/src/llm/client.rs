pub(crate) fn llm_debug() -> bool {
    matches!(
        std::env::var("INPUT_LLM_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

pub(crate) async fn chat_once(system: &str, user: &str) -> Result<String, String> {
    crate::local::chat(system, user).await
}

pub(crate) async fn chat_once_plain(system: &str, user: &str) -> Result<String, String> {
    crate::local::chat_plain(system, user).await
}
