use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_hypixel_api_key() -> String {
    std::env::var("HYPIXEL_API_KEY").expect("Hypixel API key is not set in .env file")
}

pub fn get_time_as_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn format_number_with_commas(num: u64) -> String {
    let s = num.to_string();
    let mut result = String::new();

    let chars: Vec<char> = s.chars().rev().collect();
    for (i, c) in chars.iter().enumerate() {
        if i != 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result.chars().rev().collect()
}

pub fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}b", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
        .replace(".0", "")
        .to_string()
}

pub fn strip_formatting(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '§' {
            chars.next();
        } else {
            result.push(ch);
        }
    }

    result
}