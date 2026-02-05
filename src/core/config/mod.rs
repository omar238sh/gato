use serde::{Deserialize, Serialize};
pub mod load;
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub title: String,
    pub id: String,
    pub author: String,
    pub email: Option<String>,
    pub description: String,
    pub compression: Option<CompressionConfig>,
    ignore: Vec<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompressionConfig {
    pub level: Option<i32>,
    pub method: CompressionMethod,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CompressionMethod {
    Zlib,
    Zstd,
}

impl Config {
    pub fn ignored(self) -> Vec<String> {
        let mut ignored = self.ignore;
        ignored.push(".gato".to_string());
        ignored.push("gato.toml".to_string());
        ignored
    }
}
