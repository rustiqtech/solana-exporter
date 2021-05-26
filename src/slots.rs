//! Statistics of skipped and validated slots.

use log::{debug, error};
use prometheus_exporter::prometheus::IntCounterVec;
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::epoch_info::EpochInfo;
use std::collections::BTreeMap;

/// The monitor of skipped and validated slots per validator with minimal internal state.
pub struct SkippedSlotsMonitor<'a> {
    /// Shared Solana RPC client.
    client: &'a RpcClient,
    /// Prometheus gauge.
    leader_slots: &'a IntCounterVec,
    /// The last observed epoch number.
    epoch_number: u64,
    /// The last observed slot index.
    slot_index: u64,
    /// The slot leader schedule for the last observed epoch.
    slot_leaders: BTreeMap<usize, String>,
    /// `true` iff `SkippedSlotMonitor::export_skipped_slots` already ran.
    already_ran: bool,
}

impl<'a> SkippedSlotsMonitor<'a> {
    /// Constructs a monitor given `client`.
    pub fn new(client: &'a RpcClient, leader_slots: &'a IntCounterVec) -> Self {
        Self {
            client,
            leader_slots,
            epoch_number: 0,
            slot_index: 0,
            slot_leaders: Default::default(),
            already_ran: false,
        }
    }

    /// Exports the skipped slot statistics given `epoch_info`.
    pub fn export_skipped_slots(&mut self, epoch_info: &EpochInfo) -> Result<(), ClientError> {
        self.on_first_run(epoch_info)?;

        let first_slot = epoch_info.absolute_slot - epoch_info.slot_index;

        if self.epoch_number != epoch_info.epoch {
            self.slot_leaders = self.get_slot_leaders(None)?;
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
        let mut feed = self.leader_slots.local();
        for slot_in_epoch in self.slot_index..epoch_info.slot_index {
            let leader = &self.slot_leaders[&(slot_in_epoch as usize)];
            let absolute_slot = first_slot + slot_in_epoch;
            let status = if confirmed_blocks.contains(&absolute_slot) {
                "validated"
            } else {
                "skipped"
            };
            debug!("Leader {} {} slot {}", leader, status, absolute_slot);
            feed.with_label_values(&[leader, status]).inc_by(1)
        }
        feed.flush();

        self.slot_index = epoch_info.slot_index;
        debug!("Exported leader slots and updated the slot index");
        Ok(())
    }

    /// Reads and exports historical skipped block data.
    fn on_first_run(&mut self, epoch_info: &EpochInfo) -> Result<(), ClientError> {
        if self.already_ran {
            return Ok(());
        }
        let fst = |p: (&usize, &String)| *p.0;
        // Get leader schedules while they are still available.
        //
        // FIXME: even for the previous 3 epochs there are no schedules returned. So the whole code
        // block is a noop.
        for epoch in epoch_info.epoch - 3..epoch_info.epoch {
            debug!("Getting skipped slots in epoch {}", epoch);
            let slot_leaders = self.get_slot_leaders(Some(epoch))?;
            if slot_leaders.is_empty() {
                error!("Empty leader schedule in epoch {}", epoch);
            } else {
                let range_start =
                    slot_leaders.first_key_value().map(fst).unwrap_or_default() as u64;
                let range_end = slot_leaders.last_key_value().map(fst).unwrap_or_default() as u64;
                let confirmed_blocks = self
                    .client
                    .get_confirmed_blocks(range_start, Some(range_end))?;
                let mut feed = self.leader_slots.local();
                for slot in range_start..=range_end {
                    let leader = &slot_leaders[&(slot as usize)];
                    let status = if confirmed_blocks.contains(&slot) {
                        "validated"
                    } else {
                        "skipped"
                    };
                    feed.with_label_values(&[leader, status]).inc_by(1)
                }
                feed.flush();
            }
        }
        self.already_ran = true;
        Ok(())
    }

    /// Gets the leader schedule internally and inverts it, returning the slot leaders in `epoch` or
    /// in the current epoch if `epoch` is `None`.
    fn get_slot_leaders(&self, epoch: Option<u64>) -> Result<BTreeMap<usize, String>, ClientError> {
        let mut slot_leaders = BTreeMap::new();
        match self.client.get_leader_schedule(epoch)? {
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
