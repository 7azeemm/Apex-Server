use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::error;
use crate::api::auth::add_pending_notification;
use crate::utils::database::get_db_pool;
use crate::structs::plan::Plan;

pub fn schedule() {
    tokio::spawn(async move {
        loop {
            if let Err(e) = expire_plans().await {
                error!("Plan expiration error: {e}");
            }

            sleep(Duration::from_secs(60)).await;
        }
    });
}

async fn expire_plans() -> Result<(), sqlx::Error> {
    let pool = get_db_pool();
    let now = chrono::Utc::now().timestamp();

    // Fetch all users who are not on Free plan
    let users = sqlx::query!(
        r#"
        SELECT player_uuid, plan, plan_started_at
        FROM users
        WHERE plan != 'Free'
        "#)
        .fetch_all(pool)
        .await?;

    for user in users {
        let Ok(plan) = Plan::from_str(&user.plan) else {
            error!(plan = user.plan,
                player_uuid = user.player_uuid,
                "Failed to deserialize plan from database"
            );
            continue;
        };
        let Some(duration) = plan.duration() else { continue };
        let expires_at = user.plan_started_at + duration;

        if now >= expires_at {
            let player_uuid = &user.player_uuid;
            
            sqlx::query(
                r#"
                UPDATE users
                SET
                    plan_history = plan_history || jsonb_build_object(
                        'plan', plan,
                        'started_at', plan_started_at,
                        'ended_at', $1
                    ),
                    plan = 'Free',
                    plan_started_at = $2
                WHERE player_uuid = $3
                "#)
                .bind(expires_at)
                .bind(now)
                .bind(player_uuid)
                .execute(pool)
                .await?;

            add_pending_notification(
                player_uuid,
                format!("Your Plan {plan} has been expired!")
            ).await;
        }
    }

    Ok(())
}