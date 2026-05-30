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

use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
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
use futures::StreamExt;
use serde_json::{json, Value};
use std::convert::Infallible;

use super::profile_store::ProfileStore;

pub const PORT: u16 = 8765;

/// Port the proxy actually bound to. Set once at startup (it may differ from
/// PORT if 8765 was taken), and read when generating provider config so the
/// base_url always matches the live server.
static PROXY_PORT: AtomicU16 = AtomicU16::new(PORT);

pub fn active_port() -> u16 {
    PROXY_PORT.load(Ordering::Relaxed)
}

/// The base_url written into a chat-provider's config.toml.
pub fn upstream_url(provider_id: &str) -> String {
    format!("http://127.0.0.1:{}/v1/{provider_id}", active_port())
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

        // Try the preferred port, then a few fallbacks if it is already taken
        // (e.g. another Switcher instance), so the proxy always comes up.
        let mut listener = None;
        for offset in 0..10u16 {
            let port = PORT + offset;
            match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
                Ok(bound) => {
                    PROXY_PORT.store(port, Ordering::Relaxed);
                    listener = Some(bound);
                    break;
                }
                Err(_) => continue,
            }
        }
        match listener {
            Some(listener) => {
                if let Err(error) = axum::serve(listener, app).await {
                    eprintln!("[proxy] server error: {error}");
                }
            }
            None => eprintln!("[proxy] could not bind any port in {PORT}..{}", PORT + 10),
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
    chat_body["stream"] = Value::Bool(true);
    chat_body["stream_options"] = json!({ "include_usage": true });
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut upstream = state.client.post(&endpoint).json(&chat_body);
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        upstream = upstream.header(axum::http::header::AUTHORIZATION, auth);
    }

    let response = match upstream.send().await {
        Ok(response) => response,
        Err(error) => {
            state.store.log_proxy(&format!(
                "{provider} model={effective_model} -> {endpoint} CONNECT_ERROR {error}"
            ));
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("无法连接上游 {endpoint}：{error}"),
            );
        }
    };

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        state.store.log_proxy(&format!(
            "{provider} model={effective_model} -> {endpoint} {} ({} bytes)",
            status.as_u16(),
            text.len()
        ));
        return error_response(
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            &format!("上游返回 {status}: {text}"),
        );
    }

    // Translate the upstream Chat Completions SSE stream into Responses events
    // incrementally, so tokens (and reasoning) surface as they arrive.
    let store = state.store.clone();
    let log_provider = provider.clone();
    let log_endpoint = endpoint.clone();
    let sse_stream = async_stream::stream! {
        let mut translator = StreamTranslator::new(&effective_model);
        for event in translator.start() {
            yield Ok::<Event, Infallible>(Event::default().data(event.to_string()));
        }
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        while let Some(chunk) = byte_stream.next().await {
            let Ok(bytes) = chunk else { break };
            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(newline) = buffer.find('\n') {
                let line: String = buffer.drain(..=newline).collect();
                let line = line.trim();
                let Some(payload) = line.strip_prefix("data:") else { continue };
                let payload = payload.trim();
                if payload.is_empty() || payload == "[DONE]" {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<Value>(payload) {
                    for event in translator.push(&value) {
                        yield Ok(Event::default().data(event.to_string()));
                    }
                }
            }
        }
        for event in translator.finish() {
            yield Ok(Event::default().data(event.to_string()));
        }
        store.log_proxy(&format!(
            "{log_provider} model={} -> {log_endpoint} 200 stream (in={} out={} reasoning={})",
            effective_model, translator.usage_input, translator.usage_output, translator.usage_reasoning
        ));
        store.record_usage(&log_provider, &effective_model, translator.usage_input, translator.usage_output);
    };
    Sse::new(sse_stream).into_response()
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
                messages.push(json!({ "role": normalize_role(role), "content": content }));
            }
        }
    }
}

