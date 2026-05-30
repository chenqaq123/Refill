//! Local translation proxy that lets Codex (which only speaks the OpenAI
//! Responses API) talk to upstream providers that only offer Chat Completions,
//! e.g. DeepSeek.
//!
//! Codex is pointed at `http://127.0.0.1:<PORT>/v1/<provider_id>` and appends
//! `/responses`. We:
//!   1. translate the Responses request body into a Chat Completions body,
//!   2. forward it (non-streaming) to the real upstream `/chat/completions`,
//!   3. synthesize the Responses SSE event sequence Codex expects.
//!
//! Synthesizing the event stream from a single non-streaming upstream response
//! is far more robust than mapping streaming deltas one-to-one, at the cost of
//! the reply arriving in one chunk instead of token-by-token.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::post,
    Router,
};
use futures::stream;
use serde_json::{json, Value};

use super::profile_store::ProfileStore;

pub const PORT: u16 = 8765;

/// The base_url written into a chat-provider's config.toml.
pub fn upstream_url(provider_id: &str) -> String {
    format!("http://127.0.0.1:{PORT}/v1/{provider_id}")
}

#[derive(Clone)]
struct ProxyState {
    store: Arc<ProfileStore>,
    client: reqwest::Client,
}

/// Spawn the proxy server on the Tauri/tokio runtime. Best-effort: if the port
/// is already taken (another Switcher instance), we simply log and move on.
pub fn start(store: Arc<ProfileStore>) {
    tauri::async_runtime::spawn(async move {
        let state = ProxyState {
            store,
            client: reqwest::Client::new(),
        };
        let app = Router::new()
            .route("/v1/{provider}/responses", post(handle_responses))
            .route("/{provider}/responses", post(handle_responses))
            .with_state(state);

        let addr = format!("127.0.0.1:{PORT}");
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                if let Err(error) = axum::serve(listener, app).await {
                    eprintln!("[proxy] server error: {error}");
                }
            }
            Err(error) => {
                eprintln!("[proxy] could not bind {addr}: {error}");
            }
        }
    });
}

async fn handle_responses(
    State(state): State<ProxyState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some((base_url, model)) = state.store.provider_upstream(&provider) else {
        return error_response(
            StatusCode::NOT_FOUND,
            &format!("未知的 provider：{provider}"),
        );
    };

    let request: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("无法解析请求体：{error}"))
        }
    };

    let mut chat_body = responses_to_chat(&request);
    // Codex's model picker may send a model that doesn't belong to this
    // provider (e.g. "gpt-5.5"). Force the provider's configured model so the
    // upstream always receives a name it recognizes.
    let effective_model = if model.is_empty() {
        request
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string()
    } else {
        model
    };
    chat_body["model"] = Value::String(effective_model.clone());
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut upstream = state.client.post(&endpoint).json(&chat_body);
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        upstream = upstream.header(axum::http::header::AUTHORIZATION, auth);
    }

    let response = match upstream.send().await {
        Ok(response) => response,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("无法连接上游 {endpoint}：{error}"),
            )
        }
    };

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return error_response(
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            &format!("上游返回 {status}: {text}"),
        );
    }

    let chat: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("上游响应不是合法 JSON：{error}"),
            )
        }
    };

    let events = chat_to_responses_events(&chat, &effective_model);
    let stream = stream::iter(
        events
            .into_iter()
            .map(|event| Ok::<Event, std::convert::Infallible>(Event::default().data(event.to_string()))),
    );
    Sse::new(stream).into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(json!({ "error": { "message": message } }))).into_response()
}

// ----------------------------------------------------------------------------
// Request translation: Responses API -> Chat Completions
// ----------------------------------------------------------------------------

fn responses_to_chat(request: &Value) -> Value {
    let mut messages: Vec<Value> = Vec::new();

    if let Some(instructions) = request.get("instructions").and_then(Value::as_str) {
        if !instructions.is_empty() {
            messages.push(json!({ "role": "system", "content": instructions }));
        }
    }

    if let Some(items) = request.get("input").and_then(Value::as_array) {
        for item in items {
            push_input_item(item, &mut messages);
        }
    } else if let Some(text) = request.get("input").and_then(Value::as_str) {
        messages.push(json!({ "role": "user", "content": text }));
    }

    let mut chat = json!({
        "model": request.get("model").cloned().unwrap_or(Value::Null),
        "messages": messages,
        "stream": false,
    });

    if let Some(tools) = request.get("tools").and_then(Value::as_array) {
        let converted: Vec<Value> = tools.iter().filter_map(convert_tool).collect();
        if !converted.is_empty() {
            chat["tools"] = json!(converted);
        }
    }
    if let Some(choice) = request.get("tool_choice") {
        chat["tool_choice"] = choice.clone();
    }
    if let Some(temp) = request.get("temperature") {
        chat["temperature"] = temp.clone();
    }
    if let Some(top_p) = request.get("top_p") {
        chat["top_p"] = top_p.clone();
    }
    if let Some(max) = request.get("max_output_tokens") {
        chat["max_tokens"] = max.clone();
    }

    chat
}

