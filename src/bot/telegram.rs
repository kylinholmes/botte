use chrono::{DateTime, Local};
use crossbeam::channel::Receiver;
use log::{error, info};
use once_cell::sync::OnceCell;
use sysinfo::{Disks, Networks, System};
use teloxide::{prelude::*, utils::command::BotCommands};
use teloxide::utils::markdown::escape;
use tokio::spawn;

use crate::{G_TOKIO_RUNTIME, boardcast::BROADCAST_SENDER, config::CONFIG};

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
    #[command(description = "退出 Botte 进程")]
    Exit,
    #[command(description = "执行shell命令")]
    Shell,

    #[command(description = "测量性能")]
    Metrics,
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
            bot.send_message(
                msg.chat.id,
                format!("这里是Botte, 你的 `chat id` 是: {}", msg.chat.id),
            )
            .await?;
        }
        Command::Mock => {
            // get args
            let id = msg.chat.id;
            let msg = msg.text().unwrap_or_else(|| "").to_string();
            // rm /mock prefix
            let msg = msg.trim_start_matches("/mock ").trim().to_string();
            if msg.is_empty() || msg == "/mock" {
                bot.send_message(id, "Please provide a message to mock.")
                    .await?;
                return Ok(());
            }
            info!("[bot] mock recv alert msg: {:?}", msg);
            BROADCAST_SENDER
                .get()
                .unwrap()
                .send(msg.to_string())
                .await
                .unwrap();
        }
        Command::Exit => {
            if msg.chat.id.to_string() == STATUS.get().unwrap().admin_chat_id[0] {
                info!("[bot] exit command received, shutting down...");
                bot.send_message(msg.chat.id, "Shutting down...").await?;
                std::process::exit(0);
            } else {
                bot.send_message(msg.chat.id, "You are not authorized to use this command.")
                    .await?;
                error!(
                    "Unauthorized exit command attempt from chat id: {}",
                    msg.chat.id
                );
            }
        }
        Command::Shell => {
            let user = msg.chat.id.to_string();
            if user != STATUS.get().unwrap().admin_chat_id[0] {
                bot.send_message(msg.chat.id, "You are not authorized to use this command.")
                    .await?;
                error!(
                    "Unauthorized shell command attempt from chat id: {}",
                    msg.chat.id
                );
                return Ok(());
            }

            let user = msg.chat.username().unwrap_or("unknown");
            let hostname = get_hostname();
            let cmd = msg.text().unwrap_or_else(|| "").to_string();
            // rm /shell prefix
            let cmd = cmd.trim_start_matches("/shell ").trim().to_string();
            if cmd.is_empty() || cmd == "/shell" {
                bot.send_message(msg.chat.id, "Please provide a shell command.")
                    .await?;
                return Ok(());
            }
            info!("[bot] shell command: {}", cmd);
            let output = run_shell(cmd.clone());
            let cmd = escape(&cmd);
            let output = escape(&output);
            let fmt = format!(
                "<b>{}@{}</b> &gt; <code>{}</code>\n<pre>{}</pre>", 
                user, hostname, 
                cmd, output
            );
            bot.send_message(msg.chat.id, fmt)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Command::Metrics => {
            let metric = metric();
            bot.send_message(msg.chat.id, format!("{}", metric)).await?;
        }
    };

    Ok(())
}

fn run_shell(cmd: String) -> String {
    use std::process::Command;

    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    }
}

fn metric() -> String {
    let mut system = System::new_all();
    system.refresh_all();

    // CPU 总占用率
    let total_cpu_usage: f32 = system.global_cpu_usage();

    // 内存使用率
    let total_memory = system.total_memory() as f64 / 1_073_741_824.0; // 转换为GB
    let used_memory = system.used_memory() as f64 / 1_073_741_824.0; // 转换为GB
    let memory_usage = format!(
        "Memory: {:.2} GB / {:.2} GB ({:.2}%)",
        used_memory,
        total_memory,
        (used_memory / total_memory) * 100.0
    );

    let network = Networks::new_with_refreshed_list();
    // 总网络IO
    let total_received: u64 = network.iter().map(|(_, data)| data.received()).sum();
    let total_transmitted: u64 = network.iter().map(|(_, data)| data.transmitted()).sum();
    let network_io = format!(
        "Network: Received {:.2} MB, Transmitted {:.2} MB",
        total_received as f64 / 1_048_576.0,    // 转换为MB
        total_transmitted as f64 / 1_048_576.0  // 转换为MB
    );

    // 一个磁盘的使用情况
    let disk_info = if let Some(disk) = Disks::new_with_refreshed_list().get(0) {
        format!(
            "Disk: {} {:.2} GB free, {:.2} GB total",
            disk.name().to_string_lossy(),
            disk.available_space() as f64 / 1_073_741_824.0, // 转换为GB
            disk.total_space() as f64 / 1_073_741_824.0      // 转换为GB
        )
    } else {
        "Disk: No disk information available".to_string()
    };

    let tokio_met: tokio::runtime::RuntimeMetrics = G_TOKIO_RUNTIME.metrics();
    let tokio_info = format!(
        "Tokio Runtime: {} tasks, {} alive, {} depth",
        tokio_met.num_workers(),
        tokio_met.num_alive_tasks(),
        tokio_met.global_queue_depth()
    );

    // 拼接结果
    format!(
        "CPU Usage: {:.2}%\n{}\n{}\n{}\n{}",
        total_cpu_usage, memory_usage, network_io, disk_info, tokio_info
    )
}

fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
