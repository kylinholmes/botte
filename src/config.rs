use std::path::PathBuf;

use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};

pub static CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();

pub static CONFIG: Lazy<BotteConfig> = Lazy::new(|| {
    let config_path = CONFIG_PATH.get().unwrap();
    let config_str = std::fs::read_to_string(config_path).expect("Failed to read config file");
    let config: BotteConfig = toml::from_str(&config_str).expect("Failed to parse config file");
    config
});

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BotteConfig {
    pub listen: String,
    pub allow_chat_id: Vec<String>
}