fn push_input_item(item: &Value, messages: &mut Vec<Value>) {
    match item.get("type").and_then(Value::as_str) {
        Some("function_call") => {
            let call_id = item.get("call_id").and_then(Value::as_str).unwrap_or("");
            messages.push(json!({
                "role": "assistant",
                "content": Value::Null,
                "tool_calls": [{
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": item.get("name").and_then(Value::as_str).unwrap_or(""),
                        "arguments": item.get("arguments").and_then(Value::as_str).unwrap_or(""),
                    }
                }]
            }));
        }
        Some("function_call_output") => {
            let call_id = item.get("call_id").and_then(Value::as_str).unwrap_or("");
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": output_to_text(item.get("output")),
            }));
        }
        // "message" or untyped role/content entries.
        _ => {
            if let Some(role) = item.get("role").and_then(Value::as_str) {
                let content = content_to_text(item.get("content"));
                messages.push(json!({ "role": role, "content": content }));
            }
        }
    }
}

fn convert_tool(tool: &Value) -> Option<Value> {
    if tool.get("type").and_then(Value::as_str) != Some("function") {
        return None;
    }
    // Responses flattens the function fields onto the tool object; Chat nests
    // them under "function".
    if tool.get("function").is_some() {
        return Some(tool.clone());
    }
    let name = tool.get("name")?;
    let mut function = json!({ "name": name });
    if let Some(description) = tool.get("description") {
        function["description"] = description.clone();
    }
    if let Some(parameters) = tool.get("parameters") {
        function["parameters"] = parameters.clone();
    }
    Some(json!({ "type": "function", "function": function }))
}

fn content_to_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn output_to_text(output: Option<&Value>) -> String {
    match output {
        Some(Value::String(text)) => text.clone(),
        Some(other) => content_to_text(Some(other)),
        None => String::new(),
    }
}

// ----------------------------------------------------------------------------
// Response synthesis: Chat Completions -> Responses SSE events
// ----------------------------------------------------------------------------

