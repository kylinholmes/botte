use std::thread;

use log::info;
use once_cell::sync::OnceCell;
pub mod telegram;
pub mod traits;
use crossbeam::channel::{Sender, bounded};

pub static BOTS_TX: OnceCell<Sender<String>> = OnceCell::new();

pub fn run_bots() {
    let (tx, rx) = bounded(64);
    let _ = BOTS_TX.set(tx);
    thread::Builder::new()
        .name("bot-runtime".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let bot = telegram::TelegramBot::new(rx);
                bot.run().await;
                info!("[bot] botte runs");
            });
        })
        .unwrap();
}
