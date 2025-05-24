use botte::boardcast::init_channel;
use botte::bot::run_bots;
use botte::config::CONFIG;
use botte::api::run_serve;
use botte::mail::run_mail;
use botte::webhook::run_webhook;
use botte::G_TOKIO_RUNTIME;
use log::info;

use std::path::PathBuf;
use std::process::exit;
use clap::Parser;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::util::SubscriberInitExt;


fn enbale_server() {
    run_mail();
    run_serve();
}

fn enable_client() {
    run_bots();
    run_webhook();
}

fn main() {
    // enable_full_backtrace();
    enable_panic_hook();
    let _guard = boot().unwrap();

    init_channel().unwrap();
    enable_client();
    enbale_server();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}


#[derive(Debug, Parser)]
#[command(version, about = "botte", author = "kylin")]
struct Args {
    #[clap(short, long, default_value = "config/botte.toml")]
    pub config: PathBuf,
}

fn boot() -> anyhow::Result<WorkerGuard> {
    let args = Args::parse();
    let _ = botte::config::CONFIG_PATH.set(args.config);
    let cfg = CONFIG.clone();
    println!("{:?}", cfg);

    let file_appender = tracing_appender::rolling::daily("logs", "botte.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::level_filters::LevelFilter::INFO)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(non_blocking)
        .finish()
        .init();

    Ok(guard)
}

pub fn enable_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let println_panic_msg = |msg: &str| {
            println!("{}", msg);
            info!("{}", msg);
        };

        if let Some(location) = panic_info.location() {
            println_panic_msg(&format!(
                "panic occurred location in file '{}' at line {}",
                location.file(),
                location.line()
            ));
        }
        if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
            println_panic_msg(&format!("panic occurred payload: {}", payload));
        }
        println_panic_msg(&format!("panic occurred: {:?}", panic_info));
        default_hook(panic_info);
        exit(-1);
    }));
}

pub fn enable_full_backtrace() {
	unsafe { std::env::set_var("RUST_BACKTRACE", "full"); }
}