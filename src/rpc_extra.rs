use anyhow::anyhow;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo};

/// The number of slots at the beginning of an epoch to keep looking for the first block.
const SLOT_OFFSET: u64 = 100;

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
    let epoch_schedule = client.get_epoch_schedule()?;
    let first_slot = epoch_schedule.get_first_slot_in_epoch(epoch);

    // First block in `epoch`.
    let first_block = client.get_blocks_with_limit(first_slot, 1)?.get(0).cloned();

    if let Some(block) = first_block {
        f(block)
    } else if epoch_info.slot_index < SLOT_OFFSET {
        // Possibly not yet computed the first block.
        Ok(None)
    } else {
        Err(anyhow!("no blocks found"))
    }
}
