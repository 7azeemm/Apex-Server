use chrono::Utc;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::structs::plan::{ExpiredPlan, Plan};

#[derive(Debug, Serialize, Deserialize, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct User {
    player_uuid: String,
    player_name: String,
    plan: Plan,
    plan_started_at: i64,
    tokens_used_today: i64,
    tokens_used_total: i64,
    plan_history: Vec<ExpiredPlan>,
    created_at: i64,
}

impl User {
    pub fn create(player_uuid: String, player_name: String, plan: Plan) -> Self {
        let current_time = Utc::now().timestamp();
        Self::new(
            player_uuid, player_name, plan, current_time,
            0, 0, Vec::new(), current_time,
        )
    }

    pub fn exceeded_limit(&self) -> bool {
        self.tokens_used_today >= self.plan.daily_tokens()
    }
    pub fn update_token_usage(&mut self, tokens: i64) {
        self.tokens_used_today += tokens;
        self.tokens_used_total += tokens;
    }
    
    pub fn upgrade_plan(&mut self, new_plan: Plan) {
        let now = Utc::now().timestamp();

        if self.plan.to_string() != Plan::Free.to_string() {
            self.plan_history.push(ExpiredPlan::new(
                self.plan.clone(),
                self.plan_started_at,
                now
            ));
        }
        
        self.plan = new_plan;
        self.plan_started_at = now;
    }
}