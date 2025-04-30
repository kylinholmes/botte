use botte::bot::run_bots;
use botte::config::CONFIG;
use botte::api::run_serve;

use std::path::PathBuf;
use clap::Parser;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::util::SubscriberInitExt;


fn main() {
    let _guard = boot().unwrap();

    run_bots();
    run_serve();
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