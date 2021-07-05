use crate::rewards::caching::RewardsCache;
use anyhow::anyhow;
use prometheus_exporter::prometheus::{GaugeVec, IntGaugeVec};
use solana_client::rpc_client::RpcClient;
use solana_runtime::bank::RewardType;
use solana_sdk::account::Account;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo, pubkey::Pubkey};
use solana_transaction_status::{Reward, Rewards};
use std::collections::{HashMap, HashSet};
use std::u64;

pub mod caching;
pub mod caching_metadata;

const SLOT_OFFSET: u64 = 20;

/// Maximum number of epochs to look back, INCLUSIVE of the current epoch.
const MAX_EPOCH_LOOKBACK: u64 = 5;

struct StakingApy {
    voter: String,
    percent: f64,
}

#[derive(Clone, Default, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct ValidatorReward {
    voter: String,
    lamports: u64,
}

/// The monitor of rewards paid to validators and delegators.
pub struct RewardsMonitor<'a> {
    /// Shared Solana RPC client.
    client: &'a RpcClient,
    /// Prometheus staking APY gauge.
    staking_apys: &'a GaugeVec,
    /// Prometheus cumulative validator rewards gauge.
    validator_rewards: &'a IntGaugeVec,
    /// Caching database for rewards
    cache: &'a RewardsCache, // NOTE: use get_seen_epochs() for "last_rewards_epoch".
}

impl<'a> RewardsMonitor<'a> {
    /// Initialises a new rewards monitor.
    pub fn new(
        client: &'a RpcClient,
        staking_apys: &'a GaugeVec,
        validator_rewards: &'a IntGaugeVec,
        rewards_cache: &'a RewardsCache,
    ) -> Self {
        Self {
            client,
            staking_apys,
            validator_rewards,
            cache: rewards_cache,
        }
    }

    /// Exports reward metrics once an epoch.
    pub fn export_rewards(&mut self, epoch_info: &EpochInfo) -> anyhow::Result<()> {
        let epoch = epoch_info.epoch;
        // FIXME: do not skip calculations if epoch is in database!
        // FIXME: Because it is possible that an epoch has been partially cached, this needs to be refactored.
        // if self.cache.seen_epoch(epoch)? {
        //     return Ok(());
        // }

        // FIXME: If we can get Rewards here then validator_rewards should always be Some.
        if self.get_rewards_for_epoch(epoch, epoch_info)?.is_some() {
            // FIXME: replace the constant with the previous epoch duration when using only one
            // epoch, and with the average of all used epochs if using several.
            let epoch_duration = 2.5;
            let staking_apys = self.calculate_staking_rewards(epoch_info, epoch_duration)?;

            for s in staking_apys {
                self.staking_apys
                    .get_metric_with_label_values(&[&s.voter])
                    .map(|c| c.set(s.percent))?;
            }

            let validator_rewards = self
                .calculate_validator_rewards(epoch)?
                .ok_or_else(|| anyhow!("current epoch has no rewards"))?;
            for v in validator_rewards {
                self.validator_rewards
                    .get_metric_with_label_values(&[&v.voter])
                    .map(|c| c.set(v.lamports as i64))?;
            }
        }
        Ok(())
    }

    /// Calculates the validator rewards for an epoch.
    fn calculate_validator_rewards(
        &self,
        epoch: Epoch,
    ) -> anyhow::Result<Option<HashSet<ValidatorReward>>> {
        Ok(self.cache.get_epoch_rewards(epoch)?.map(|rewards| {
            rewards
                .into_iter()
                .filter(|r| r.reward_type == Some(RewardType::Voting))
                .map(|r| ValidatorReward {
                    voter: r.pubkey,
                    lamports: r.post_balance,
                })
                .collect::<HashSet<_>>()
        }))
    }

    /// Calculates the staking rewards over the last `MAX_EPOCH_LOOKBACK` epochs.
    fn calculate_staking_rewards(
        &self,
        current_epoch_info: &EpochInfo,
        epoch_duration: f64,
    ) -> anyhow::Result<HashSet<StakingApy>> {
        let current_epoch = current_epoch_info.epoch;
        let mut rewards = HashMap::new();
        let mut accounts = HashMap::new();

        // Filling historical gaps
        self.fill_historical_epochs(current_epoch_info, &mut rewards, &mut accounts);

        // Fill current epoch and find APY
        self.fill_current_epoch_and_find_apy(current_epoch_info, &mut rewards, &mut accounts)
    }

    /// Fills `rewards` and `accounts` with previous epochs' information, up to `MAX_EPOCH_LOOKBACK` epochs ago.
    fn fill_historical_epochs(
        &self,
        current_epoch_info: &EpochInfo,
        rewards: &mut HashMap<(Pubkey, u64), Reward>,
        accounts: &mut HashMap<(Pubkey, u64), Option<Account>>,
    ) {
        let current_epoch = current_epoch_info.epoch;

        for epoch in (current_epoch - MAX_EPOCH_LOOKBACK)..current_epoch {
            // Historical rewards
            let historical_rewards = self
                .get_rewards_for_epoch(epoch, current_epoch_info)?
                .ok_or_else(|| anyhow!("historical epoch has no rewards"))?;
            for reward in historical_rewards {
                rewards.insert((reward.pubkey.parse()?, epoch), reward);
            }

            let historical_account = self.cache.get_epoch_data(epoch)?;
            if let Some(historical_account) = historical_account {
                accounts.extend(
                    historical_account
                        .into_iter()
                        .map(|(p, oa)| ((p, epoch), oa)),
                );
            }
        }
    }

