use serde::{Deserialize, Serialize};
pub mod load;
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub title: String,
    pub compression: Option<CompressionConfig>,
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
