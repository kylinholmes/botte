use std::fmt;

use log::{error, info};
use teloxide::prelude::*;
use crossbeam::channel::Receiver;

use crate::config::CONFIG;


#[derive(Debug)]
pub struct TelegramBot {
    bot: Bot,
    rx: Receiver<String>,
}

impl TelegramBot {
    pub fn new(rx: Receiver<String>) -> Self {
        let bot = Bot::from_env();
        TelegramBot {
            bot,
            rx,
        }
    }

    pub async fn run(&self) {
        self.send_msg("768449054".into(), "Hello, botte!").await;
        println!("[bot] botte run");
        let stream = self.rx.clone();
        loop {
            match stream.recv() {
                Ok(msg) => {
                    // Handle the message
                    info!("Recv: {}", msg);
                    self.boardcast(msg).await;
                }
                Err(_) => {
                    error!("Error receiving message");
                }
            }
        }
    }

    async fn boardcast(&self, msg: String) {
        let chat_ids = &CONFIG.allow_chat_id;
        for chat_id in chat_ids {
            self.send_msg(chat_id.clone(), &msg).await;
        }
    }

    async fn send_msg(&self, chat_id: String, message: &str) {
        self.bot.send_message(chat_id, message).await.unwrap();
    }
}
