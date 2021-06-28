use serde::{Deserialize, Serialize};
use solana_sdk::clock::Epoch;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EpochCreditCacheMetadata {
    seen_epochs: BTreeSet<Epoch>,
}

impl EpochCreditCacheMetadata {
    pub fn seen_epochs(&self) -> &BTreeSet<Epoch> {
        &self.seen_epochs
    }

    pub fn insert_epoch(&mut self, epoch: Epoch) {
        self.seen_epochs.insert(epoch);
    }
}
