use crate::geolocation::api::MaxMindAPIKey;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Whitelist(pub HashSet<String>);

impl Whitelist {
    pub fn contains(&self, value: &str) -> bool {
        self.0.is_empty() || self.0.contains(value)
    }
}

pub const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExporterConfig {
    /// Solana RPC address.
    pub rpc: String,
    /// Prometheus target socket address.
    pub target: SocketAddr,
    /// Whitelisted node pubkeys.
    pub node_whitelist: Option<Whitelist>,
    /// Whitelisted staking account pubkeys for APY calculation
    pub staking_account_whitelist: Option<Whitelist>,
    /// Maxmind API username and password.
    pub maxmind: Option<MaxMindAPIKey>,
}
