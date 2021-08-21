use crate::config::Whitelist;
use crate::rewards::caching::RewardsCache;
use crate::rpc_extra::with_first_block;
use anyhow::anyhow;
use log::debug;
use prometheus_exporter::prometheus::{GaugeVec, IntGaugeVec};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_runtime::bank::RewardType;
use solana_sdk::account::Account;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo, pubkey::Pubkey};
use solana_stake_program::stake_state::StakeState;
use solana_transaction_status::{Reward, Rewards, TransactionDetails, UiTransactionEncoding};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::u64;
use time::OffsetDateTime;

pub mod caching;

/// How many seconds there are in a day
const SECONDS_IN_DAY: u64 = 86400;

/// How many days there are in a year
const DAYS_IN_YEAR: u64 = 365;

/// A default epoch length to use in case it cannot be found.
const DEFAULT_EPOCH_LENGTH: f64 = 3.0;

/// Maximum number of epochs to look back, INCLUSIVE of the current epoch.
const MAX_EPOCH_LOOKBACK: u64 = 5;

pub(crate) type VoterEpoch = (Pubkey, Epoch);
type VoterEpochRewardMap = HashMap<VoterEpoch, Reward>;
type VoterEpochApyMap = HashMap<VoterEpoch, f64>;

/// Staking APY of a particular voter pubkey in an epoch.
#[derive(Clone, Default, Debug, PartialOrd, PartialEq)]
struct StakingApy {
    voter: Pubkey,
    percent: f64,
}

/// Amount of staking rewards of a particular voter pubkey in an epoch.
#[derive(Clone, Default, Debug, PartialOrd, PartialEq)]
pub struct StakingReward {
    pub pubkey: Pubkey,
    pub lamports: i64,
    pub post_balance: u64, // Account balance in lamports after `lamports` was applied
}

/// Amount of rewards of a particular validator pubkey in an epoch.
#[derive(Clone, Default, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct ValidatorReward {
    voter: String,
    lamports: u64,
}

/// Set of staking APY
#[derive(Clone, Default, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct VoterApy {
    /// APY for the current epoch
    current_apy: f64,
    /// APY over the last `MAX_EPOCH_LOOKBACK` epochs.
    average_apy: f64,
}

/// The monitor of rewards paid to validators and delegators.
pub struct RewardsMonitor<'a> {
    /// Shared Solana RPC client.
    client: &'a RpcClient,
    /// Prometheus current staking APY gauge.
    current_staking_apy: &'a GaugeVec,
    /// Prometheus average staking APY gauge.
    average_staking_apy: &'a GaugeVec,
    /// Prometheus cumulative validator rewards gauge.
    validator_rewards: &'a IntGaugeVec,
    /// Caching database for rewards
    cache: &'a RewardsCache,
}

impl<'a> RewardsMonitor<'a> {
    /// Initialises a new rewards monitor.
    pub fn new(
        client: &'a RpcClient,
        current_staking_apy: &'a GaugeVec,
        average_staking_apy: &'a GaugeVec,
        validator_rewards: &'a IntGaugeVec,
        rewards_cache: &'a RewardsCache,
    ) -> Self {
        Self {
            client,
            current_staking_apy,
            average_staking_apy,
            validator_rewards,
            cache: rewards_cache,
        }
    }

