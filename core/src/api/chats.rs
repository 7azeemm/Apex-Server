use crate::utils::database::get_db_pool;
use crate::structs::chat::{Chat, ChatSummary, Message};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Extension;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use common::http::HTTP_CLIENT;
use crate::constants::AI_SERVER_IP;
use crate::structs::api_structs::ApiResponse;
use crate::structs::auth_structs::Session;

pub async fn get_chats(Extension(session): Extension<Arc<RwLock<Session>>>) -> Response {
    let session = session.read().await;
    let player_uuid = session.user().player_uuid().to_owned();

    let chats = match sqlx::query!(
        r#"
        SELECT id, chat_name, updated_at
        FROM chats
        WHERE player_uuid = $1
        ORDER BY updated_at DESC
        "#,
        player_uuid
    ).fetch_all(get_db_pool()).await {
        Ok(chats) => chats,
        Err(e) => return ApiResponse::internal_err("Failed to fetch chats", e, &session.context())
    };

    let chats: Vec<ChatSummary> = chats
        .into_iter()
        .map(|chat| {
            ChatSummary::new(
                chat.id,
                chat.chat_name,
                chat.updated_at,
            )
        })
        .collect();

    ApiResponse::ok(json!({"chats": chats}))
}

pub async fn create_chat(session: &Arc<RwLock<Session>>, prompt: &str) -> Result<Chat, Response> {
    let chat_name = generate_chat_title(prompt).await;

    let pool = get_db_pool();
    let current_time = Utc::now().timestamp();
    let chat_uuid = Uuid::new_v4().to_string();
    let player_uuid = session.read().await.user().player_uuid().clone();
    let messages = vec![];
    let tokens = 0;

    match sqlx::query!(
        r#"
        INSERT INTO chats (id, player_uuid, chat_name, messages, token_usage, updated_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        chat_uuid, player_uuid, chat_name, json!(messages),
        tokens, current_time, current_time
    ).execute(pool).await {
        Ok(_) => {
            let chat = Chat::new(chat_uuid.clone(), chat_name.to_owned(), messages, 0, current_time, current_time);
            let mut session = session.write().await;
            session.chats_mut().insert(chat_uuid, chat.clone());
            Ok(chat)
        }
        Err(e) => {
            let error_context = session.read().await.context();
            Err(ApiResponse::internal_err("Failed to create chat", e, &error_context))
        }
    }
}

pub async fn get_chat(Extension(session): Extension<Arc<RwLock<Session>>>, Path(chat_uuid): Path<String>) -> Response {
    fetch_chat(get_db_pool(), &session, &chat_uuid)
        .await
        .map_or_else(|e| e, ApiResponse::ok)
}

async fn fetch_chat(pool: &PgPool, session: &Arc<RwLock<Session>>, chat_uuid: &str) -> Result<Chat, Response> {
    let player_uuid = {
        let session = session.read().await;
        if let Some(chat) = session.chats().get(chat_uuid) {
            return Ok(chat.clone());
        }
        session.user().player_uuid().to_owned()
    };

    let chat = match sqlx::query!(
        r#"
        SELECT id, chat_name, messages, token_usage, updated_at, created_at
        FROM chats
        WHERE id = $1 AND player_uuid = $2
        "#,
        chat_uuid, player_uuid
    ).fetch_optional(pool).await {
        Ok(Some(chat)) => chat,
        Ok(None) => return Err(chat_err_resp(session, chat_uuid, "Chat not found", "", true).await),
        Err(e) => return Err(chat_err_resp(session, chat_uuid, "Failed to fetch chat", &e.to_string(), false).await)
    };

    let messages = match serde_json::from_value::<Vec<Message>>(chat.messages) {
        Ok(messages) => messages,
        Err(e) => return Err(chat_err_resp(session, chat_uuid, "Failed to fetch chat", &format!("Failed to parse messages: {e}"), false).await)
    };

    let chat = Chat::new(
        chat_uuid.to_owned(),
        chat.chat_name,
        messages,
        chat.token_usage,
        chat.updated_at,
        chat.created_at,
    );

    let mut session = session.write().await;
    session.chats_mut().insert(chat_uuid.to_owned(), chat.clone());

    Ok(chat)
}

pub async fn get_chat_or_create(session: &Arc<RwLock<Session>>, chat_uuid: String, prompt: &str, retry: bool) -> Result<Chat, Response> {
    match chat_uuid == "new" {
        true => create_chat(session, prompt).await,
        false => {
            let chat = fetch_chat(get_db_pool(), session, &chat_uuid).await?;
            match retry {
                true => remove_last_round(session, &chat).await,
                false => Ok(chat),
            }
        }
    }
}

pub async fn update_chat(session: &Arc<RwLock<Session>>, chat: &Chat, new_messages: Vec<Message>, tokens_usage: i64) {
    let player_uuid = session.read().await.user().player_uuid().to_owned();

    let mut chat = chat.clone();
    let chat_uuid = chat.uuid().to_owned();
    let messages_json = serde_json::to_value(new_messages.clone()).unwrap();
    for message in new_messages {
        chat.add_message(message);
    }

    match sqlx::query!(
        r#"
        WITH chat_update AS (
            UPDATE chats
            SET messages = messages || $1::jsonb,
                token_usage = $2,
                updated_at = $3
            WHERE id = $4 AND player_uuid = $5
        )
        UPDATE users
        SET tokens_used_today = tokens_used_today + $6,
            tokens_used_total = tokens_used_total + $6
        WHERE player_uuid = $5
        "#,
        messages_json, chat.token_usage(), chat.updated_at(),
        chat_uuid, player_uuid, tokens_usage
    ).execute(get_db_pool()).await {
        Err(e) => { chat_err_resp(session, &chat_uuid, "Failed to save chat and update tokens", &e.to_string(), false).await; },
        Ok(_) => {
            let mut session = session.write().await;
            session.user_mut().update_token_usage(tokens_usage);
            session.chats_mut().insert(chat_uuid, chat);
        }
    }
}

pub async fn remove_last_round(session: &Arc<RwLock<Session>>, chat: &Chat) -> Result<Chat, Response> {
    let messages_len = chat.messages().len();

    if messages_len < 2 {
        let error_context = session.read().await.context();
        return Err(ApiResponse::err_and_log(
            "No messages to remove",
            StatusCode::BAD_REQUEST,
            format!("Chat has {messages_len} message"),
            &error_context,
        ));
    }

    let chat_uuid = chat.uuid();
    let player_uuid = session.read().await.user().player_uuid().to_owned();

    match sqlx::query!(
        r#"
        UPDATE chats
        SET messages = (messages - -1) - -1,
            updated_at = $1
        WHERE id = $2 AND player_uuid = $3
        "#,
        Utc::now().timestamp(), chat_uuid, player_uuid
    ).execute(get_db_pool()).await {
        Err(e) => Err(chat_err_resp(session, chat_uuid, "Failed to remove last two messages", &e.to_string(), false).await),
        Ok(_) => {
            let mut chat = chat.clone();
            chat.remove_last_round();
            let mut session = session.write().await;
            session.chats_mut().insert(chat_uuid.to_owned(), chat.clone());
            Ok(chat)
        }
    }
}

pub async fn delete_chat(Extension(session): Extension<Arc<RwLock<Session>>>, Path(chat_uuid): Path<String>) -> Response {
    let player_uuid = session.read().await.user().player_uuid().to_owned();

    match sqlx::query!(
        r#"
        DELETE FROM chats
        WHERE id = $1 AND player_uuid = $2
        "#,
        chat_uuid, player_uuid
    ).execute(get_db_pool()).await {
        Ok(result) if result.rows_affected() == 0 => return chat_err_resp(&session, &chat_uuid, "Chat not found", "", true).await,
        Err(e) => return chat_err_resp(&session, &chat_uuid, "Failed to delete chat", &e.to_string(), false).await,
        Ok(_) => {}
    };

    let mut session = session.write().await;
    session.chats_mut().remove(&chat_uuid);

    ApiResponse::ok(())
}

pub async fn chat_err_resp(
    session: &Arc<RwLock<Session>>,
    chat_uuid: &str,
    msg: &str,
    error: &str,
    chat_not_found: bool,
) -> Response {
    let mut error_context = session.read().await.context();
    error_context.push(("chat_uuid", chat_uuid.to_owned()));
    match chat_not_found {
        false => ApiResponse::internal_err(msg, error, &error_context),
        true => ApiResponse::err_and_log(msg, StatusCode::NOT_FOUND, error, &error_context),
    }
}

async fn generate_chat_title(prompt: &str) -> String {
    let response = match HTTP_CLIENT
        .post(AI_SERVER_IP.to_owned() + "/generate_title")
        .json(&json!({ "prompt": prompt }))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return "New Chat".to_string(),
    };

    match response.json::<String>().await {
        Ok(title) if !title.trim().is_empty() => title,
        _ => "New Chat".to_string(),
    }
}