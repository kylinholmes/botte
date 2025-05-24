// use tokio::sync::mpsc::{self, Sender, Receiver};
use once_cell::sync::OnceCell;
// use tokio::sync::broadcast::{self, Receiver as BroadcastReceiver, Sender as BroadcastSender};
use tokio::sync::mpsc::{self, Sender, Receiver};

use crate::{bot::BOTS_TX, webhook::HOOK_TX};

pub static BROADCAST_SENDER: OnceCell<Sender<String>> = OnceCell::new();
// pub static BROADCAST_RECEIVER: OnceCell<Sender<String>> = OnceCell::new();

pub async fn init_channel() -> anyhow::Result<()> {
    // let (sender, mut receiver) = broadcast::channel(32);
    let (sender, mut receiver): (Sender<String>, Receiver<String>) = mpsc::channel(32);
    BROADCAST_SENDER
        .set(sender)
        .map_err(|_| anyhow::anyhow!("Failed to set broadcast sender"))?;
    // BROADCAST_RECEIVER.set(receiver).map_err(|_| anyhow::anyhow!("Failed to set broadcast receiver"))?;


    while let Some(msg) = receiver.recv().await {
        // to bots
        if let Some(tx) = BOTS_TX.get() {
            tx.send(msg.clone()).unwrap();
        }

        // webhook
        if let Some(tx) = HOOK_TX.get() {
            tx.send(msg.clone()).unwrap();
        }
    }
    Ok(())
}
