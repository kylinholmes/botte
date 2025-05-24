
use std::thread;

use crossbeam::channel::{Receiver, Sender};
use log::info;
use once_cell::sync::OnceCell;

use crate::{config::CONFIG, G_TOKIO_RUNTIME};

pub static HOOK_TX: OnceCell<Sender<String>> = OnceCell::new();

pub fn run_webhook() {
    if let Some(webhook) = (&CONFIG.webhook).clone() {
        info!("[webhook] webhook enabled, urls: {:?}", webhook.hook_urls);
        let (tx, rx) = crossbeam::channel::bounded(64);
        HOOK_TX.set(tx).unwrap();
        let urls = webhook.hook_urls.clone();
        thread::Builder::new()
            .name("bot-runtime".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(4)
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    boardcast(urls, rx);
                });
        })
        .unwrap();
    }
}

pub fn boardcast(urls: Vec<String>, rx: Receiver<String>) {
    while let Ok(msg) = rx.recv() {
        info!("[webhook] received msg: {}", msg);
        for u in &urls {
            info!("[webhook] send to {}", u);
            let u = u.clone();
            let m = msg.clone();
            G_TOKIO_RUNTIME.spawn(async move {
                let client = reqwest::Client::new();
                client.post(u.clone()).body(m.clone()).send().await.unwrap();
            });
        }
    }
}