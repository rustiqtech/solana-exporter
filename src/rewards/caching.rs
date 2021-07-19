use anyhow::Context;
use solana_sdk::clock::Epoch;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{Reward, Rewards};
use std::collections::HashMap;

pub type PubkeyOwnerApyMapping = HashMap<Pubkey, (Pubkey, f64)>;

pub const EPOCH_REWARDS_TREE_NAME: &str = "epoch_rewards";
pub const APY_TREE_NAME: &str = "apy";
pub const EPOCH_LENGTH_NAME: &str = "epoch_length";

/// A caching database for vote accounts' credit growth
pub struct RewardsCache {
    epoch_rewards_tree: sled::Tree,
    apy_tree: sled::Tree,
    epoch_length_tree: sled::Tree,
}

impl RewardsCache {
    /// Creates a new cache using a tree.
    pub fn new(
        epoch_rewards_tree: sled::Tree,
        apy_tree: sled::Tree,
        epoch_length_tree: sled::Tree,
    ) -> Self {
        Self {
            epoch_rewards_tree,
            apy_tree,
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

    /// Adds a set of staking APY data of an epoch.
    pub fn add_epoch_data(&self, epoch: Epoch, apys: PubkeyOwnerApyMapping) -> anyhow::Result<()> {
        let mut mapping = self.get_epoch_apy(epoch)?.unwrap_or_default();

        mapping.extend(apys.into_iter());

        self.apy_tree
            .insert(epoch.to_be_bytes(), bincode::serialize(&mapping)?)
            .context("could not insert new APY data into database")?;
        Ok(())
    }

    /// Returns a set of staking APY data of an epoch
    pub fn get_epoch_apy(&self, epoch: Epoch) -> anyhow::Result<Option<PubkeyOwnerApyMapping>> {
        self.apy_tree
            .get(epoch.to_be_bytes())
            .context("could not fetch from database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize fetched data")
    }
}
