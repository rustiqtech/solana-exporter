use anyhow::Context;
use serde::{Deserialize, Serialize};
use solana_sdk::clock::Epoch;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{Reward, Rewards};
use std::collections::{BTreeMap, HashMap};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct VoterKey(Pubkey, Epoch);

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct VoterValue(pub u64, pub u64);

pub const EPOCH_REWARDS_TREE_NAME: &str = "epoch_rewards";
pub const APY_TREE_NAME: &str = "voter";
pub const EPOCH_LENGTH_NAME: &str = "epoch_length";

#[derive(Copy, Clone, Serialize, Deserialize)]
struct ApyTreeKey(Epoch, Pubkey);

#[derive(Copy, Clone, Serialize, Deserialize)]
struct ApyTreeValue(Pubkey, f64);

/// A caching database for vote accounts' credit growth
pub struct RewardsCache {
    epoch_rewards_tree: sled::Tree,
    voter_tree: sled::Tree,
    epoch_length_tree: sled::Tree,
}

impl RewardsCache {
    /// Creates a new cache using a tree.
    pub fn new(
        epoch_rewards_tree: sled::Tree,
        voter_tree: sled::Tree,
        epoch_length_tree: sled::Tree,
    ) -> Self {
        Self {
            epoch_rewards_tree,
            voter_tree,
            epoch_length_tree,
        }
    }

    /// Adds the length of an epoch.
    pub fn add_epoch_length(&self, epoch: Epoch, length: f64) -> anyhow::Result<()> {
        self.epoch_length_tree
            .insert(epoch.to_be_bytes(), bincode::serialize(&length)?)
            .context("could not insert epoch length into database")?;

        Ok(())
    }

    /// Returns the length of an epoch
    pub fn get_epoch_length(&self, epoch: Epoch) -> anyhow::Result<Option<f64>> {
        self.epoch_length_tree
            .get(epoch.to_be_bytes())
            .context("could not fetch epoch length from database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize fetched epoch length")
    }

    /// Adds a set of rewards of an epoch.
    pub fn add_epoch_rewards(&self, epoch: Epoch, rewards: &[Reward]) -> anyhow::Result<()> {
        // Insert into database
        self.epoch_rewards_tree
            .insert(epoch.to_be_bytes(), bincode::serialize(&rewards.to_vec())?)
            .context("could not insert epoch rewards into database")?;

        Ok(())
    }

    /// Returns the set of rewards of an epoch.
    pub fn get_epoch_rewards(&self, epoch: Epoch) -> anyhow::Result<Option<Rewards>> {
        self.epoch_rewards_tree
            .get(epoch.to_be_bytes())
            .context("could not fetch epoch rewards from database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize fetched epoch rewards")
    }

    pub fn add_voter_history(
        &self,
        vote_pubkey: &Pubkey,
        credits: &[(Epoch, u64, u64)],
    ) -> anyhow::Result<()> {
        for (epoch, credits, previous_credits) in credits {
            let key = bincode::serialize(&VoterKey(*vote_pubkey, *epoch))?;
            self.voter_tree
                .insert(
                    key,
                    bincode::serialize(&VoterValue(*credits, *previous_credits))?,
                )
                .context("could not insert voter credits into database")?;
        }

        Ok(())
    }

    pub fn get_voter_history(
        &self,
        vote_pubkey: &Pubkey,
    ) -> anyhow::Result<BTreeMap<Epoch, VoterValue>> {
        let mut mapping = BTreeMap::new();
        for kv in self.voter_tree.scan_prefix(bincode::serialize::<Pubkey>(
            &vote_pubkey,
        )?) {
            let (k, v) = kv?;
            let k: VoterKey = bincode::deserialize(&k)?;
            let v: VoterValue = bincode::deserialize(&v)?;
            mapping.insert(k.1, v);
        }
        Ok(mapping)
    }
}
