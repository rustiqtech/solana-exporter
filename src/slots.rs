//! Statistics of skipped and validated slots.

use log::debug;
use prometheus_exporter::prometheus::IntCounterVec;
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::epoch_info::EpochInfo;
use std::collections::BTreeMap;

/// The monitor of skipped and validated slots per validator with minimal internal state.
pub struct SkippedSlotsMonitor<'a> {
    /// Shared Solana RPC client.
    client: &'a RpcClient,
    /// The last observed epoch number.
    epoch_number: u64,
    /// The last observed slot index.
    slot_index: u64,
    /// The slot leader schedule for the last observed epoch.
    slot_leaders: BTreeMap<usize, String>,
}

impl<'a> SkippedSlotsMonitor<'a> {
    /// Constructs a monitor given `client`.
    pub fn new(client: &'a RpcClient) -> Self {
        Self {
            client,
            epoch_number: 0,
            slot_index: 0,
            slot_leaders: Default::default(),
        }
    }

    /// Exports the skipped slot statistics given `epoch_info` to `prometheus_leader_slots`.
    pub fn export_skipped_slots(
        &mut self,
        epoch_info: &EpochInfo,
        prometheus_leader_slots: &IntCounterVec,
    ) -> Result<(), ClientError> {
        let first_slot = epoch_info.absolute_slot - epoch_info.slot_index;

        if self.epoch_number != epoch_info.epoch {
            self.slot_leaders = self.get_slot_leaders()?;
            self.epoch_number = epoch_info.epoch;
            self.slot_index = epoch_info.slot_index;
            debug!("SkippedSlotsMonitor state updated");
        } else if self.slot_index == epoch_info.slot_index {
            debug!("At the slot index");
            return Ok(());
        }

        let range_start = first_slot + self.slot_index;
        let range_end = first_slot + epoch_info.slot_index;

        let confirmed_blocks = self
            .client
            .get_confirmed_blocks(range_start, Some(range_end))?;
        debug!(
            "Confirmed blocks from {} to {}: {:?}",
            range_start, range_end, confirmed_blocks
        );
        let mut feed = prometheus_leader_slots.local();
        for slot_in_epoch in self.slot_index..epoch_info.slot_index {
            let leader = &self.slot_leaders[&(slot_in_epoch as usize)];
            let absolute_slot = first_slot + slot_in_epoch;
            let status = if confirmed_blocks.contains(&absolute_slot) {
                "validated"
            } else {
                "skipped"
            };
            debug!("Leader {} {} slot {}", leader, status, absolute_slot);
            feed.with_label_values(&[status, leader]).inc_by(1)
        }
        feed.flush();

        self.slot_index = epoch_info.slot_index;
        debug!("Exported leader slots and updated the slot index");
        Ok(())
    }

    /// Gets the leader schedule internally and inverts it, returning the slot leaders in the current
    /// epoch.
    fn get_slot_leaders(&self) -> Result<BTreeMap<usize, String>, ClientError> {
        let mut slot_leaders = BTreeMap::new();
        match self.client.get_leader_schedule(None)? {
            None => (),
            Some(leader_schedule) => {
                for (pk, slots) in leader_schedule {
                    for slot in slots {
                        slot_leaders.insert(slot, pk.clone());
                    }
                }
            }
        }
        Ok(slot_leaders)
    }
}
