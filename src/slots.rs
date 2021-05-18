use solana_client::{
    client_error::ClientError,
    rpc_client::RpcClient,
    rpc_response::{RpcLeaderSchedule, RpcVoteAccountStatus},
};
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

    pub fn get_skipped_slots(&mut self, epoch_info: &EpochInfo) -> Result<(), ClientError> {
        let first_slot = epoch_info.absolute_slot - epoch_info.slot_index;
        let slot_index = first_slot + epoch_info.slots_in_epoch;

        if self.epoch_number != epoch_info.epoch {
            let leader_schedule = self.client.get_leader_schedule(None)?;
            self.epoch_number = epoch_info.epoch;
            self.slot_index = epoch_info.slot_index;
        } else if self.slot_index == epoch_info.slot_index {
            return Ok(());
        }

        // TODO

        Ok(())
    }

    fn slot_leaders(&self, leader_schedule: RpcLeaderSchedule) -> BTreeMap<usize, String> {
        let mut slot_leaders = BTreeMap::new();
        for (pk, slots) in leader_schedule {
            for slot in slots {
                slot_leaders.insert(slot, pk.clone());
            }
        }
        slot_leaders
    }
}
