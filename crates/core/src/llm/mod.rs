pub mod chat_history;
pub mod connector;
pub mod manager;
pub mod onet_cli_provider;
pub mod storage;
pub mod types;

pub use connector::{ChatStream, LlmConnector, LlmProvider};
pub use manager::{GlobalProviderState, ProviderManager};
pub use onet_cli_provider::OnetCliLLMProvider;
pub use types::{ProviderConfig, ProviderType};

pub use llm_connector::types::{ChatRequest, Message, MessageBlock, Role, StreamingResponse};

use gpui::App;

/// 提取流式响应中当前可展示的文本。
///
/// 优先使用 provider 返回的正文内容；若正文为空，则回退到 reasoning/thinking，
/// 以兼容 Ollama 下 Qwen 等只在 `thinking` 字段返回内容的模型。
pub fn extract_stream_text(response: &StreamingResponse) -> Option<&str> {
    response.get_content().or_else(|| {
        response
            .choices
            .iter()
            .find_map(|choice| choice.delta.reasoning_any().filter(|text| !text.is_empty()))
    })
}

/// Debug-only LLM 请求日志，避免在普通日志级别刷屏。
pub fn debug_llm_request(
    scope: &str,
    provider: &ProviderConfig,
    request: &ChatRequest,
    extra: impl AsRef<str>,
) {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return;
    }

    let api_base = provider.api_base.as_deref().unwrap_or("<default>");
    let api_key_state = if provider
        .api_key
        .as_deref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        "set"
    } else {
        "empty"
    };

    tracing::debug!(
        target: "onetcli::llm::request",
        scope = scope,
        provider_type = %provider.provider_type.as_str(),
        provider_name = %provider.name,
        model = %request.model,
        api_base = %api_base,
        api_key = api_key_state,
        message_count = request.messages.len(),
        stream = request.stream.unwrap_or(false),
        max_tokens = request.max_tokens.unwrap_or_default(),
        temperature = request.temperature.unwrap_or_default(),
        extra = extra.as_ref(),
        "LLM request debug"
    );
}

pub fn init(cx: &mut App) {
    storage::init(cx);
    let state = GlobalProviderState::new();
    cx.set_global(state);
}

#[cfg(test)]
mod tests {
    use super::extract_stream_text;
    use llm_connector::types::{Delta, StreamingChoice, StreamingResponse};

    #[test]
    fn extract_stream_text_prefers_content() {
        let response = StreamingResponse {
            content: "可见正文".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Delta {
                    content: Some("可见正文".to_string()),
                    thinking: Some("内部思考".to_string()),
                    ..Default::default()
                },
                finish_reason: None,
                logprobs: None,
            }],
            ..Default::default()
        };

        assert_eq!(extract_stream_text(&response), Some("可见正文"));
    }

    #[test]
    fn extract_stream_text_falls_back_to_reasoning() {
        let response = StreamingResponse {
            choices: vec![StreamingChoice {
                index: 0,
                delta: Delta {
                    thinking: Some("推理内容".to_string()),
                    ..Default::default()
                },
                finish_reason: Some("length".to_string()),
                logprobs: None,
            }],
            ..Default::default()
        };

        assert_eq!(extract_stream_text(&response), Some("推理内容"));
    }
}
