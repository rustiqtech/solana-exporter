use crate::rewards::caching::RewardsCache;
use anyhow::anyhow;
use log::debug;
use prometheus_exporter::prometheus::{GaugeVec, IntGaugeVec};
use solana_client::rpc_client::RpcClient;
use solana_runtime::bank::RewardType;
use solana_sdk::account::Account;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo, pubkey::Pubkey};
use solana_stake_program::stake_state::StakeState;
use solana_transaction_status::{Reward, Rewards};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::u64;

pub mod caching;
pub mod caching_metadata;

const SLOT_OFFSET: u64 = 20;

/// Maximum number of epochs to look back, INCLUSIVE of the current epoch.
const MAX_EPOCH_LOOKBACK: u64 = 5;

pub(crate) type PubkeyEpoch = (Pubkey, Epoch);
type PkEpochRewardMap = HashMap<PubkeyEpoch, Reward>;
type PkEpochApyMap = HashMap<PubkeyEpoch, f64>;

#[derive(Clone, Default, Debug, PartialOrd, PartialEq)]
struct StakingApy {
    voter: Pubkey,
    percent: f64,
}

#[derive(Clone, Default, Debug, PartialOrd, PartialEq)]
pub struct StakingReward {
    pub pubkey: Pubkey,
    pub lamports: i64,
    pub post_balance: u64, // Account balance in lamports after `lamports` was applied
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
            let staking_apys = self.calculate_staking_rewards(epoch_info)?;

            for s in staking_apys {
                self.staking_apys
                    .get_metric_with_label_values(&[&format!("{}", s.voter)])
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
    ) -> anyhow::Result<HashSet<StakingApy>> {
        // Filling historical gaps
        let (mut _rewards, mut apys) = self.fill_historical_epochs(current_epoch_info)?;

        // Fill current epoch and find APY
        self.fill_current_epoch_and_find_apy(current_epoch_info, /* &mut rewards, */ &mut apys)
    }

    /// Fills `rewards` and `apys` with previous epochs' information, up to `MAX_EPOCH_LOOKBACK` epochs ago.
    fn fill_historical_epochs(
        &self,
        current_epoch_info: &EpochInfo,
    ) -> anyhow::Result<(PkEpochRewardMap, PkEpochApyMap)> {
        let current_epoch = current_epoch_info.epoch;

        let mut rewards = HashMap::new();
        let mut apys = HashMap::new();

        for epoch in (current_epoch - MAX_EPOCH_LOOKBACK)..current_epoch {
            // Historical rewards
            let historical_rewards = self
                .get_rewards_for_epoch(epoch, current_epoch_info)?
                .ok_or_else(|| anyhow!("historical epoch has no rewards"))?;
            for reward in historical_rewards {
                rewards.insert((reward.pubkey.parse()?, epoch), reward);
            }

            let historical_apys = self.cache.get_epoch_apy(epoch)?.unwrap_or_default();
            apys.extend(historical_apys.into_iter().map(|(p, a)| ((p, epoch), a)));
        }

        Ok((rewards, apys))
    }

