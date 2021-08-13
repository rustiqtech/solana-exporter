use crate::geolocation::api::MaxMindAPIKey;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;

pub type Whitelist = HashSet<String>;

pub const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExporterConfig {
    /// Solana RPC address.
    pub rpc: String,
    /// Prometheus target socket address.
    pub target: SocketAddr,
    /// Whitelist addresses
    pub pubkey_whitelist: Whitelist,
    /// Maxmind API
    pub maxmind: Option<MaxMindAPIKey>,
}

impl ExporterConfig {
    pub fn whitelist_contains(&self, value: &str) -> bool {
        if self.pubkey_whitelist.is_empty() {
            true
        }
        else {
            self.pubkey_whitelist.contains(value)
        }
    }
}