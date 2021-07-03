use crate::persistent_database::metadata::Metadata;
use crate::rewards::caching_metadata::EpochCreditCacheMetadata;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use solana_sdk::account;
use solana_sdk::account::Account;
use solana_sdk::clock::Epoch;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{Reward, Rewards};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub type AccountsInfo = BTreeMap<Pubkey, Option<Account>>;

/// Name of the caching database.
pub const EPOCH_REWARDS_CACHE_TREE_NAME: &str = "epoch_rewards_credit_cache";
pub const ACCOUNT_CACHE_TREE_NAME: &str = "account_cache";

/// A caching database for vote accounts' credit growth
pub struct RewardsCache {
    epoch_rewards_tree: sled::Tree,
    account_tree: sled::Tree,
}

impl RewardsCache {
    /// Creates a new cache using a tree.
    pub fn new(epoch_rewards_tree: sled::Tree, account_tree: sled::Tree) -> Self {
        Self {
            epoch_rewards_tree,
            account_tree,
        }
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

    /// Adds a set of account data of an epoch.
    pub fn add_epoch_data(
        &self,
        epoch: Epoch,
        account_info: &[Option<Account>],
    ) -> anyhow::Result<()> {
        self.account_tree
            .insert(
                epoch.to_be_bytes(),
                bincode::serialize(&account_info.to_vec())?,
            )
            .context("could not insert new account data into database")?;
        Ok(())
    }

    /// Returns a set of account data of an epoch
    pub fn get_epoch_data(&self, epoch: Epoch) -> anyhow::Result<Option<AccountsInfo>> {
        self.account_tree
            .get(epoch.to_be_bytes())
            .context("could not fetch from database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize fetched data")
    }
}

// impl<'a> RewardsCache<'a> {
//     /// Creates a new cache using a tree.
//     pub fn new(tree: sled::Tree, metadata: &'a Metadata) -> Self {
//         Self { tree, metadata }
//     }
//
//     // TODO: Figure out a new data structure for storing staking rewards
//     // TODO: Implement read/write API for new data structure
//
//     /// Adds a set of rewards of an epoch
//     pub fn add_epoch_rewards(&self, epoch: Epoch, rewards: &[Reward]) -> anyhow::Result<()> {
//         for reward in rewards {
//             let mut content = self.get_credit_history(&reward.pubkey)?.unwrap_or_default();
//             content.epoch_reward.entry(epoch).or_insert_with(|| reward.clone());
//             let _ = self.write_rewards_history(&reward.pubkey, &content)?;
//         }
//
//         // Add to seen epochs
//         self.add_reward_epoch_to_metadata(epoch)?;
//
//         Ok(())
//     }
//
//     /// Gets a vote pubkey's credit history.
//     pub fn get_credit_history(&self, vote_pubkey: &str) -> anyhow::Result<Option<RewardsHistory>> {
//         self.tree
//             .get(vote_pubkey)
//             .context("could not fetch from database")?
//             .map(|x| bincode::deserialize(&x))
//             .transpose()
//             .context("could not deserialize the fetched credit_history")
//     }
//
//     /// Returns a list of all the epochs for which the database has records of. If the metadata
//     /// tree has no record, it is assumed that no epochs have been seen.
//     pub fn get_seen_reward_epochs(&self) -> anyhow::Result<BTreeSet<Epoch>> {
//         Ok(self
//             .metadata
//             .get_metadata::<EpochCreditCacheMetadata>(REWARDS_CACHE_TREE_NAME)?
//             .unwrap_or_default()
//             .rewards_epoch_seen()
//             .clone())
//     }
//
//     /// Returns the last seen epoch.
//     pub fn get_last_seen_reward_epoch(&self) -> anyhow::Result<Option<Epoch>> {
//         Ok(self.get_seen_reward_epochs()?.into_iter().next_back())
//     }
//
//     /// Write a key-value pair to the database. Returns the previously inserted value.
//     fn write_rewards_history(
//         &self,
//         vote_pubkey: &str,
//         credit_history: &RewardsHistory,
//     ) -> anyhow::Result<Option<RewardsHistory>> {
//         self.tree
//             .insert(vote_pubkey, bincode::serialize(credit_history)?)
//             .context("could not insert into database")?
//             .map(|x| bincode::deserialize(&x))
//             .transpose()
//             .context("could not deserialize the previously inserted credit_history")
//     }
//
//     /// Adds one epoch to the list of epochs for which the database has records of. Returns the
//     /// previous set of seen epochs (before insertion).
//     fn add_reward_epoch_to_metadata(
//         &self,
//         epoch: Epoch,
//     ) -> anyhow::Result<Option<EpochCreditCacheMetadata>> {
//         let mut metadata = self
//             .metadata
//             .get_metadata::<EpochCreditCacheMetadata>(REWARDS_CACHE_TREE_NAME)?
//             .unwrap_or_default();
//
//         metadata.insert_rewards_epoch(epoch);
//
//         self.metadata
//             .set_metadata(REWARDS_CACHE_TREE_NAME, &metadata)
//     }
//
//     /// Adds a set of account data of an epoch.
//     pub fn add_account_data_epoch(&self, epoch: Epoch, account_infos: &[Option<Account>]) -> anyhow::Result<()> {
//         todo!()
//     }
//
//     /// Gets a pubkey's account data at a particular epoch.
//     pub fn get_account_data_epoch(&self, epoch: Epoch, account: &Pubkey) -> anyhow::Result<Option<Account>> {
//         todo!()
//     }
// }

// /// The value (in key-value) for the epoch credit caching database.
// #[derive(Clone, Debug, Serialize, Deserialize, Default)]
// pub struct RewardsHistory {
//     pub epoch_reward: BTreeMap<Epoch, Reward>,
//     pub account_info: BTreeMap<Epoch, Option<Account>>
// }