    /// Fills `rewards` and `accounts` with the current epoch's information, either from the cache or RPC. The cache will be updated.
    ///
    /// FIXME: `rewards` is currently not used. If needed, it can be moved back out of the
    /// comment. Otherwise we should remove it.
    fn fill_current_epoch_and_find_apy(
        &self,
        current_epoch_info: &EpochInfo,
        // rewards: &mut PkEpochRewardMap,
        apys: &mut PkEpochApyMap,
    ) -> anyhow::Result<HashSet<StakingApy>> {
        let current_epoch = current_epoch_info.epoch;

        let current_rewards = self
            .get_rewards_for_epoch(current_epoch, current_epoch_info)?
            .ok_or_else(|| anyhow!("current epoch has no rewards"))?;

        // Extract into staking rewards and validator rewards.
        let staking_rewards = current_rewards.into_iter().filter_map(|r| {
            if r.reward_type != Some(RewardType::Staking) {
                None
            } else if let Ok(pubkey) = r.pubkey.parse() {
                Some(StakingReward {
                    pubkey,
                    lamports: r.lamports,
                    post_balance: r.post_balance,
                })
            } else {
                None
            }
        });

        // // Copy staking_rewards into rewards
        // for staking_reward in staking_rewards.iter().cloned() {
        //     rewards.insert(
        //         (staking_reward.pubkey, current_epoch),
        //         staking_reward.clone(),
        //     );
        // }

        // Fetched pubkeys from cache
        let cached_apys = self.cache.get_epoch_apy(current_epoch)?.unwrap_or_default();

        // Use cached pubkeys to find what keys we need to query
        let cached_pubkeys: BTreeSet<_> = cached_apys.keys().collect();
        let to_query: Vec<_> = staking_rewards
            .filter(|r| !cached_pubkeys.contains(&r.pubkey))
            .collect();

        // Move cached pubkeys into apys
        apys.extend(
            cached_apys
                .into_iter()
                .map(|(p, a)| ((p, current_epoch), a)),
        );

        if !to_query.is_empty() {
            let mut pka = HashMap::new();

            // Seen voters are added here so that an APY calculation occurs is done only once
            // for a given voter.
            let mut seen_voters = BTreeSet::new();
            // Chunk into 100
            for chunk in to_query.chunks(100) {
                let pubkeys: Vec<_> = chunk.iter().map(|r| r.pubkey).collect();
                let account_infos = self.client.get_multiple_accounts(pubkeys.as_slice())?;

                // Write to hashmap
                for (reward, account_info) in chunk.iter().zip(account_infos) {
                    if let Some(account_info) = account_info {
                        if let Some(StakingApy { voter, percent }) = calculate_staking_apy(
                            &account_info,
                            &mut seen_voters,
                            3.0, /* FIXME: calculate */
                            reward.lamports as u64,
                            reward.post_balance,
                        )? {
                            pka.insert((voter, current_epoch), percent);
                        }
                    }
                }

                let insert = pka
                    .clone()
                    .into_iter()
                    .map(|((pk, _), a)| (pk, a))
                    .collect();

                // Write to cache in chunks of 100 at a time.
                self.cache.add_epoch_data(current_epoch, insert)?;
            }

            // Extend accounts
            apys.extend(pka);
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

/// Calculates the staking APY of an `AccountInfo` containing a `StakeState`.
/// Returns the calculated APY while registering the delegated voter in `seen_voters`
/// for later reference.
fn calculate_staking_apy(
    account_info: &Account,
    seen_voters: &mut BTreeSet<Pubkey>,
    epoch_duration: f64,
    lamports: u64,
    post_balance: u64,
) -> anyhow::Result<Option<StakingApy>> {
    let stake_state: StakeState = bincode::deserialize(&account_info.data)?;
    if let Some(delegation) = stake_state.delegation() {
        let percent = if !seen_voters.contains(&delegation.voter_pubkey) && lamports > 0 {
            let prev_balance = post_balance - lamports;
            let epoch_rate = lamports as f64 / prev_balance as f64;
            let apr = epoch_rate / epoch_duration * 365.0;
            let epochs_in_year = 365.0 / epoch_duration;
            let apy = f64::powf(1.0 + apr / epochs_in_year, epochs_in_year) - 1.0;
            debug!(
                "Staking APY of {} is {:.4} (APR {:.4})",
                delegation.voter_pubkey,
                apy * 100.0,
                apr * 100.0
            );
            seen_voters.insert(delegation.voter_pubkey);
            apy * 100.0
        } else {
            0.0
        };
        Ok(Some(StakingApy {
            voter: delegation.voter_pubkey,
            percent,
        }))
    } else {
        Ok(None)
    }
}