/// Chat Completions only accepts system/user/assistant/tool. Map the Responses
/// API's "developer" role (used by Codex for its instructions) to "system".
fn normalize_role(role: &str) -> &str {
    match role {
        "developer" => "system",
        other => other,
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
// Streaming response translation: Chat Completions SSE -> Responses SSE
// ----------------------------------------------------------------------------

struct ToolAccum {
    output_index: usize,
    item_id: String,
    call_id: String,
    name: String,
    args: String,
}

/// Incrementally converts upstream Chat Completions stream chunks into Responses
/// API SSE events. Tracks an optional reasoning item, an optional assistant
/// message, and any number of function-call items, emitting open/delta/close
/// events in Responses order.
pub struct StreamTranslator {
    response_id: String,
    model: String,
    created_at: u64,
    next_index: usize,

    reasoning_id: Option<String>,
    reasoning_index: usize,
    reasoning_text: String,
    reasoning_closed: bool,

    message_id: Option<String>,
    message_index: usize,
    message_text: String,

    tools: Vec<ToolAccum>,

    pub usage_input: u64,
    pub usage_output: u64,
    pub usage_reasoning: u64,
}

impl StreamTranslator {
    pub fn new(model: &str) -> Self {
        Self {
            response_id: format!("resp_{}", unique_suffix()),
            model: model.to_string(),
            created_at: now_secs(),
            next_index: 0,
            reasoning_id: None,
            reasoning_index: 0,
            reasoning_text: String::new(),
            reasoning_closed: false,
            message_id: None,
            message_index: 0,
            message_text: String::new(),
            tools: Vec::new(),
            usage_input: 0,
            usage_output: 0,
            usage_reasoning: 0,
        }
    }

    fn response_obj(&self, status: &str, output: &[Value]) -> Value {
        json!({
            "id": self.response_id,
            "object": "response",
            "created_at": self.created_at,
            "status": status,
            "model": self.model,
            "output": output,
        })
    }

    pub fn start(&self) -> Vec<Value> {
        vec![
            json!({ "type": "response.created", "response": self.response_obj("in_progress", &[]) }),
            json!({ "type": "response.in_progress", "response": self.response_obj("in_progress", &[]) }),
        ]
    }

    pub fn push(&mut self, chunk: &Value) -> Vec<Value> {
        let mut events = Vec::new();

        if let Some(usage) = chunk.get("usage").filter(|usage| !usage.is_null()) {
            self.capture_usage(usage);
        }

        let delta = chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"));
        let Some(delta) = delta else {
            return events;
        };

        if let Some(reasoning) = delta
            .get("reasoning_content")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        {
            self.push_reasoning(reasoning, &mut events);
        }

        if let Some(content) = delta
            .get("content")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        {
            self.close_reasoning(&mut events);
            self.push_content(content, &mut events);
        }

        if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
                self.push_tool_call(call, &mut events);
            }
        }

        events
    }

    pub fn finish(&mut self) -> Vec<Value> {
        let mut events = Vec::new();
        self.close_reasoning(&mut events);

        let mut output: Vec<Value> = Vec::new();
        if let Some(reasoning_id) = &self.reasoning_id {
            output.push(json!({
                "id": reasoning_id,
                "type": "reasoning",
                "summary": [{ "type": "summary_text", "text": self.reasoning_text }],
            }));
        }

        if let Some(message_id) = self.message_id.clone() {
            events.push(json!({
                "type": "response.output_text.done",
                "item_id": message_id,
                "output_index": self.message_index,
                "content_index": 0,
                "text": self.message_text,
            }));
            events.push(json!({
                "type": "response.content_part.done",
                "item_id": message_id,
                "output_index": self.message_index,
                "content_index": 0,
                "part": { "type": "output_text", "text": self.message_text, "annotations": [] }
            }));
            let item = json!({
                "id": message_id,
                "type": "message",
                "status": "completed",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": self.message_text, "annotations": [] }]
            });
            events.push(json!({ "type": "response.output_item.done", "output_index": self.message_index, "item": item.clone() }));
            output.push(item);
        }

        for tool in &self.tools {
            events.push(json!({
                "type": "response.function_call_arguments.done",
                "item_id": tool.item_id,
                "output_index": tool.output_index,
                "arguments": tool.args,
            }));
            let item = json!({
                "id": tool.item_id,
                "type": "function_call",
                "status": "completed",
                "name": tool.name,
                "arguments": tool.args,
                "call_id": tool.call_id,
            });
            events.push(json!({ "type": "response.output_item.done", "output_index": tool.output_index, "item": item.clone() }));
            output.push(item);
        }

        let mut completed = self.response_obj("completed", &output);
        completed["usage"] = json!({
            "input_tokens": self.usage_input,
            "output_tokens": self.usage_output,
            "total_tokens": self.usage_input + self.usage_output,
        });
        events.push(json!({ "type": "response.completed", "response": completed }));
        events
    }

    fn push_reasoning(&mut self, text: &str, events: &mut Vec<Value>) {
        if self.reasoning_id.is_none() {
            let id = format!("rs_{}", unique_suffix());
            self.reasoning_index = self.next_index;
            self.next_index += 1;
            self.reasoning_id = Some(id.clone());
            events.push(json!({
                "type": "response.output_item.added",
                "output_index": self.reasoning_index,
                "item": { "id": id, "type": "reasoning", "summary": [] }
            }));
            events.push(json!({
                "type": "response.reasoning_summary_part.added",
                "item_id": id,
                "output_index": self.reasoning_index,
                "summary_index": 0,
                "part": { "type": "summary_text", "text": "" }
            }));
        }
        self.reasoning_text.push_str(text);
        events.push(json!({
            "type": "response.reasoning_summary_text.delta",
            "item_id": self.reasoning_id.as_ref().unwrap(),
            "output_index": self.reasoning_index,
            "summary_index": 0,
            "delta": text,
        }));
    }

    fn close_reasoning(&mut self, events: &mut Vec<Value>) {
        if self.reasoning_closed {
            return;
        }
        if let Some(id) = self.reasoning_id.clone() {
            events.push(json!({
                "type": "response.reasoning_summary_text.done",
                "item_id": id,
                "output_index": self.reasoning_index,
                "summary_index": 0,
                "text": self.reasoning_text,
            }));
            events.push(json!({
                "type": "response.reasoning_summary_part.done",
                "item_id": id,
                "output_index": self.reasoning_index,
                "summary_index": 0,
                "part": { "type": "summary_text", "text": self.reasoning_text }
            }));
            events.push(json!({
                "type": "response.output_item.done",
                "output_index": self.reasoning_index,
                "item": { "id": id, "type": "reasoning", "summary": [{ "type": "summary_text", "text": self.reasoning_text }] }
            }));
            self.reasoning_closed = true;
        }
    }

    fn push_content(&mut self, text: &str, events: &mut Vec<Value>) {
        if self.message_id.is_none() {
            let id = format!("msg_{}", unique_suffix());
            self.message_index = self.next_index;
            self.next_index += 1;
            self.message_id = Some(id.clone());
            events.push(json!({
                "type": "response.output_item.added",
                "output_index": self.message_index,
                "item": { "id": id, "type": "message", "status": "in_progress", "role": "assistant", "content": [] }
            }));
            events.push(json!({
                "type": "response.content_part.added",
                "item_id": id,
                "output_index": self.message_index,
                "content_index": 0,
                "part": { "type": "output_text", "text": "", "annotations": [] }
            }));
        }
        self.message_text.push_str(text);
        events.push(json!({
            "type": "response.output_text.delta",
            "item_id": self.message_id.as_ref().unwrap(),
            "output_index": self.message_index,
            "content_index": 0,
            "delta": text,
        }));
    }

    fn push_tool_call(&mut self, call: &Value, events: &mut Vec<Value>) {
        let upstream_index = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let function = call.get("function");
        let name_fragment = function
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let args_fragment = function
            .and_then(|function| function.get("arguments"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let call_id = call.get("id").and_then(Value::as_str);

        // Find or create the accumulator for this upstream tool index.
        if self.tools.get(upstream_index).is_none() {
            // Grow to fit; upstream indices are sequential in practice.
            while self.tools.len() <= upstream_index {
                let output_index = self.next_index;
                self.next_index += 1;
                let item_id = format!("fc_{}", unique_suffix());
                self.tools.push(ToolAccum {
                    output_index,
                    item_id: item_id.clone(),
                    call_id: format!("call_{}", unique_suffix()),
                    name: String::new(),
                    args: String::new(),
                });
                events.push(json!({
                    "type": "response.output_item.added",
                    "output_index": output_index,
                    "item": { "id": item_id, "type": "function_call", "status": "in_progress", "name": "", "arguments": "", "call_id": self.tools.last().unwrap().call_id }
                }));
            }
        }

        let tool = &mut self.tools[upstream_index];
        if let Some(id) = call_id {
            tool.call_id = id.to_string();
        }
        if !name_fragment.is_empty() {
            tool.name.push_str(name_fragment);
        }
        if !args_fragment.is_empty() {
            tool.args.push_str(args_fragment);
            events.push(json!({
                "type": "response.function_call_arguments.delta",
                "item_id": tool.item_id,
                "output_index": tool.output_index,
                "delta": args_fragment,
            }));
        }
    }

    fn capture_usage(&mut self, usage: &Value) {
        self.usage_input = usage.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(self.usage_input);
        self.usage_output = usage.get("completion_tokens").and_then(Value::as_u64).unwrap_or(self.usage_output);
        self.usage_reasoning = usage
            .get("completion_tokens_details")
            .and_then(|details| details.get("reasoning_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(self.usage_reasoning);
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}{seq:x}")
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
    fn maps_developer_role_to_system() {
        let request = json!({
            "input": [
                { "type": "message", "role": "developer", "content": [{ "type": "input_text", "text": "rules" }] },
                { "type": "message", "role": "user", "content": "hi" }
            ]
        });
        let chat = responses_to_chat(&request);
        let messages = chat["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "rules");
        assert_eq!(messages[1]["role"], "user");
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

    fn all_events(model: &str, chunks: &[Value]) -> Vec<Value> {
        let mut translator = StreamTranslator::new(model);
        let mut events = translator.start();
        for chunk in chunks {
            events.extend(translator.push(chunk));
        }
        events.extend(translator.finish());
        events
    }

    fn types_of(events: &[Value]) -> Vec<String> {
        events.iter().map(|e| e["type"].as_str().unwrap_or("").to_string()).collect()
    }

    #[test]
    fn streams_text_token_by_token() {
        let chunks = vec![
            json!({ "choices": [{ "delta": { "content": "Hello" } }] }),
            json!({ "choices": [{ "delta": { "content": " world" } }] }),
            json!({ "choices": [{ "delta": {} }], "usage": { "prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5 } }),
        ];
        let events = all_events("deepseek-chat", &chunks);
        let types = types_of(&events);
        assert_eq!(types.first().map(String::as_str), Some("response.created"));
        assert_eq!(types.last().map(String::as_str), Some("response.completed"));
        // Two separate content deltas were emitted (true streaming).
        let deltas: Vec<&str> = events
            .iter()
            .filter(|e| e["type"] == "response.output_text.delta")
            .map(|e| e["delta"].as_str().unwrap())
            .collect();
        assert_eq!(deltas, vec!["Hello", " world"]);
        let completed = events.last().unwrap();
        assert_eq!(completed["response"]["output"][0]["content"][0]["text"], "Hello world");
        assert_eq!(completed["response"]["usage"]["input_tokens"], 3);
        assert_eq!(completed["response"]["usage"]["output_tokens"], 2);
    }

    #[test]
    fn streams_reasoning_then_content() {
        let chunks = vec![
            json!({ "choices": [{ "delta": { "reasoning_content": "think..." } }] }),
            json!({ "choices": [{ "delta": { "content": "answer" } }] }),
        ];
        let events = all_events("deepseek-reasoner", &chunks);
        let types = types_of(&events);
        assert!(types.contains(&"response.reasoning_summary_text.delta".to_string()));
        // reasoning item (index 0) is closed before the message (index 1) opens.
        let reasoning_done = events.iter().position(|e| e["type"] == "response.output_item.done" && e["item"]["type"] == "reasoning").unwrap();
        let message_added = events.iter().position(|e| e["type"] == "response.output_item.added" && e["item"]["type"] == "message").unwrap();
        assert!(reasoning_done < message_added);
        let completed = events.last().unwrap();
        assert_eq!(completed["response"]["output"][0]["type"], "reasoning");
        assert_eq!(completed["response"]["output"][1]["type"], "message");
    }

    #[test]
    fn streams_tool_call_fragments() {
        let chunks = vec![
            json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 0, "id": "call_1", "function": { "name": "shell", "arguments": "{\"cmd\":" } }] } }] }),
            json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 0, "function": { "arguments": "\"ls\"}" } }] } }] }),
        ];
        let events = all_events("deepseek-chat", &chunks);
        let args_deltas: Vec<&str> = events
            .iter()
            .filter(|e| e["type"] == "response.function_call_arguments.delta")
            .map(|e| e["delta"].as_str().unwrap())
            .collect();
        assert_eq!(args_deltas, vec!["{\"cmd\":", "\"ls\"}"]);
        let done = events.iter().find(|e| e["type"] == "response.output_item.done" && e["item"]["type"] == "function_call").unwrap();
        assert_eq!(done["item"]["name"], "shell");
        assert_eq!(done["item"]["arguments"], "{\"cmd\":\"ls\"}");
        assert_eq!(done["item"]["call_id"], "call_1");
    }
}
