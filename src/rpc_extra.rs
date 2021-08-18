use crate::config::Whitelist;
use solana_client::{rpc_client::RpcClient, rpc_response::RpcVoteAccountStatus};
use solana_sdk::clock::Epoch;

/// Applies `f` to the first block in `epoch`.
pub fn with_first_block<F, A>(client: &RpcClient, epoch: Epoch, f: F) -> anyhow::Result<Option<A>>
where
    F: Fn(u64) -> anyhow::Result<Option<A>>,
{
    let epoch_schedule = client.get_epoch_schedule()?;
    let first_slot = epoch_schedule.get_first_slot_in_epoch(epoch);

    // First block in `epoch`.
    let first_block = client.get_blocks_with_limit(first_slot, 1)?.get(0).cloned();

    if let Some(block) = first_block {
        f(block)
    } else {
        Ok(None)
    }
}

/// Maps node pubkeys to vote pubkeys based on the information provided in `vote_accounts`.
pub fn vote_pubkeys(node_pubkeys: &Whitelist, vote_accounts: &RpcVoteAccountStatus) -> Whitelist {
    Whitelist(
        vote_accounts
            .current
            .iter()
            .chain(vote_accounts.delinquent.iter())
            .filter(|acc| node_pubkeys.0.contains(&acc.node_pubkey))
            .map(|acc| acc.vote_pubkey.clone())
            .collect(),
    )
}