    /// Exports reward metrics. APY values will not be re-calculated more than once an epoch.
    pub fn export_rewards(
        &mut self,
        epoch_info: &EpochInfo,
        vote_pubkey_whitelist: &Whitelist,
    ) -> anyhow::Result<()> {
        let epoch = epoch_info.epoch;

        // Possible that rewards haven't shown up yet for this epoch
        if self.get_rewards_for_epoch(epoch)?.is_some() {
            let staking_apys = self.calculate_staking_rewards(epoch_info, vote_pubkey_whitelist)?;

            for (
                voter,
                VoterApy {
                    current_apy,
                    average_apy,
                },
            ) in staking_apys
            {
                self.current_staking_apy
                    .get_metric_with_label_values(&[&format!("{}", voter)])
                    .map(|c| c.set(current_apy))?;
                self.average_staking_apy
                    .get_metric_with_label_values(&[&format!("{}", voter)])
                    .map(|c| c.set(average_apy))?;
            }

            let validator_rewards = self
                .calculate_validator_rewards(epoch, vote_pubkey_whitelist)?
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
        vote_pubkey_whitelist: &Whitelist,
    ) -> anyhow::Result<Option<HashSet<ValidatorReward>>> {
        Ok(self.cache.get_epoch_rewards(epoch)?.map(|rewards| {
            rewards
                .into_iter()
                .filter(|r| {
                    r.reward_type == Some(RewardType::Voting)
                        && vote_pubkey_whitelist.contains(&r.pubkey)
                })
                .map(|r| ValidatorReward {
                    voter: r.pubkey,
                    lamports: r.post_balance,
                })
                .collect::<HashSet<_>>()
        }))
    }

    /// Calculates the staking rewards for both the current epoch and the last `MAX_EPOCH_LOOKBACK` epochs.
    fn calculate_staking_rewards(
        &self,
        current_epoch_info: &EpochInfo,
        vote_pubkey_whitelist: &Whitelist,
    ) -> anyhow::Result<HashMap<Pubkey, VoterApy>> {
        // Since during an epoch the APY cannot change, make sure that all information about an epoch
        // is only calculated once, and then written to database to prevent inconsistent exporting.
        if let Some(apys) = self.cache.get_epoch_voter_apy(current_epoch_info.epoch)? {
            Ok(apys)
        } else {
            // Filling historical gaps
            let (_, mut apys) = self.fill_historical_epochs(current_epoch_info)?;

            // Fill current epoch and find APY
            let mapping = self.fill_current_epoch_and_find_apy(
                current_epoch_info,
                &mut apys,
                vote_pubkey_whitelist,
            )?;

            // Write to database
            self.cache
                .add_epoch_voter_apy(current_epoch_info.epoch, &mapping)?;

            Ok(mapping)
        }
    }

    /// Fills `rewards` and `apys` with previous epochs' information, up to `MAX_EPOCH_LOOKBACK` epochs ago.
    fn fill_historical_epochs(
        &self,
        current_epoch_info: &EpochInfo,
    ) -> anyhow::Result<(VoterEpochRewardMap, VoterEpochApyMap)> {
        let current_epoch = current_epoch_info.epoch;

        let mut rewards = HashMap::new();
        let mut apys = HashMap::new();

        for epoch in (current_epoch - MAX_EPOCH_LOOKBACK)..current_epoch {
            // Historical rewards
            let historical_rewards = self
                .get_rewards_for_epoch(epoch)?
                .ok_or_else(|| anyhow!("historical epoch has no rewards"))?;
            for reward in historical_rewards {
                rewards.insert((reward.pubkey.parse()?, epoch), reward);
            }

            let historical_apys = self.cache.get_epoch_apy(epoch)?.unwrap_or_default();

            apys.extend(
                historical_apys
                    .into_iter()
                    .map(|(_, (voter, apy))| ((voter, epoch), apy)),
            );
        }

        Ok((rewards, apys))
    }

    /// Fills `rewards` and `accounts` with the current epoch's information, either from the cache or RPC.
    /// The cache will be updated.
    fn fill_current_epoch_and_find_apy(
        &self,
        current_epoch_info: &EpochInfo,
        apys: &mut VoterEpochApyMap,
        vote_pubkey_whitelist: &Whitelist,
    ) -> anyhow::Result<HashMap<Pubkey, VoterApy>> {
        let current_epoch = current_epoch_info.epoch;

        let current_rewards = self
            .get_rewards_for_epoch(current_epoch)?
            .ok_or_else(|| anyhow!("current epoch has no rewards"))?
            .into_iter()
            .filter(|reward| vote_pubkey_whitelist.contains(&reward.pubkey));

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

        // Fetched pubkeys from cache
        let cached_apys = self.cache.get_epoch_apy(current_epoch)?.unwrap_or_default();

        // Use cached pubkeys to find what keys we need to query
        let cached_pubkeys: BTreeSet<_> = cached_apys.keys().collect();
        let to_query: Vec<_> = staking_rewards
            .filter(|r| !cached_pubkeys.contains(&r.pubkey))
            .collect();

        // Move cached pubkeys into APYs by voter
        apys.extend(
            cached_apys
                .into_iter()
                .map(|(_, (voter, apy))| ((voter, current_epoch), apy)),
        );

        if !to_query.is_empty() {
            let mut queried = HashMap::new();

            // Seen voters are added here so that an APY calculation occurs is done only once
            // for a given voter.
            let mut seen_voters = BTreeSet::new();

            // Chunk into 100
            for chunk in to_query.chunks(100) {
                let pubkeys: Vec<_> = chunk.iter().map(|r| r.pubkey).collect();
                debug!("Getting {} accounts", chunk.len());
                let account_infos = self.client.get_multiple_accounts(pubkeys.as_slice())?;

                // For each response in chunk
                for (reward, account_info) in chunk
                    .iter()
                    .zip(account_infos)
                    .flat_map(|(r, oa)| oa.map(|a| (r, a)))
                {
                    // Calculate APY
                    if let Some(StakingApy { voter, percent }) = calculate_staking_apy(
                        &account_info,
                        &mut seen_voters,
                        self.epoch_duration_days(current_epoch - 1, current_epoch_info)?
                            .unwrap_or(DEFAULT_EPOCH_LENGTH),
                        reward.lamports as u64,
                        reward.post_balance,
                    )? {
                        // Insert reward pubkey and voter
                        queried.insert(reward.pubkey, (voter, percent));
                    }
                }

                // Write to cache in chunks of 100 at a time.
                self.cache.add_epoch_data(current_epoch, queried.clone())?;
            }

            // Extend accounts by voter
            apys.extend(
                queried
                    .into_iter()
                    .map(|(_, (voter, percent))| ((voter, current_epoch), percent)),
            );
        }

        // A mapping of pubkeys to APYs in the preceding `MAX_EPOCH_LOOKBACK` epochs.
        let mut voter_epoch_apys: HashMap<Pubkey, BTreeMap<Epoch, f64>> = HashMap::new();
        // Fill in the epoch APYs of voters.
        for ((voter, epoch), apy) in apys {
            voter_epoch_apys
                .entry(*voter)
                .and_modify(|epoch_apys| {
                    epoch_apys.insert(*epoch, *apy);
                })
                .or_insert_with(|| std::iter::once((*epoch, *apy)).collect());
        }

        // Epoch durations up to lookback
        let epoch_durations = (current_epoch - MAX_EPOCH_LOOKBACK + 1..=current_epoch)
            .map(|epoch| {
                Ok((
                    epoch,
                    self.epoch_duration_days(epoch - 1, current_epoch_info)?
                        .unwrap_or(DEFAULT_EPOCH_LENGTH),
                ))
            })
            .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
        let duration_max_epoch_lookback: f64 = epoch_durations.values().sum();

        let mut voter_apys = HashMap::new();

        // Calculate the current and average APY
        for (voter, epoch_apys) in voter_epoch_apys {
            let mut total_apy = 0.0;
            for (epoch, duration) in &epoch_durations {
                let apy = *epoch_apys.get(epoch).unwrap_or(&0.0);
                total_apy += apy * duration;
            }
            let average_apy = total_apy / duration_max_epoch_lookback;
            let current_apy = *epoch_apys.get(&current_epoch).unwrap_or(&0.0);
            voter_apys.insert(
                voter,
                VoterApy {
                    current_apy,
                    average_apy,
                },
            );
        }
        Ok(voter_apys)
    }

    /// Calculates the duration of the epoch in days. May or may not use a cached result if the
    /// epoch is in the past. If the requested epoch is the current epoch, then the duration
    /// will be extrapolated from the current average slot time.
    /// Note that this function returns the epoch number exactly as requested. For calculating
    /// rewards, remember that the rewards for epoch `N-1` are in epoch `N`.
    /// Returns `None` if no block time is available for measurement.
    fn epoch_duration_days(
        &self,
        epoch: Epoch,
        epoch_info: &EpochInfo,
    ) -> anyhow::Result<Option<f64>> {
        // If it's the current epoch then we must extrapolate
        if epoch == epoch_info.epoch {
            let first_slot = epoch_info.absolute_slot - epoch_info.slot_index;
            return if let Some(first_slot_time) = self.client.get_block(first_slot)?.block_time {
                let average_slot_time = (OffsetDateTime::now_utc().unix_timestamp()
                    - first_slot_time) as f64
                    / (epoch_info.slot_index) as f64;
                Ok(Some(
                    average_slot_time * epoch_info.slots_in_epoch as f64 / SECONDS_IN_DAY as f64,
                ))
            } else {
                Ok(None)
            };
        }

        if let Some(length) = self.cache.get_epoch_length(epoch)? {
            Ok(Some(length))
        } else {
            debug!("Finding epoch {}", epoch);
            let days_in_epoch = {
                let first_block_timestamp = |ep| {
                    with_first_block(self.client, ep, |block| {
                        let ui_confirmed_block = self.client.get_block_with_config(
                            block,
                            RpcBlockConfig {
                                encoding: Some(UiTransactionEncoding::Base64),
                                transaction_details: Some(TransactionDetails::None),
                                rewards: Some(false),
                                commitment: None,
                            },
                        )?;
                        Ok(ui_confirmed_block.block_time)
                    })
                };

                let start_timestamp = first_block_timestamp(epoch)?;
                let end_timestamp = first_block_timestamp(epoch + 1)?;

                // Timestamps must exist for start and end block
                if let (Some(start_timestamp), Some(end_timestamp)) =
                    (start_timestamp, end_timestamp)
                {
                    (end_timestamp - start_timestamp) as f64 / SECONDS_IN_DAY as f64
                } else {
                    // Otherwise return early, do not update cache.
                    return Ok(None);
                }
            };

            self.cache.add_epoch_length(epoch, days_in_epoch)?;
            Ok(Some(days_in_epoch))
        }
    }

    /// Gets the rewards for `epoch`, either from RPC or cache. The cache will be updated.
    /// Returns `Ok(None)` if there haven't been any rewards in the given epoch yet, `Ok(Some(rewards))` if there have, and
    /// otherwise returns an error.
    fn get_rewards_for_epoch(&self, epoch: Epoch) -> anyhow::Result<Option<Rewards>> {
        if let Some(rewards) = self.cache.get_epoch_rewards(epoch)? {
            Ok(Some(rewards))
        } else {
            with_first_block(self.client, epoch, |block| {
                let rewards = self.client.get_block(block)?.rewards;
                self.cache.add_epoch_rewards(epoch, &rewards)?;
                Ok(Some(rewards))
            })
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
            let apr = epoch_rate / epoch_duration * (DAYS_IN_YEAR as f64);
            let epochs_in_year = (DAYS_IN_YEAR as f64) / epoch_duration;
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
            return Ok(None);
        };
        Ok(Some(StakingApy {
            voter: delegation.voter_pubkey,
            percent,
        }))
    } else {
        Ok(None)
    }
}
