use crate::utils::database::get_db_pool;
use crate::structs::auth_structs::Session;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;
use common::discord_logger;
use crate::api::auth::{add_pending_notification, remove_user_session};
use crate::structs::api_structs::ApiResponse;
use crate::structs::plan::{ExpiredPlan, Plan};
use crate::structs::user::User;

pub async fn get_or_create_user(player_uuid: String, player_name: String) -> Result<User, (String, String)> {
    let pool = get_db_pool();

    match sqlx::query!(
        r#"
        SELECT *
        FROM users
        WHERE player_uuid = $1
        "#, player_uuid
    ).fetch_optional(pool).await {
        Ok(None) => create_user(pool, player_uuid, player_name).await,
        Ok(Some(user)) => {
            let Ok(plan) = Plan::from_str(&user.plan) else {
                return Err((
                   "Failed to create user".to_owned(),
                   format!("Failed to deserialize plan {}", user.plan),
                ))
            };

            let plan_history: Vec<ExpiredPlan> = match serde_json::from_value(user.plan_history) {
                Ok(h) => h,
                Err(e) => return Err((
                    "Failed to create user".to_owned(),
                    format!("Failed to deserialize plan_history: {}", e),
                ))
            };

            Ok(User::new(
                user.player_uuid,
                user.player_name,
                plan,
                user.plan_started_at,
                user.tokens_used_today,
                user.tokens_used_total,
                plan_history,
                user.created_at,
            ))
        }
        Err(e) => Err(("Failed to fetch user".to_owned(), format!("{}", e))),
    }
}

pub async fn create_user(pool: &PgPool, player_uuid: String, player_name: String) -> Result<User, (String, String)> {
    let new_user = User::create(player_uuid.clone(), player_name.clone(), Plan::Free);
    let history = match serde_json::to_value(new_user.plan_history()) {
        Err(e) => return Err((
            "Failed to create user".to_owned(),
            format!("Failed to serialize plan_history: {}", e),
        )),
        Ok(h) => h,
    };

    let result = sqlx::query!(
        r#"
        INSERT INTO users (
            player_uuid, player_name, plan, plan_started_at,
            tokens_used_today, tokens_used_total, plan_history, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        new_user.player_uuid(), new_user.player_name(), new_user.plan().to_string(),
        new_user.plan_started_at(), new_user.tokens_used_today(), new_user.tokens_used_total(),
        history, new_user.created_at()
    ).execute(pool).await;

    if let Err(e) = result {
        return Err(("Failed to create user".to_owned(), format!("{}", e)));
    }

    discord_logger::log_new_user(&player_name);
    Ok(new_user)
}

pub async fn update_user_token_usage(session: &Arc<RwLock<Session>>, tokens_used: i64) {
    let player_uuid = session.read().await.user().player_uuid().clone();

    match sqlx::query!(
        r#"
        UPDATE users
        SET tokens_used_today = tokens_used_today + $1,
            tokens_used_total = tokens_used_total + $1
        WHERE player_uuid = $2
        "#,
        tokens_used, player_uuid
    ).execute(get_db_pool()).await {
        Ok(_) => {
            let mut session = session.write().await;
            session.user_mut().update_token_usage(tokens_used);
        }
        Err(err) => {
            let _ = ApiResponse::internal_err(
                "Failed to update user token usage",
                err,
                &session.read().await.context(),
            );
        }
    }
}

pub async fn upgrade_plan(player_uuid: &str, player_name: &str, new_plan: Plan) {
    remove_user_session(player_uuid).await;

    // Ensure user exists — this loads OR creates the user
    let mut user = match get_or_create_user(player_uuid.to_string(), player_name.to_string()).await {
        Ok(user) => user,
        Err((api_err, sys_err)) => {
            error!(api_err, sys_err, player_uuid, player_name, "Failed to upgrade plan!!!");
            return;
        }
    };

    let old_plan = user.plan().to_string();
    user.upgrade_plan(new_plan);

    let history_json = match serde_json::to_value(user.plan_history()) {
        Ok(v) => v,
        Err(error) => {
            error!(?error, "Failed to serialize plan_history");
            return;
        }
    };

    match sqlx::query!(
        r#"
        UPDATE users
        SET
            plan = $1,
            plan_started_at = $2,
            tokens_used_today = 0,
            plan_history = $3
        WHERE player_uuid = $4
        "#,
        new_plan.to_string(),
        user.plan_started_at(),
        history_json,
        player_uuid
    ).execute(get_db_pool()).await {
        Ok(_) => {
            add_pending_notification(
                player_uuid,
                format!("Your Plan has been upgraded from {old_plan} to {new_plan}")
            ).await;
            discord_logger::log_plan_upgrade(
                player_name,
                &old_plan.to_string(),
                &new_plan.to_string(),
            )
        },
        Err(err) => {
            let _ = ApiResponse::internal_err(
                "Failed to update user plan",
                err,
                &[
                    ("player_uuid", player_uuid.to_owned()),
                    ("player_name", player_name.to_owned()),
                    ("plan", new_plan.to_string())
                ],
            );
        }
    }
}