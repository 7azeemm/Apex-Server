use std::fmt;
use std::str::FromStr;
use derive_new::new;
use serde::{Deserialize, Serialize};
use crate::constants::PLAN_CONFIGS;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Plan {
    Free,
    Pro,
    Ultimate,
}

#[derive(new)]
pub struct PlanConfig {
    pub plan: Plan,
    pub daily_tokens: i64,
    pub context_window: i64,
    pub duration: Option<i64>,
    pub color: i64,
    pub response_speed: u64
}

impl Plan {
    fn config(&self) -> &'static PlanConfig {
        PLAN_CONFIGS
            .iter()
            .find(|c| c.plan == *self)
            .expect("Plan config not found")
    }

    pub fn next_plan(&self) -> Option<Plan> {
        let index = PLAN_CONFIGS
            .iter()
            .position(|c| c.plan == *self)
            .expect("Plan config not found");

        PLAN_CONFIGS.get(index + 1).map(|c| c.plan)
    }

    pub fn daily_tokens(&self) -> i64 {
        self.config().daily_tokens
    }

    pub fn context_window(&self) -> i64 {
        self.config().context_window
    }

    pub fn duration(&self) -> Option<i64> {
        self.config().duration
    }

    pub fn color(&self) -> i64 {
        self.config().color
    }

    pub fn mc_color(&self) -> String {
        mc_hex_color(self.config().color)
    }

    pub fn response_speed(&self) -> u64 {
        self.config().response_speed
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
        PLAN_CONFIGS
            .iter()
            .find(|config| config.plan.to_string().eq_ignore_ascii_case(s))
            .map(|config| config.plan)
            .ok_or(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, new)]
pub struct ExpiredPlan {
    plan: Plan,
    started_at: i64,
    ended_at: i64,
}

pub fn mc_hex_color(color: i64) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let rgb = (color & 0xFFFFFF) as u32;
    let bytes = [
        (rgb >> 20) & 0xF,
        (rgb >> 16) & 0xF,
        (rgb >> 12) & 0xF,
        (rgb >> 8) & 0xF,
        (rgb >> 4) & 0xF,
        rgb & 0xF,
    ];

    let mut out = String::with_capacity(14);
    out.push('§');
    out.push('x');

    for &b in &bytes {
        out.push('§');
        out.push(HEX[b as usize] as char);
    }

    out
}
