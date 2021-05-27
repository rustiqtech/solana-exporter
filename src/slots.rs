//! Statistics of skipped and validated slots.

use log::{debug, log_enabled, Level};
use prometheus_exporter::prometheus::IntCounterVec;
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::epoch_info::EpochInfo;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

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

enum SlotStatus {
    Skipped,
    Validated,
}

impl Display for SlotStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            SlotStatus::Skipped => "skipped",
            SlotStatus::Validated => "validated",
        };
        write!(f, "{}", s)
    }
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

        let (abs_range_start, range_start) = if self.already_ran {
            // Start from the last seen slot if already ran before.
            (first_slot + self.slot_index, self.slot_index)
        } else {
            // Start from the first slot in the current epoch if running for the first time.
            self.already_ran = true;
            (first_slot, 0)
        };
        let range_end = epoch_info.slot_index;
        let abs_range_end = first_slot + range_end;

        let mut confirmed_blocks = self
            .client
            .get_confirmed_blocks(abs_range_start, Some(abs_range_end))?;
        confirmed_blocks.sort_unstable();
        debug!(
            "Confirmed blocks from {} to {}: {:?}",
            abs_range_start, abs_range_end, confirmed_blocks
        );
        let mut feed = self.leader_slots.local();
        for slot_in_epoch in range_start..range_end {
            let leader = &self.slot_leaders[&(slot_in_epoch as usize)];
            let absolute_slot = first_slot + slot_in_epoch;
            let status = if confirmed_blocks.binary_search(&absolute_slot).is_ok() {
                SlotStatus::Validated
            } else {
                SlotStatus::Skipped
            };
            if log_enabled!(Level::Debug)
                && (slot_in_epoch < range_start + 50 || range_end - 50 < slot_in_epoch)
            {
                // Log only a subset of slots on the first run.
                debug!("Leader {} {} slot {}", leader, status, absolute_slot);
            }
            feed.with_label_values(&[leader, &status.to_string()])
                .inc_by(1)
        }
        feed.flush();

        self.slot_index = epoch_info.slot_index;
        debug!("Exported leader slots and updated the slot index");
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
