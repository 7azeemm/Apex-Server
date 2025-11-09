use crate::endpoints::chats::{add_message, get_chat_or_create};
use crate::structs::auth_structs::{ApiResponse, Session};
use crate::structs::chat_structs::{Message, Sender};
use crate::validated_json::ValidatedJson;
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::response::{Response, Sse};
use axum::Extension;
use common::extensions::json_ext::JsonExt;
use futures::StreamExt;
use futures::{stream, Stream};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

const AI_SERVER_CHAT_ENDPOINT: &str = "http://127.0.0.1:8001/chat";
const MAX_MESSAGES: usize = 10;

#[derive(Debug, Deserialize)]
pub struct CompletionsRequest {
    chat_uuid: String,
    prompt: String,
    retry: Option<bool>,
}

pub async fn completions_handler(
    Extension(session): Extension<Arc<RwLock<Session>>>,
    ValidatedJson(request): ValidatedJson<CompletionsRequest>,
) -> Result<Sse<impl Stream<Item=Result<Event, Infallible>>>, Response> {
    {
        let session = session.read().await;
        let user = session.user();
        if user.exceeded_limit() {
            return Err(ApiResponse::err(
                "Daily token limit reached",
                StatusCode::PAYMENT_REQUIRED,
            ));
        }
    }

    let chat_uuid = request.chat_uuid;
    let prompt = request.prompt;
    let message = Message::new(Sender::User, prompt.clone());

    let chat = get_chat_or_create(&session, chat_uuid, message, request.retry.unwrap_or(false)).await?;
    let chat_uuid = chat.uuid().to_owned();
    let chat_name = chat.name().to_owned();

    let total_messages = chat.messages().len();
    let start = total_messages.saturating_sub(MAX_MESSAGES);
    let recent_messages = &chat.messages()[start..];

    let mut messages_json = vec![json!({"role": "system", "content": "You are a helpful assistant."})];

    messages_json.extend(recent_messages.iter().map(|msg| {
        let role = match msg.sender() {
            Sender::User => "user",
            Sender::Assistant => "assistant",
        };
        json!({"role": role, "content": msg.content()})
    }));

    let client = Client::new();
    let response = client
        .post(AI_SERVER_CHAT_ENDPOINT)
        .json(&json!({ "messages": messages_json }))
        .send()
        .await
        .map_err(|e| {
            ApiResponse::internal_err("Failed to connect to the LLM".to_string(), e, &[])
        })?;

    let initial_event = stream::once(async move {
        let data = json!({"chat_info": {"uuid": chat_uuid, "name": chat_name}}).to_string();
        Ok::<Event, Infallible>(Event::default().data(data))
    });

    let usage_data = Arc::new(Mutex::new(None::<Value>));
    let collected = Arc::new(Mutex::new(String::new()));

    let usage_ref = usage_data.clone();
    let collected_ref = collected.clone();

    let body_stream = response.bytes_stream().then(move |chunk| {
        let usage_ref = usage_ref.clone();
        let collected_ref = collected_ref.clone();

        async move {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).to_string();

                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if json.get("usage").is_some() {
                            *usage_ref.lock().await = Some(json);
                        } else {
                            let content = json.get_str("completions/content");
                            collected_ref.lock().await.push_str(content.unwrap_or_default());
                        }
                    }

                    Ok(Event::default().data(text))
                }
                Err(err) => {
                    eprintln!("Error while streaming: {err}");
                    Ok(Event::default())
                }
            }
        }
    });

    let final_event = stream::once(async move {
        match usage_data.lock().await.take() {
            None => eprintln!("Run usage is not available???"),
            Some(usage) => {
                let collected_text = collected.lock().await.clone();
                let response = Message::new(Sender::Assistant, collected_text);
                let usage = usage.get("usage").unwrap_or_default();
                let prompt_tokens = usage.get_u64("prompt_tokens").unwrap_or_default();
                let completion_tokens = usage.get_u64("completion_tokens").unwrap_or_default();
                let total_tokens = prompt_tokens + completion_tokens;
                let _ = add_message(&session, &chat, response, total_tokens as i64).await;
            }
        }

        Ok(Event::default().data("[DONE]"))
    });

    let stream = initial_event.chain(body_stream).chain(final_event);

    Ok(Sse::new(stream))
}
