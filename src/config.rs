use serde_derive::Deserialize;
use simple_log::log::{info, warn};
use std::error::Error;
use std::fs;
use std::io::{self, Read};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: Server,
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

pub fn read_config_from_file(path: &str) -> Result<Config, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

pub fn load_config() -> Config {
    match read_config_from_file("config.toml") {
        Ok(config) => {
            info!("Loaded config: {:#?}", &config);
            config
        },
        Err(e) => {
            warn!("Failed to load config: {:#?}", e);
            let c =
            Config {
                server: Server {
                    host: "127.0.0.1".to_string(),
                    port: 38120,
                }
            };
            warn!("Loading default config instead: {:#?}", c);
            c
        }
    }
}
