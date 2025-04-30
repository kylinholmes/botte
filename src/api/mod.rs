pub mod serve;
pub mod webhook;


use log::info;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub static EXTERNAL_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().unwrap()
});


pub fn run_serve() {
    EXTERNAL_RUNTIME.block_on(async {
        let _ = serve::startup(webhook::api()).await;
        info!("API server started");
    });
}