extern crate worker;

use std::collections::HashMap;
use worker::console_log;

pub struct TelegramClient {
    base_url: String,
    bot_api_token: String,
    bot_chat_id: String,
}

impl TelegramClient {
    pub fn new(bot_api_token: &str, bot_chat_id: &str) -> TelegramClient {
        TelegramClient {
            bot_api_token: bot_api_token.into(),
            bot_chat_id: bot_chat_id.into(),
            base_url: "https://api.telegram.org/".to_string(),
        }
    }

    pub async fn send_message(&self, message: &str) -> bool {
        let client = reqwest::Client::new();
        let mut map = HashMap::<&str, &str>::new();
        map.insert("chat_id", self.bot_chat_id.as_str());
        map.insert("text", message);
        let url = format!("{}bot{}/sendMessage", self.base_url, self.bot_api_token);

        match client.post(url).json(&map).send().await {
            Ok(_) => true,
            Err(error) => {
                console_log!("Telegram API response error: {error}");
                false
            }
        }
    }
}
