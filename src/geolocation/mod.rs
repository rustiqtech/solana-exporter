use solana_client::rpc_response::RpcContactInfo;
use std::net::IpAddr;

pub mod api;
pub mod caching;

pub const GEO_DB_CACHE_LOCATION: &str = "geolocation_cache.db";

pub fn get_rpc_contact_ip(rpc: &RpcContactInfo) -> Option<IpAddr> {
    rpc.tpu.or(rpc.gossip).or(rpc.rpc).map(|s| s.ip())
}
