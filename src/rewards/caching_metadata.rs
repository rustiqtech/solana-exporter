use serde::{Deserialize, Serialize};
use solana_sdk::clock::Epoch;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EpochCreditCacheMetadata {
    rewards_epoch_seen: BTreeSet<Epoch>,
    account_data_epoch_seen: BTreeSet<Epoch>,
}

impl EpochCreditCacheMetadata {
    pub fn rewards_epoch_seen(&self) -> &BTreeSet<Epoch> {
        &self.rewards_epoch_seen
    }

    pub fn insert_rewards_epoch(&mut self, epoch: Epoch) {
        self.rewards_epoch_seen.insert(epoch);
    }

    pub fn account_data_epoch_seen(&self) -> &BTreeSet<Epoch> {
        &self.account_data_epoch_seen
    }

    pub fn insert_account_data_epoch(&mut self, epoch: Epoch) {
        self.account_data_epoch_seen.insert(epoch);
    }
}
