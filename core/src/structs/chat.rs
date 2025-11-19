use chrono::Utc;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct Chat {
    uuid: String,
    name: String,
    messages: Vec<Message>,
    token_usage: i64,
    created_at: i64,
    updated_at: i64,
}

impl Chat {
    pub fn add_message(&mut self, message: Message) {
        self.token_usage += *message.tokens();
        self.messages.push(message);
        self.updated_at = Utc::now().timestamp();
    }

    pub fn remove_last_round(&mut self) {
        self.messages.pop();
        self.messages.pop();
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct Message {
    sender: Sender,
    content: String,
    tokens: i64
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sender {
    User,
    Assistant,
}

#[derive(Serialize, new)]
pub struct ChatSummary {
    chat_uuid: String,
    chat_name: String,
    updated_at: i64,
}