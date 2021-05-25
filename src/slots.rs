use log::debug;
use prometheus_exporter::prometheus::IntCounterVec;
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::epoch_info::EpochInfo;
use std::collections::BTreeMap;

pub struct SkippedSlotsMonitor<'a> {
    client: &'a RpcClient,
    epoch_number: u64,
    slot_index: u64,
    slot_leaders: BTreeMap<usize, String>,
}

impl<'a> SkippedSlotsMonitor<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self {
            client,
            epoch_number: 0,
            slot_index: 0,
            slot_leaders: Default::default(),
        }
    }

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
        for block in self.slot_index..epoch_info.slot_index {
            let leader = &self.slot_leaders[&(block as usize)];
            let absolute_block = first_slot + block;
            let status = if confirmed_blocks.contains(&absolute_block) {
                "validated"
            } else {
                "skipped"
            };
            debug!("Leader {} {} block {}", leader, status, absolute_block);
            prometheus_leader_slots
                .local()
                .with_label_values(&[status, leader])
                .inc_by(1)
        }

        self.slot_index = epoch_info.slot_index;
        debug!("Exported leader slots and updated the slot index");
        Ok(())
    }

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
