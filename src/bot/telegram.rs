use chrono::{DateTime, Local};
use crossbeam::channel::Receiver;
use log::{error, info};
use once_cell::sync::OnceCell;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::spawn;

use crate::{boardcast::BROADCAST_SENDER, config::CONFIG};

pub static STATUS: OnceCell<TGStatus> = OnceCell::new();

#[derive(Debug)]
pub struct TGStatus {
    pub start_at: DateTime<Local>,
    pub admin_chat_id: Vec<String>,
}

impl TGStatus {
    pub fn new() -> Self {
        let start_at = Local::now();
        TGStatus {
            start_at,
            admin_chat_id: vec!["768449054".into()],
        }
    }
}

#[derive(Debug)]
pub struct TelegramBot {
    bot: Bot,
    rx: Receiver<String>,
}

impl TelegramBot {
    pub fn new(rx: Receiver<String>) -> Self {
        let bot = Bot::from_env();
        TelegramBot { bot, rx }
    }

    pub async fn run(&self) {
        let dt = chrono::Local::now();
        self.boardcast(format!("Hello botte! {:?}", dt)).await;
        STATUS.set(TGStatus::new()).unwrap();
        println!("[bot] botte run");
        let b = self.bot.clone();

        spawn(async {
            Command::repl(b, answer).await;
        });
        self.repeat().await;
    }

    pub async fn boardcast(&self, msg: String) {
        let chat_ids = &CONFIG.telegram.allow_chat_id;
        for chat_id in chat_ids {
            self.send_msg(chat_id.clone(), &msg).await;
        }
    }

    pub async fn send_msg(&self, chat_id: String, message: &str) {
        self.bot.send_message(chat_id, message).await.unwrap();
    }

    async fn repeat(&self) {
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
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]

#[derive(Debug)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display current chat id.")]
    ChatId,
    #[command(description = "Up time.")]
    Uptime,
    #[command(description = "start the bot.")]
    Start,
    #[command(description = "mock recv alert msg, try to boardcast other rx")]
    Mock,

    Exit,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    info!("{:?}", msg);
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::ChatId => {
            bot.send_message(msg.chat.id, format!("Your chat id is: {}", msg.chat.id))
                .await?;
        }
        Command::Uptime => {
            let during = Local::now() - STATUS.get().unwrap().start_at;
            bot.send_message(msg.chat.id, format!("Up time: {:?}", during))
                .await?;
        }
        Command::Start => {
            bot.send_message(msg.chat.id, format!("这里是Botte, 你的 `chat id` 是: {}", msg.chat.id))
                .await?;
        },
        Command::Mock => {
            // get args
            let msg = msg.text().unwrap_or_else(|| "" ).to_string();
            // rm /mock prefix
            let msg = msg.trim_start_matches("/mock ").trim().to_string();
            info!("[bot] mock recv alert msg: {:?}", msg);
            BROADCAST_SENDER.get().unwrap().send(msg.to_string()).await.unwrap();
        },
        Command::Exit => {
            if msg.chat.id.to_string() == STATUS.get().unwrap().admin_chat_id[0] {
                info!("[bot] exit command received, shutting down...");
                bot.send_message(msg.chat.id, "Shutting down...").await?;
                std::process::exit(0);
            } else {
                bot.send_message(msg.chat.id, "You are not authorized to use this command.")
                    .await?;
                error!("Unauthorized exit command attempt from chat id: {}", msg.chat.id);
            }
        }
    };

    Ok(())
}
