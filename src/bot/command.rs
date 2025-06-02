use chrono::Local;
use log::{error, info};
use sysinfo::{Disks, Networks, Pid, System};
use teloxide::utils::markdown::escape;
use teloxide::{prelude::*, utils::command::BotCommands};

// use crate::mail::EMAIL_HISTORY;
use crate::G_TOKIO_RUNTIME;
use crate::boardcast::BROADCAST_SENDER;
use crate::bot::STATUS;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
#[derive(Debug)]
pub enum Command {
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
    #[command(description = "找到最占用CPU/Mem的进程")]
    Top,
    #[command(description = "查看进程信息")]
    Peek,
    // #[command(description = "查看邮件")]
    // Mails,
}

pub async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
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
            let output = output;
            let fmt = format!(
                "<b>{}@{}</b> &gt; <code>{}</code>\n<pre>{}</pre>",
                user, hostname, cmd, output
            );
            bot.send_message(msg.chat.id, fmt)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Command::Metrics => {
            let metric = metric();
            bot.send_message(msg.chat.id, metric)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Command::Top => {
            let top = top();
            info!("[bot] top command: {}", top);
            bot.send_message(msg.chat.id, top)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Command::Peek => {
            let arg1 = msg
                .text()
                .unwrap_or_else(|| "")
                .trim_start_matches("/peek ")
                .trim()
                .to_string();
            if arg1.is_empty() || arg1 == "/peek" {
                let top = top();
                bot.send_message(msg.chat.id, top)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                return Ok(());
            }
            let peek = peek(arg1);
            info!("[bot] peek command: {}", peek);
            bot.send_message(msg.chat.id, peek)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        },
        // Command::Mails => {
        //     let history = EMAIL_HISTORY.lock().unwrap();
        //     let mut response = String::from("<b>邮件历史记录：</b>\n\n");
        //     for (_, mail) in history.iter() {
        //         response.push_str(&format!("<b>From: {}\tTo: {}\tDate: {}\nSubject: {}</b>\n{}\n\n", mail.from, mail.to, mail.date, mail.subject, mail.content));
        //     }
        //     bot.send_message(msg.chat.id, "")
        //     //     .parse_mode(teloxide::types::ParseMode::Html)
        //         .await?;
        // }
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
    // keyword -> value
    let mut metrics: Vec<(&str, String)> = vec![];
    let mut system = System::new_all();
    system.refresh_all();

    // CPU 总占用率
    let total_cpu_usage: f32 = system.global_cpu_usage();
    metrics.push(("CPU", format!("{:.2}%", total_cpu_usage)));

    // 内存使用率
    let total_memory = system.total_memory() as f64 / 1_073_741_824.0; // 转换为GB
    let used_memory = system.used_memory() as f64 / 1_073_741_824.0; // 转换为GB
    let memory_usage = format!(
        "{:.2} GB / {:.2} GB ({:.2}%)",
        used_memory,
        total_memory,
        (used_memory / total_memory) * 100.0
    );
    metrics.push(("Mem", memory_usage));

    let network = Networks::new_with_refreshed_list();
    // 总网络IO
    let total_received: u64 = network.iter().map(|(_, data)| data.received()).sum();
    let total_transmitted: u64 = network.iter().map(|(_, data)| data.transmitted()).sum();
    let network_io = format!(
        "Rx {:.2} MB, Tx {:.2} MB",
        total_received as f64 / 1_048_576.0,    // 转换
        total_transmitted as f64 / 1_048_576.0  // 转换为MB
    );
    metrics.push(("Net", network_io));

    // 一个磁盘的使用情况
    let disk_info = if let Some(disk) = Disks::new_with_refreshed_list().get(0) {
        let used = (disk.total_space() - disk.available_space()) as f64;
        let total = disk.total_space() as f64;
        let percent = used / total * 100.0;
        format!(
            "{:.2} GB / {:.2} GB ({:.2}%)",
            used / 1_073_741_824.0,         // 转换为GB
            total as f64 / 1_073_741_824.0, // 转换为GB
            percent
        )
    } else {
        " No disk information available".to_string()
    };
    metrics.push(("Disk", disk_info));

    let tokio_met: tokio::runtime::RuntimeMetrics = G_TOKIO_RUNTIME.metrics();
    let tokio_info = format!(
        "{} tasks, {} alive, {} depth",
        tokio_met.num_workers(),
        tokio_met.num_alive_tasks(),
        tokio_met.global_queue_depth()
    );
    metrics.push(("Tokio-RT", tokio_info));

    // 获取程序自身占用内存
    let pname = std::env::current_exe()
        .map(|path| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|_| "botte".to_string());
    let pid = std::process::id();
    let process_memory = system
        .process(Pid::from_u32(pid))
        .map_or(0.0, |p| p.memory() as f64 / 1_048_576.0); // 转换为MB

    let process_memory_usage = format!("{:.2} MB in-use", process_memory);
    let kw = format!("{}@{}", pname, pid);
    metrics.push((&kw, process_memory_usage));

    return metrics
        .iter()
        .map(|(k, v)| format!("<b>{}</b>: {}", k, v))
        .collect::<Vec<String>>()
        .join("\n");
}

fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn top() -> String {
    let mut system = System::new_all();
    system.refresh_all();
    let mut processes: Vec<_> = system.processes().iter().collect();
    processes.sort_by(|(_, a), (_, b)| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap());
    let mut top = String::new();
    top.push_str("<b>Top 5 CPU:</b> \n");
    for (_pid, process) in processes.iter().take(5) {
        top.push_str(&format!(
            "{}: {:.1}%\n",
            process
                .exe()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default(),
            process.cpu_usage()
        ));
    }

    top.push_str("\n<b>Top 5 Mem:</b> \n");
    processes.sort_by(|(_, a), (_, b)| b.memory().partial_cmp(&a.memory()).unwrap());
    for (_pid, process) in processes.iter().take(5) {
        top.push_str(&format!(
            "{}: {:.2} GB\n",
            process
                .exe()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default(),
            process.memory() as f64 / 1_048_576.0 / 1024.0
        ));
    }

    top
}

fn peek(pname: String) -> String {
    let mut system = System::new_all();
    system.refresh_all();
    let processes: Vec<_> = system.processes().iter().collect();
    // 获取进程信息
    let p =processes
        .iter()
        .filter(|(_, p)| p.name().to_string_lossy().contains(&pname));

    let mut peek = String::new();
    for (pid, process) in p.into_iter() {
        if process.name().to_string_lossy().contains(&pname) {
            peek.push_str(&format!("<b>{:#?}</b>: \n", process.name()));
            peek.push_str(&format!("CPU: {:.1}%\n", process.cpu_usage()));
            peek.push_str(&format!(
                "Mem: {:.2} GB\n",
                process.memory() as f64 / 1_048_576.0 / 1024.0
            ));
            peek.push_str(&format!("PID: {}\n\n", pid.as_u32()));
        }
    }
    peek
}
