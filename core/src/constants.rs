use crate::structs::plan::{Plan, PlanConfig};

pub const MAINTENANCE: bool = false;
pub const MIN_VERSION: &str = "1.0.0";
pub const PLAN_CONFIGS: [PlanConfig; 3] = [
    PlanConfig { plan: Plan::Free, daily_tokens: 15_000, duration: None, color: 0 },
    PlanConfig { plan: Plan::Plus, daily_tokens: 75_000, duration: Some(2592000), color: 1 },
    PlanConfig { plan: Plan::Pro, daily_tokens: 225_000, duration: Some(2592000), color: 2 },
];

pub const CONTACTS: &[(&str, &str)] = &[
    ("Discord", "https://discord"),
    ("Github", "https://github"),
    ("Modrinth", "https://modrinth"),
];