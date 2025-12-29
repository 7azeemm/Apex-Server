use std::collections::VecDeque;
use crate::api::chats::{update_chat, get_chat_or_create};
use crate::structs::chat::{Message, Sender, ToolExecution};
use crate::utils::validated_json::ValidatedJson;
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::response::{Response, Sse};
use axum::Extension;
use common::extensions::json_ext::JsonExt;
use futures::StreamExt;
use futures::{stream, Stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{sleep, Instant};
use tracing::{error, info};
use common::http::HTTP_CLIENT;
use crate::constants::{AI_SERVER_CHAT_ENDPOINT, CACHED_TOKENS_RATE};
use crate::structs::api_structs::ApiResponse;
use crate::structs::auth_structs::Session;
use crate::structs::plan::Plan;

const MAX_PROMPT_CHARS: usize = 4000;

#[derive(Debug, Deserialize)]
pub struct CompletionsRequest {
    chat_uuid: String,
    prompt: String,
    retry: Option<bool>,
}

enum StreamItem {
    Token(String),
    Usage(Value),
}

pub async fn completions_handler(
    Extension(session): Extension<Arc<RwLock<Session>>>,
    ValidatedJson(request): ValidatedJson<CompletionsRequest>,
) -> Result<Sse<impl Stream<Item=Result<Event, Infallible>>>, Response> {
    let chat_uuid = request.chat_uuid;
    let prompt = request.prompt;
    let retry = request.retry.unwrap_or(false);

    if prompt.len() > MAX_PROMPT_CHARS {
        return Err(ApiResponse::err(
            "Prompt is too long!",
            StatusCode::BAD_REQUEST
        ));
    }

    let (plan, player) = {
        let session = session.read().await;
        let user = session.user();
        if user.exceeded_limit() {
            info!("{} reached token daily limit (daily_tokens: {})", user.player_name(), user.plan().daily_tokens());
            return Err(ApiResponse::err(
                "Daily token limit reached",
                StatusCode::PAYMENT_REQUIRED,
            ));
        }
        (user.plan().clone(), user.player_name().clone())
    };

    let chat = get_chat_or_create(&session, chat_uuid, &prompt, retry).await?;
    let chat_uuid = chat.uuid().to_owned();
    let chat_name = chat.name().to_owned();

    let context_window = plan.context_window();
    let messages = build_context_messages(chat.messages(), &prompt, context_window);

    let request = json!({"messages": messages, "player": player});
    let response = HTTP_CLIENT
        .post(AI_SERVER_CHAT_ENDPOINT)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            ApiResponse::internal_err("Failed to connect to the AI Model".to_string(), e, &[])
        })?;

    let start_time = Instant::now();
    let (tx, rx) = mpsc::channel::<StreamItem>(200);

    tokio::spawn(async move {
        let mut stream = response.bytes_stream();
        let mut full_text = String::new();
        let mut tools_data = None;

        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Err(e) => error!(?e, "Streaming Error"),
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    match serde_json::from_str::<Value>(&text) {
                        Err(err) => error!(?err, "Failed to decode data from AI server"),
                        Ok(mut json) => {
                            if let Some(content) = json.get_str("completions/content") {
                                full_text.push_str(content);

                                let pieces: Vec<String> = content.split_inclusive(char::is_whitespace)
                                    .map(|s| s.to_string())
                                    .collect();

                                for piece in pieces {
                                    let _ = tx.send(StreamItem::Token(piece)).await;
                                }
                            } else if let Some(usage) = json.get("usage") {
                                let input_tokens = usage.get_i64("input_tokens").unwrap_or(0);
                                let output_tokens = usage.get_i64("output_tokens").unwrap_or(0);
                                let cached_tokens = usage.get_i64("cached_tokens").unwrap_or(0);
                                let prompt_tokens = usage.get_i64("prompt_tokens").unwrap_or(0);
                                let tokens_usage = (input_tokens - cached_tokens) + output_tokens + (cached_tokens as f32 * CACHED_TOKENS_RATE) as i64;

                                json["usage"]["tokens_usage"] = Value::from(tokens_usage);
                                let _ = tx.send(StreamItem::Usage(json)).await;

                                let prompt = Message::new(Sender::User, prompt, None, prompt_tokens);
                                let response = Message::new(Sender::Assistant, full_text, tools_data, output_tokens);

                                update_chat(&session, &chat, vec![prompt, response], tokens_usage).await;
                                break;
                            } else if let Some(tools) = json.get("tools") {
                                match serde_json::from_value::<Vec<ToolExecution>>(tools.clone()) {
                                    Ok(tools) => tools_data = Some(tools),
                                    Err(err) => error!(?err, "Failed to serialize tools")
                                }
                            } else if let Some(error) = json.get("error") {
                                let error_type = error.get_str("type").unwrap_or("Unknown");
                                let error_message = error.get_str("message").unwrap_or("Unknown");
                                error!(error_type, error_message, "Received Error from AI Server")
                            } else { error!(?json, "Received unexpected data from AI Server") }
                        }
                    }
                }
            }
        }
    });

    let response_speed = plan.response_speed();
    let interval = Duration::from_secs_f32(1.0 / response_speed as f32);

    let initial_data = json!({"chat_info": {"uuid": chat_uuid, "name": chat_name}}).to_string();
    let initial_event = stream::once(async move { Ok(Event::default().data(initial_data)) });

    let mut first_token_time = None;

    let body_stream = stream::unfold(rx, move |mut rx| async move {
        match rx.recv().await {
            Some(StreamItem::Token(word)) => {
                tokio::time::sleep(interval).await;

                if first_token_time.is_none() {
                    first_token_time = Some(Instant::now());
                }

                let json_data = json!({"completions": {"content": word}}).to_string();
                Some((Ok(Event::default().data(json_data)), rx))
            },
            Some(StreamItem::Usage(mut usage_json)) => {
                let duration = first_token_time.unwrap_or(start_time).elapsed();
                usage_json["usage"]["latency_ms"] = Value::from(duration.as_millis() as u64);

                let json_data = usage_json.to_string();
                Some((Ok(Event::default().data(json_data)), rx))
            },
            None => None,
        }
    });

    Ok(Sse::new(initial_event.chain(body_stream)))
}

pub fn build_context_messages(chat_messages: &[Message], user_prompt: &str, context_window: i64) -> Vec<Value> {
    let mut history: Vec<&Message> = vec![];
    let mut history_tokens: i64 = 0;

    for msg in chat_messages.iter().rev() {
        let msg_tokens = *msg.tokens();

        if history_tokens + msg_tokens > context_window {
            break;
        }

        history_tokens += msg_tokens;
        history.push(msg);
    }

    history.reverse();

    let mut messages = vec![];

    for msg in history {
        messages.push(json!(msg));
    }

    messages.push(json!({
        "role": Sender::User,
        "content": user_prompt
    }));

    messages
}