    /// Fills `rewards` and `accounts` with the current epoch's information, either from the cache or RPC. The cache will be updated.
    fn fill_current_epoch_and_find_apy(
        &self,
        current_epoch_info: &EpochInfo,
        rewards: &mut HashMap<(Pubkey, Epoch), Reward>,
        // FIXME: Change type of this mapping
        accounts: &mut HashMap<(Pubkey, Epoch), Option<Account>>,
    ) -> anyhow::Result<HashSet<StakingApy>> {
        let current_epoch = current_epoch_info.epoch;

        let current_rewards = self
            .get_rewards_for_epoch(current_epoch, current_epoch_info)?
            .ok_or_else(|| anyhow!("current epoch has no rewards"))?;

        // Extract into staking rewards and validator rewards.
        let staking_rewards = current_rewards
            .into_iter()
            .filter(|reward| reward.reward_type == Some(RewardType::Staking))
            .collect::<Vec<_>>();

        // Copy staking_rewards into rewards
        for staking_reward in staking_rewards.iter().cloned() {
            rewards.insert(
                (staking_reward.pubkey.parse()?, current_epoch),
                staking_reward.clone(),
            );
        }

        // Fetched pubkeys from cache
        let cached_pubkeys = self
            .cache
            .get_epoch_data(current_epoch)?
            .unwrap_or_default();

        // Use cached pubkeys to find what keys we need to query
        let to_query = staking_rewards
            .into_iter()
            .map(|s| Ok(s.pubkey.parse()?))
            .collect::<anyhow::Result<HashSet<Pubkey>>>()?
            .difference(&cached_pubkeys.keys().cloned().collect::<HashSet<_>>())
            .cloned()
            .collect::<Vec<Pubkey>>(); // Use a Vec here to preserve ordering.

        // Move cached pubkeys into accounts
        accounts.extend(
            cached_pubkeys
                .into_iter()
                .map(|(p, a)| ((p, current_epoch), a)),
        );

        if !to_query.is_empty() {
            // Create empty hashmap
            let mut pka: HashMap<(_, _), _> = to_query
                .iter()
                .map(|pubkey| ((*pubkey, current_epoch), None))
                .collect::<HashMap<_, _>>();

            // Chunk into 100
            for chunk in to_query.chunks(100) {
                let account_infos = self.client.get_multiple_accounts(&chunk)?;

                // Write to hashmap
                // FIXME: Insert only APY data, calculate them here.
                for (pubkey, account_info) in chunk.iter().zip(account_infos) {
                    pka.insert((*pubkey, current_epoch), account_info);
                }

                let insert = pka
                    .clone()
                    .into_iter()
                    .map(|((pk, _), a)| (pk, a))
                    .collect::<HashMap<_, _>>();

                // Write to cache in chunks of 100 at a time.
                // FIXME: This writes all account state into database, which is inefficient. Refactor so that only APY data is written and `accounts` only stores APY data and drop account information here.
                self.cache.add_epoch_data(current_epoch, insert)?;
            }

            // Extend accounts
            accounts.extend(pka);
        }

        todo!("missing staking apy calculation using multiple epochs")
    }

    /// Gets the rewards for `epoch` given the current `epoch_info`, either from RPC or cache. The cache will be updated.
    /// Returns `Ok(None)` if there haven't been any rewards in the given epoch yet, `Ok(Some(rewards))` if there have, and
    /// otherwise returns an error.
    fn get_rewards_for_epoch(
        &self,
        epoch: Epoch,
        epoch_info: &EpochInfo,
    ) -> anyhow::Result<Option<Rewards>> {
        if let Some(rewards) = self.cache.get_epoch_rewards(epoch)? {
            Ok(Some(rewards))
        } else {
            // Convert epoch number to slot
            let start_slot = epoch * epoch_info.slots_in_epoch;

            // We cannot use an excessively large range if the epoch just started. There is a chance that
            // the end slot has not been reached and strange behaviour will occur.
            // If this is the current epoch and less than `SLOT_OFFSET` slots have elapsed, then do not define an
            // end_slot for use in the RPC call.
            let end_slot = if epoch_info.epoch == epoch && epoch_info.slot_index < SLOT_OFFSET {
                None
            } else {
                Some(start_slot + SLOT_OFFSET)
            };

            // First block only
            let block = self
                .client
                .get_blocks(start_slot, end_slot)?
                .get(0)
                .cloned();

            if let Some(block) = block {
                let rewards = self.client.get_block(block)?.rewards;
                self.cache.add_epoch_rewards(epoch, &rewards)?;
                Ok(Some(rewards))
            } else if end_slot.is_none() {
                // Possibly not yet computed the first block.
                Ok(None)
            } else {
                Err(anyhow!("no blocks found"))
            }
        }
    }
}
