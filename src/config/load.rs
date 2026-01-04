use std::path::PathBuf;

use crate::{add::smart_read, config::Config};

pub fn load_config() -> Config {
    let config_path = PathBuf::from(".gato").join("config.toml");
    let config_byts = smart_read(&config_path).expect("config not found");
    let config_string = config_byts.to_str().expect("invalid config");

    let config: Config = toml::from_str(config_string).expect("Failed to parse config");
    config
}
