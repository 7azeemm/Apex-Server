use crate::database::get_db_pool;
use crate::structs::auth_structs::{ApiResponse, Session};
use crate::structs::user_structs::{Plan, PlanHistory, User};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_user_or_create(player_uuid: String, player_name: String) -> Result<User, (String, String)> {
    let pool = get_db_pool();

    match sqlx::query(
        r#"
        SELECT *
        FROM users
        WHERE player_uuid = $1
        "#,
    )
        .bind(&player_uuid)
        .fetch_optional(pool)
        .await
    {
        Ok(None) => create_user(pool, player_uuid, player_name).await,
        Ok(Some(user)) => {
            let plan_str: String = user.get("plan");
            let plan = match Plan::from_str(&plan_str) {
                Ok(p) => p,
                Err(_) => return Err((
                    "Failed to create user".to_owned(),
                    format!("Failed to deserialize plan, plan: {}", plan_str),
                ))
            };
            let history_json: serde_json::Value = user.get("history");
            let history: Vec<PlanHistory> = match serde_json::from_value(history_json) {
                Ok(h) => h,
                Err(e) => return Err((
                    "Failed to create user".to_owned(),
                    format!("Failed to deserialize history: {}", e),
                ))
            };

            Ok(User::new(
                user.get("player_uuid"),
                user.get("player_name"),
                plan,
                user.get("plan_owned_at"),
                user.get("used_tokens_today"),
                user.get("total_tokens_used"),
                history,
                user.get("created_at"),
            ))
        }
        Err(e) => Err(("Failed to fetch user".to_owned(), format!("{}", e))),
    }
}

pub async fn create_user(pool: &PgPool, player_uuid: String, player_name: String) -> Result<User, (String, String)> {
    let new_user = User::create(player_uuid.clone(), player_name.clone(), Plan::Free);
    let history = match serde_json::to_value(new_user.history()) {
        Err(e) => return Err((
            "Failed to create user".to_owned(),
            format!("Failed to serialize history: {}", e),
        )),
        Ok(h) => h,
    };

    let result = sqlx::query(
        r#"
            INSERT INTO users (
                player_uuid, player_name, plan, plan_owned_at,
                used_tokens_today, total_tokens_used, history, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
        .bind(new_user.player_uuid())
        .bind(new_user.player_name())
        .bind(new_user.plan().to_string())
        .bind(new_user.plan_owned_at())
        .bind(new_user.used_tokens_today())
        .bind(new_user.total_tokens_used())
        .bind(history)
        .bind(new_user.created_at())
        .execute(pool)
        .await;

    if let Err(e) = result {
        return Err(("Failed to create user".to_owned(), format!("{}", e)));
    }

    println!("Created User {} in database", player_name);
    Ok(new_user)
}

pub async fn update_user_token_usage(session: &Arc<RwLock<Session>>, tokens_used: i64) {
    let pool = get_db_pool();
    let player_uuid = session.read().await.user().player_uuid().clone();

    match sqlx::query(
        r#"
        UPDATE users
        SET used_tokens_today = used_tokens_today + $1,
            total_tokens_used = total_tokens_used + $2
        WHERE player_uuid = $3
        "#,
    )
        .bind(tokens_used)
        .bind(tokens_used)
        .bind(player_uuid)
        .execute(pool)
        .await
    {
        Ok(_) => {
            let mut session = session.write().await;
            session.user_mut().update_usage_token(tokens_used);
        }
        Err(e) => {
            ApiResponse::internal_err(
                "Failed to update user token usage",
                e,
                &session.read().await.context(),
            );
        }
    }
}
