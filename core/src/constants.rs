use crate::structs::plan::{Plan, PlanConfig};

pub const CACHED_INPUT_TOKENS_RATE: f32 = 0.25;

pub const MAINTENANCE: bool = false;
pub const MIN_VERSION: &str = "1.0.0";
pub const PLAN_CONFIGS: [PlanConfig; 3] = [
    PlanConfig {
        plan: Plan::Free,
        daily_tokens: 15_000,
        context_window: 4096,
        duration: None,
        color: 0xFFA9A9A9,
        response_speed: 15
    },
    PlanConfig {
        plan: Plan::Pro,
        daily_tokens: 75_000,
        context_window: 8192,
        duration: Some(2592000),
        color: 0xFF00C8FF,
        response_speed: 40
    },
    PlanConfig {
        plan: Plan::Ultimate,
        daily_tokens: 225_000,
        context_window: 16384,
        duration: Some(2592000),
        color: 0xFFFFA800,
        response_speed: 40
    },
];