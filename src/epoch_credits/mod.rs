use anyhow::anyhow;
use solana_client::rpc_client::RpcClient;
use solana_sdk::clock::Epoch;
use solana_transaction_status::Rewards;

pub mod caching;
pub mod caching_metadata;

const SLOT_OFFSET: u64 = 20;

pub fn get_rewards_for_epoch(epoch: Epoch, client: &RpcClient) -> anyhow::Result<Rewards> {
    let info = client.get_epoch_info()?;

    // Convert epoch number to slot
    let start_slot = epoch * info.slots_in_epoch;

    // We cannot use an excessively large range if the epoch just started. There is a chance that
    // the end slot has not been reached and strange behaviour will occur.
    // If this is the current epoch and less than 10 slots have elapsed, then do not define an
    // end_slot for use in the RPC call.
    let end_slot = if info.epoch == epoch && info.slot_index < 10 {
        None
    } else {
        Some(start_slot + SLOT_OFFSET)
    };

    // First block only
    let block = client
        .get_blocks(start_slot, end_slot)?
        .get(0)
        .cloned()
        .ok_or_else(|| anyhow!("no blocks found"))?;

    Ok(client.get_block(block)?.rewards)
}
