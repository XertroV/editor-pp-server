use serde_derive::Deserialize;
use simple_log::log::{info, warn};
use std::error::Error;
use std::fs;
use std::io::Read;

#[derive(Debug)]
#[derive(Deserialize)]
pub struct Config {
    pub server: Server,
}

#[derive(Debug, Default)]
#[derive(Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub no_local: Option<bool>,
}

pub fn read_config_from_file(path: &str) -> Result<Config, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

pub fn load_config() -> Config {
    // Config {
    //     server: Server {
    //         host: "127.0.0.1".to_string(),
    //         port: 38120,
    //     }
    // }
    match read_config_from_file("editor-pp-server.toml") {
        Ok(config) => {
            println!("Loaded config: {:?}", &config);
            config
        },
        Err(e) => {
            warn!("Failed to load config: {:?}", e);
            let c =
            Config {
                server: Server {
                    host: "127.0.0.1".to_string(),
                    port: 38120,
                    ..Default::default()
                }
            };
            println!("Loading default config instead: {:?}", c);
            c
        }
    }
}
