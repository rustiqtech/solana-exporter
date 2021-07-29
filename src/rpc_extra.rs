use anyhow::anyhow;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo};

/// The number of slots at the beginning of an epoch to look for the first block.
pub const SLOT_OFFSET: u64 = 40;

/// Applies `f` to the first block in `epoch`.
pub fn with_first_block<F, A>(
    client: &RpcClient,
    epoch: Epoch,
    epoch_info: &EpochInfo,
    f: F,
) -> anyhow::Result<Option<A>>
where
    F: Fn(u64) -> anyhow::Result<Option<A>>,
{
    // First slot in `epoch`.
    let first_slot = epoch * epoch_info.slots_in_epoch;

    // We cannot use an excessively large range if the epoch just started. There is a chance that
    // the end slot has not been reached and strange behaviour will occur.
    // If this is the current epoch and less than `SLOT_OFFSET` slots have elapsed, then do not define an
    // end_slot for use in the RPC call.
    let end_slot = if epoch_info.epoch == epoch && epoch_info.slot_index < SLOT_OFFSET {
        None
    } else {
        Some(first_slot + SLOT_OFFSET)
    };

    let blocks = client.get_blocks(first_slot, Some(first_slot + SLOT_OFFSET))?;
    // First block in `epoch`.
    let first_block = blocks.get(0).cloned();

    if let Some(block) = first_block {
        f(block)
    } else if end_slot.is_none() {
        // Possibly not yet computed the first block.
        Ok(None)
    } else {
        println!("first_slot {}, blocks {:?}", first_slot, blocks);
        Err(anyhow!("no blocks found"))
    }
}
