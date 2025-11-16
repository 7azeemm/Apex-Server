use derive_new::new;
use getset::Getters;
use std::collections::HashMap;

#[derive(Default, Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct MayorInfo {
    mayor: Mayor,
    minister: Option<Mayor>,
    election: Option<Vec<(Mayor, Option<u64>)>>,
}

impl MayorInfo {
    pub fn update(&mut self, mayor: Mayor, minister: Option<Mayor>, election: Option<Vec<(Mayor, Option<u64>)>>) {
        self.mayor = mayor;
        self.minister = minister;
        self.election = election;
    }
}

#[derive(Default, Clone, Debug, new, Getters)]
#[getset(get = "pub")]
pub struct Mayor {
    name: String,
    perks: HashMap<String, String>,
}

#[derive(Debug, new)]
pub struct SBDate {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    min: i64,
    sec: i64,
}

impl SBDate {
    pub fn get_year(&self) -> i64 {
        self.year
    }
    pub fn get_month(&self) -> i64 {
        self.month
    }
    pub fn get_day(&self) -> i64 {
        self.day
    }
    pub fn get_hour(&self) -> i64 {
        self.hour
    }
    pub fn get_min(&self) -> i64 {
        self.min
    }
    pub fn get_sec(&self) -> i64 {
        self.sec
    }
}