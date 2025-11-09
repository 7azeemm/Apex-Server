use crate::database::get_db_pool;
use crate::endpoints::users::update_user_token_usage;
use crate::structs::auth_structs::{ApiResponse, Session};
use crate::structs::chat_structs::{Chat, ChatSummary, Message};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Extension;
use chrono::Utc;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub async fn get_chats(Extension(session): Extension<Arc<RwLock<Session>>>) -> Response {
    let player_uuid = session.read().await.user().player_uuid().to_owned();

    let pool = get_db_pool();
    let chats = match sqlx::query(
        r#"
        SELECT chat_uuid, chat_name, updated_at
        FROM chats
        WHERE user_uuid = $1
        ORDER BY updated_at DESC
        "#,
    )
        .bind(&player_uuid)
        .fetch_all(pool)
        .await
    {
        Ok(chats) => chats,
        Err(e) => {
            let error_context = session.read().await.context();
            return ApiResponse::internal_err("Failed to fetch chats", e, &error_context);
        }
    };

    let chats: Vec<ChatSummary> = chats
        .into_iter()
        .map(|chat| {
            ChatSummary::new(
                chat.get("chat_uuid"),
                chat.get("chat_name"),
                chat.get("updated_at"),
            )
        })
        .collect();

    ApiResponse::ok(json!({"chats": chats}))
}

pub async fn create_chat(session: &Arc<RwLock<Session>>, message: Message) -> Result<Chat, Response> {
    let pool = get_db_pool();
    let current_time = Utc::now().timestamp();
    let chat_uuid = Uuid::new_v4().to_string();
    let player_uuid = session.read().await.user().player_uuid().clone();
    let chat_name = message.content().split_whitespace().take(4).collect::<Vec<&str>>().join(" ");
    let messages = vec![message];
    let tokens = 0;

    match sqlx::query(
        r#"
        INSERT INTO chats (chat_uuid, user_uuid, chat_name, messages, token_usage, updated_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#)
        .bind(&chat_uuid)
        .bind(&player_uuid)
        .bind(&chat_name)
        .bind(json!(messages))
        .bind(tokens)
        .bind(current_time)
        .bind(current_time)
        .execute(pool)
        .await
    {
        Ok(_) => {
            let chat = Chat::new(chat_uuid.clone(), chat_name.to_owned(), messages, tokens, current_time, current_time);
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

    let chat = match sqlx::query(
        r#"
        SELECT chat_uuid, chat_name, messages, token_usage, updated_at, created_at
        FROM chats
        WHERE chat_uuid = $1 AND user_uuid = $2
        "#,
    )
        .bind(chat_uuid)
        .bind(player_uuid)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(chat)) => chat,
        Ok(None) => return Err(chat_err_resp(session, chat_uuid, "Chat not found", "Shouldn't happen (While trying to fetch chat)", true).await),
        Err(e) => return Err(chat_err_resp(session, chat_uuid, "Failed to fetch chat", &e.to_string(), false).await)
    };

    let messages = match serde_json::from_value::<Vec<Message>>(chat.get("messages")) {
        Ok(messages) => messages,
        Err(e) => return Err(chat_err_resp(session, chat_uuid, "Failed to fetch chat", &format!("Failed to parse messages: {e}"), false).await)
    };

    let chat = Chat::new(
        chat_uuid.to_owned(),
        chat.get("chat_name"),
        messages,
        chat.get("token_usage"),
        chat.get("updated_at"),
        chat.get("created_at"),
    );

    let mut session = session.write().await;
    session.chats_mut().insert(chat_uuid.to_owned(), chat.clone());

    Ok(chat)
}

pub async fn get_chat_or_create(session: &Arc<RwLock<Session>>, chat_uuid: String, message: Message, retry: bool) -> Result<Chat, Response> {
    match chat_uuid == "new" {
        true => create_chat(session, message).await,
        false => {
            let chat = fetch_chat(get_db_pool(), session, &chat_uuid).await?;
            match retry {
                true => remove_last_message(session, &chat).await,
                false => add_message(session, &chat, message, 0).await,
            }
        }
    }
}

pub async fn add_message(session: &Arc<RwLock<Session>>, chat: &Chat, message: Message, tokens: i64) -> Result<Chat, Response> {
    let player_uuid = session.read().await.user().player_uuid().to_owned();
    let pool = get_db_pool();

    let mut chat = chat.clone();
    let chat_uuid = chat.uuid().to_owned();
    chat.add_message(message.clone(), tokens);
    let new_message = serde_json::to_value(vec![message]).unwrap();

    match sqlx::query(
        r#"
        UPDATE chats
        SET messages = messages || $1::jsonb,
            token_usage = $2,
            updated_at = $3
        WHERE chat_uuid = $4 AND user_uuid = $5
        "#,
    )
        .bind(new_message)
        .bind(chat.token_usage())
        .bind(chat.updated_at())
        .bind(&chat_uuid)
        .bind(player_uuid)
        .execute(pool)
        .await
    {
        Ok(_) => {
            if tokens > 0 {
                update_user_token_usage(session, tokens).await
            }
            let mut session = session.write().await;
            session.chats_mut().insert(chat_uuid, chat.clone());
            Ok(chat)
        }
        Err(e) => Err(chat_err_resp(session, &chat_uuid, "Failed to add message", &e.to_string(), false).await),
    }
}

pub async fn remove_last_message(session: &Arc<RwLock<Session>>, chat: &Chat) -> Result<Chat, Response> {
    let messages = chat.messages();

    if messages.is_empty() {
        let error_context = session.read().await.context();
        return Err(ApiResponse::err_and_log(
            "No messages to remove",
            StatusCode::BAD_REQUEST,
            "Chat has no messages",
            &error_context,
        ));
    }

    let chat_uuid = chat.uuid();
    let player_uuid = session.read().await.user().player_uuid().to_owned();
    let pool = get_db_pool();

    match sqlx::query(
        r#"
        UPDATE chats
        SET messages = messages - -1,
            updated_at = $1
        WHERE chat_uuid = $2 AND user_uuid = $3
        "#,
    )
        .bind(Utc::now().timestamp())
        .bind(chat_uuid)
        .bind(&player_uuid)
        .execute(pool)
        .await
    {
        Ok(_) => {
            let mut chat = chat.clone();
            chat.remove_last_message();
            let mut session = session.write().await;
            session.chats_mut().insert(chat_uuid.to_owned(), chat.clone());
            Ok(chat)
        }
        Err(e) => Err(chat_err_resp(session, chat_uuid, "Failed to remove last message", &e.to_string(), false).await),
    }
}

pub async fn delete_chat(Extension(session): Extension<Arc<RwLock<Session>>>, Path(chat_uuid): Path<String>) -> Response {
    let player_uuid = session.read().await.user().player_uuid().to_owned();
    let pool = get_db_pool();

    let result = match sqlx::query(
        r#"
        DELETE FROM chats
        WHERE chat_uuid = $1 AND user_uuid = $2
        "#,
    )
        .bind(&chat_uuid)
        .bind(&player_uuid)
        .execute(pool)
        .await
    {
        Ok(result) => result,
        Err(e) => return chat_err_resp(&session, &chat_uuid, "Failed to delete chat", &e.to_string(), false).await
    };

    // Check if any rows were affected
    if result.rows_affected() == 0 {
        return chat_err_resp(&session, &chat_uuid, "Chat not found", "Shouldn't happen (while trying to delete chat)", true).await;
    }

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
