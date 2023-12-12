use std::fs;
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
const CONFIG_FILE: &str = "./etc/ezcron.toml";
#[cfg(not(debug_assertions))]
const CONFIG_FILE: &str = "/etc/ezcron.toml";


#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigEzCron {
    pub log_dir: String,
    pub pid_dir: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub ezcron: ConfigEzCron,
}

pub fn load(conf: Option<String>) -> Result<Config, Box<dyn std::error::Error>> {
    let conf = match conf {
        Some(conf) => conf,
        None => CONFIG_FILE.to_string(),
    };

    let toml_str = fs::read_to_string(conf)?;
    let config: Config = toml::from_str(&toml_str)?;
    Ok(config)
}