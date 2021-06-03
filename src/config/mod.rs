use crate::geolocation::api::MaxMindAPIKey;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExporterConfig {
    /// Solana RPC address.
    pub rpc: String,
    /// Prometheus target socket address.
    pub target: SocketAddr,
    /// Maxmind API
    pub maxmind: MaxMindAPIKey,
}
