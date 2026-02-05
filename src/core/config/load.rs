use std::path::PathBuf;

use crate::core::{add::smart_read, config::Config, error::GatoResult};

pub fn load_config(path: &PathBuf) -> GatoResult<Config> {
    let config_path = path.join("gato.toml");
    let config_byts = smart_read(&config_path)?;
    let config_string = config_byts.to_str()?;

    let config: Config = toml::from_str(config_string)?;
    Ok(config)
}
