use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct MayorInfo {
    mayor: Mayor,
    minister: Option<Mayor>,
    election: Option<Vec<(Mayor, Option<u64>)>>, // <mayor, Option<votes>>
}

#[derive(Clone, Debug)]
pub struct Mayor {
    name: String,
    perks: HashMap<String, String>,
}

impl MayorInfo {
    pub fn new(mayor: Mayor, minister: Option<Mayor>, election: Option<Vec<(Mayor, Option<u64>)>>) -> Self {
        Self { mayor, minister, election }
    }

    pub fn update(&mut self, mayor: Mayor, minister: Option<Mayor>, election: Option<Vec<(Mayor, Option<u64>)>>) {
        self.mayor = mayor;
        self.minister = minister;
        self.election = election;
    }

    pub fn empty() -> Self {
        Self { mayor: Mayor::empty(), minister: None, election: None }
    }

    pub fn get_mayor(&self) -> &Mayor { &self.mayor }
    pub fn get_minister(&self) -> &Option<Mayor> { &self.minister }
    pub fn get_election(&self) -> &Option<Vec<(Mayor, Option<u64>)>> { &self.election }
}

impl Mayor {
    pub fn new(name: String, perks: HashMap<String, String>) -> Self {
        Self { name, perks }
    }

    fn empty() -> Self {
        Self { name: String::default(), perks: HashMap::default() }
    }

    pub fn get_name(&self) -> &str { &self.name }
    pub fn get_perks(&self) -> &HashMap<String, String> { &self.perks }
}

#[derive(Debug)]
pub struct SBDate {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    min: i64,
    sec: i64,
}

impl SBDate {
    pub fn new(year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64) -> Self {
        Self { year, month, day, hour, min, sec }
    }

    pub fn get_year(&self) -> i64 { self.year }
    pub fn get_month(&self) -> i64 { self.month }
    pub fn get_day(&self) -> i64 { self.day }
    pub fn get_hour(&self) -> i64 { self.hour }
    pub fn get_min(&self) -> i64 { self.min }
    pub fn get_sec(&self) -> i64 { self.sec }
}