fn chat_to_responses_events(chat: &Value, model: &str) -> Vec<Value> {
    let response_id = format!("resp_{}", unique_suffix());
    let created_at = now_secs();
    let message = chat
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"));

    let text = message
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let tool_calls = message
        .and_then(|message| message.get("tool_calls"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut output_items: Vec<Value> = Vec::new();
    let mut events: Vec<Value> = Vec::new();

    let base_response = |status: &str, output: &[Value]| {
        json!({
            "id": response_id,
            "object": "response",
            "created_at": created_at,
            "status": status,
            "model": model,
            "output": output,
        })
    };

    events.push(json!({ "type": "response.created", "response": base_response("in_progress", &[]) }));
    events.push(json!({ "type": "response.in_progress", "response": base_response("in_progress", &[]) }));

    let mut output_index = 0usize;

    if !text.is_empty() {
        let item_id = format!("msg_{}", unique_suffix());
        events.push(json!({
            "type": "response.output_item.added",
            "output_index": output_index,
            "item": { "id": item_id, "type": "message", "status": "in_progress", "role": "assistant", "content": [] }
        }));
        events.push(json!({
            "type": "response.content_part.added",
            "item_id": item_id,
            "output_index": output_index,
            "content_index": 0,
            "part": { "type": "output_text", "text": "", "annotations": [] }
        }));
        events.push(json!({
            "type": "response.output_text.delta",
            "item_id": item_id,
            "output_index": output_index,
            "content_index": 0,
            "delta": text,
        }));
        events.push(json!({
            "type": "response.output_text.done",
            "item_id": item_id,
            "output_index": output_index,
            "content_index": 0,
            "text": text,
        }));
        events.push(json!({
            "type": "response.content_part.done",
            "item_id": item_id,
            "output_index": output_index,
            "content_index": 0,
            "part": { "type": "output_text", "text": text, "annotations": [] }
        }));
        let item = json!({
            "id": item_id,
            "type": "message",
            "status": "completed",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text, "annotations": [] }]
        });
        events.push(json!({
            "type": "response.output_item.done",
            "output_index": output_index,
            "item": item.clone(),
        }));
        output_items.push(item);
        output_index += 1;
    }

    for call in &tool_calls {
        let function = call.get("function");
        let name = function
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let arguments = function
            .and_then(|function| function.get("arguments"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let call_id = call
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("call_{}", unique_suffix()));
        let item_id = format!("fc_{}", unique_suffix());

        let pending = json!({
            "id": item_id,
            "type": "function_call",
            "status": "in_progress",
            "name": name,
            "arguments": "",
            "call_id": call_id,
        });
        events.push(json!({
            "type": "response.output_item.added",
            "output_index": output_index,
            "item": pending,
        }));
        events.push(json!({
            "type": "response.function_call_arguments.delta",
            "item_id": item_id,
            "output_index": output_index,
            "delta": arguments,
        }));
        events.push(json!({
            "type": "response.function_call_arguments.done",
            "item_id": item_id,
            "output_index": output_index,
            "arguments": arguments,
        }));
        let item = json!({
            "id": item_id,
            "type": "function_call",
            "status": "completed",
            "name": name,
            "arguments": arguments,
            "call_id": call_id,
        });
        events.push(json!({
            "type": "response.output_item.done",
            "output_index": output_index,
            "item": item.clone(),
        }));
        output_items.push(item);
        output_index += 1;
    }

    let mut completed = base_response("completed", &output_items);
    if let Some(usage) = chat.get("usage") {
        completed["usage"] = convert_usage(usage);
    }
    events.push(json!({ "type": "response.completed", "response": completed }));

    events
}

fn convert_usage(usage: &Value) -> Value {
    let input = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let output = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(input + output);
    json!({
        "input_tokens": input,
        "output_tokens": output,
        "total_tokens": total,
    })
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("{:x}", duration.as_nanos()))
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_messages_and_instructions() {
        let request = json!({
            "model": "deepseek-chat",
            "instructions": "be nice",
            "input": [
                { "type": "message", "role": "user", "content": [{ "type": "input_text", "text": "hi" }] }
            ]
        });
        let chat = responses_to_chat(&request);
        assert_eq!(chat["model"], "deepseek-chat");
        let messages = chat["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "be nice");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hi");
        assert_eq!(chat["stream"], false);
    }

    #[test]
    fn translates_function_call_round_trip() {
        let request = json!({
            "input": [
                { "type": "function_call", "call_id": "c1", "name": "shell", "arguments": "{\"cmd\":\"ls\"}" },
                { "type": "function_call_output", "call_id": "c1", "output": "file.txt" }
            ]
        });
        let chat = responses_to_chat(&request);
        let messages = chat["messages"].as_array().unwrap();
        assert_eq!(messages[0]["tool_calls"][0]["id"], "c1");
        assert_eq!(messages[0]["tool_calls"][0]["function"]["name"], "shell");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "c1");
        assert_eq!(messages[1]["content"], "file.txt");
    }

    #[test]
    fn converts_responses_tool_format() {
        let tool = json!({
            "type": "function",
            "name": "shell",
            "description": "run",
            "parameters": { "type": "object" }
        });
        let converted = convert_tool(&tool).unwrap();
        assert_eq!(converted["type"], "function");
        assert_eq!(converted["function"]["name"], "shell");
        assert_eq!(converted["function"]["description"], "run");
    }

    #[test]
    fn synthesizes_text_event_sequence() {
        let chat = json!({
            "choices": [{ "message": { "role": "assistant", "content": "Hello world" } }],
            "usage": { "prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5 }
        });
        let events = chat_to_responses_events(&chat, "deepseek-chat");
        let types: Vec<&str> = events.iter().map(|e| e["type"].as_str().unwrap()).collect();
        assert_eq!(types.first(), Some(&"response.created"));
        assert!(types.contains(&"response.output_text.delta"));
        assert!(types.contains(&"response.output_item.done"));
        assert_eq!(types.last(), Some(&"response.completed"));

        let delta = events
            .iter()
            .find(|e| e["type"] == "response.output_text.delta")
            .unwrap();
        assert_eq!(delta["delta"], "Hello world");

        let completed = events.last().unwrap();
        assert_eq!(completed["response"]["status"], "completed");
        assert_eq!(completed["response"]["output"][0]["content"][0]["text"], "Hello world");
        assert_eq!(completed["response"]["usage"]["input_tokens"], 3);
    }

    #[test]
    fn synthesizes_tool_call_events() {
        let chat = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": Value::Null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "shell", "arguments": "{\"cmd\":\"ls\"}" }
                    }]
                }
            }]
        });
        let events = chat_to_responses_events(&chat, "deepseek-chat");
        let types: Vec<&str> = events.iter().map(|e| e["type"].as_str().unwrap()).collect();
        assert!(types.contains(&"response.function_call_arguments.done"));
        let done = events
            .iter()
            .find(|e| e["type"] == "response.output_item.done")
            .unwrap();
        assert_eq!(done["item"]["type"], "function_call");
        assert_eq!(done["item"]["name"], "shell");
        assert_eq!(done["item"]["call_id"], "call_1");
    }
}
