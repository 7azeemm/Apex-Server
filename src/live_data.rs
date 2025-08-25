use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{interval, interval_at, Instant};
use crate::statics::HTTP_CLIENT;

const CONTESTS_ENDPOINT: &str = "https://api.elitebot.dev/contests/at/now";
const MAYOR_ENDPOINT: &str = "https://api.hypixel.net/v2/resources/skyblock/election";
const THRESHOLD: u64 = 300;

static DATA: LazyLock<RwLock<SkyBlockData>> = LazyLock::new(|| RwLock::new(SkyBlockData::empty()));

#[derive(Clone, Debug)]
struct SkyBlockData {
    year: u64,
    contests: HashMap<String, Vec<String>>,
    mayor_info: MayorInfo
}

#[derive(Clone, Debug)]
pub struct MayorInfo {
    mayor: Mayor,
    minister: Option<Mayor>,
    election: Option<Vec<(u64, Mayor)>> //<votes, mayor>
}

#[derive(Clone, Debug)]
pub struct Mayor {
    name: String,
    perks: HashMap<String, String>
}

impl SkyBlockData {
    fn empty() -> Self {
        Self { year: 0, contests: HashMap::default(), mayor_info: MayorInfo::empty() }
    }
}

impl MayorInfo {
    fn empty() -> Self {
        Self { mayor: Mayor::empty(), minister: None, election: None}
    }

    pub fn get_mayor(&self) -> &Mayor { &self.mayor }
    pub fn get_minister(&self) -> &Option<Mayor> { &self.minister }
    pub fn get_election(&self) -> &Option<Vec<(u64, Mayor)>> { &self.election }
}

impl Mayor {
    fn empty() -> Self {
        Self { name: String::default(), perks: HashMap::default() }
    }

    pub fn get_name(&self) -> &str { &self.name }
    pub fn get_perks(&self) -> &HashMap<String, String> { &self.perks }
}

pub fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update().await {
                Ok(()) => println!("[SkyBlock-Data] Next update in {} seconds", THRESHOLD),
                Err(err) => eprintln!("[SkyBlock-Data] Error: {:?}", err)
            }
        }
    });
}

async fn update() -> Result<(), Box<dyn std::error::Error>> {
    update_contests().await?;
    update_mayors_info().await?;

    Ok(())
}

async fn update_mayors_info() -> Result<(), Box<dyn Error>> {
    let resp = HTTP_CLIENT.get(MAYOR_ENDPOINT)
        .send().await?.text().await?;

    let json: serde_json::Value = serde_json::from_str(&resp)?;

    if !json["success"].as_bool().unwrap() {
        return Err("[SkyBlock-Data/Mayor_Info] API Request was not successful".into())
    }

    let mayor_data = &json["mayor"];

    // Parse current mayor
    let mayor = Mayor {
        name: mayor_data["name"].as_str().unwrap_or("Unknown").to_string(),
        perks: parse_perks(mayor_data["perks"].as_array()),
    };

    // Parse minister
    let minister = mayor_data["minister"].as_object()
        .map(|minister_data| {
            let mut perks = HashMap::new();
            if let Some(perk) = minister_data["perk"].as_object() {
                if let (Some(name), Some(desc)) = (perk["name"].as_str(), perk["description"].as_str()) {
                    perks.insert(name.to_string(), desc.to_string());
                }
            }

            Mayor {
                name: minister_data["name"].as_str().unwrap_or("Unknown").to_string(),
                perks,
            }
        });

    // Parse election candidates
    let election = mayor_data["election"].as_object()
        .and_then(|election_data| election_data["candidates"].as_array())
        .map(|candidates| {
            let mut candidates_vec: Vec<(u64, Mayor)> = candidates.iter()
                .filter_map(|candidate| {
                    let votes = candidate["votes"].as_u64()?;
                    let name = candidate["name"].as_str()?.to_string();
                    let perks = parse_perks(candidate["perks"].as_array());

                    Some((votes, Mayor { name, perks }))
                })
                .collect();

            // Sort by votes (highest first)
            candidates_vec.sort_by(|a, b| b.0.cmp(&a.0));
            candidates_vec
        });

    let mut data = DATA.write().await;
    data.mayor_info = MayorInfo {
        mayor,
        minister,
        election,
    };

    Ok(())
}

async fn update_contests() -> Result<(), Box<dyn Error>> {
    let resp = HTTP_CLIENT.get(CONTESTS_ENDPOINT)
        .send().await?.text().await?;

    let json: serde_json::Value = serde_json::from_str(&resp)?;

    let year = json["year"].as_u64().unwrap_or(0);
    let contests = json["contests"].as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let contests_vec = v.as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();
                    (k.clone(), contests_vec)
                })
                .collect::<HashMap<String, Vec<String>>>()
        })
        .unwrap_or_default();

    let mut data = DATA.write().await;
    data.year = year;
    data.contests = contests;

    Ok(())
}

fn parse_perks(perks_array: Option<&Vec<serde_json::Value>>) -> HashMap<String, String> {
    perks_array
        .map(|perks| {
            perks.iter()
                .filter_map(|perk| {
                    let name = perk["name"].as_str()?;
                    let description = perk["description"].as_str()?;
                    Some((name.to_string(), description.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub async fn get_skyblock_year() -> u64 {
    DATA.read().await.year
}

pub async fn get_upcoming_contests() -> Vec<(String, Vec<String>)> {
    let data = DATA.read().await;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut upcoming: Vec<_> = data.contests
        .iter()
        .filter_map(|(time_str, contests)| {
            if let Ok(event_time) = time_str.parse::<u64>() {
                if event_time > current_time {
                    Some((event_time, time_str.clone(), contests.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Sort by timestamp (earliest first)
    upcoming.sort_by_key(|(timestamp, _, _)| *timestamp);

    upcoming
        .into_iter()
        .take(5)
        .map(|(_, time_str, contests)| (time_str, contests))
        .collect()
}

pub async fn get_mayor_info() -> MayorInfo {
    let data = DATA.read().await;
    data.mayor_info.clone()
}