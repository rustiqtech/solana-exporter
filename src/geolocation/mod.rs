use solana_client::rpc_response::RpcContactInfo;
use std::net::IpAddr;

pub mod api;
pub mod caching;
pub mod identifier;

/// Get an IP address from a node. All three parameters of the node will be tried in order of TPU, Gossip,
/// and then RPC.
pub fn get_rpc_contact_ip(rpc: &RpcContactInfo) -> Option<IpAddr> {
    rpc.tpu.or(rpc.gossip).or(rpc.rpc).map(|s| s.ip())
}
