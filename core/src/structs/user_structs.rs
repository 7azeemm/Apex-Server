use chrono::Utc;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct User {
    player_uuid: String,
    player_name: String,
    plan: Plan,
    plan_owned_at: i64,
    used_tokens_today: i64,
    total_tokens_used: i64,
    history: Vec<PlanHistory>,
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
        self.used_tokens_today >= self.plan.daily_max_tokens()
    }
    pub fn update_usage_token(&mut self, tokens: i64) {
        self.used_tokens_today += tokens;
        self.total_tokens_used += tokens;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanHistory {
    plan: Plan,
    owned_at: i64,
    duration: Option<i64>,
    requests_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Plan {
    Free,
    Plus,
    Pro,
}

impl Plan {
    pub fn daily_max_tokens(&self) -> i64 {
        match self {
            Plan::Free => 15_000,
            Plan::Plus => 75_000,
            Plan::Pro => 225_000,
        }
    }

    pub fn duration(&self) -> Option<i64> {
        match self {
            Plan::Free => None,
            Plan::Plus => Some(30 * 24 * 60 * 60),
            Plan::Pro => Some(30 * 24 * 60 * 60),
        }
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl FromStr for Plan {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Free" => Ok(Plan::Free),
            "Plus" => Ok(Plan::Plus),
            "Pro" => Ok(Plan::Pro),
            _ => Err(()),
        }
    }
}
