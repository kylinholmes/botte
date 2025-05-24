// basic
pub mod config;

// server to fetch msg
pub mod api;
pub mod mail;
// transport layer
pub mod boardcast;

// client to push msg
pub mod webhook;
pub mod bot;

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub static G_TOKIO_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().unwrap()
});