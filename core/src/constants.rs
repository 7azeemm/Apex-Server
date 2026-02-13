use crate::models::plan::{Plan, PlanConfig};

pub const AI_SERVER_IP: &str = "http://127.0.0.1:9000";

pub const CACHED_TOKENS_RATE: f32 = 0.25;

pub const MAINTENANCE: bool = false;
pub const MIN_VERSION: &str = "1.0.0";
pub const PLAN_CONFIGS: [PlanConfig; 3] = [
    PlanConfig {
        plan: Plan::Free,
        daily_tokens: 16_000,
        context_window: 4096,
        duration: None,
        color: 0xFFAFAFAF,
        response_speed: 50
    },
    PlanConfig {
        plan: Plan::Pro,
        daily_tokens: 50_000,
        context_window: 8192,
        duration: Some(2592000),
        color: 0xFF3B82F6,
        response_speed: 100
    },
    PlanConfig {
        plan: Plan::Ultimate,
        daily_tokens: 250_000,
        context_window: 16384,
        duration: Some(2592000),
        color: 0xFF7C3AED,
        response_speed: 100
    },
];