pub mod serve;
pub mod webhook;


use log::info;

use crate::G_TOKIO_RUNTIME;

/// **Will block on current thread**
pub fn run_serve() {
    G_TOKIO_RUNTIME.spawn(async {
        let _ = serve::startup(webhook::api()).await;
        info!("API server started");
    });
}