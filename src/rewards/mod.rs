use crate::rewards::caching::{RewardsCache, VoterValue};
use anyhow::anyhow;
use log::debug;
use prometheus_exporter::prometheus::{GaugeVec, IntGaugeVec};
use solana_client::rpc_client::RpcClient;
use solana_runtime::bank::RewardType;
use solana_sdk::account::Account;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo, pubkey::Pubkey};
use solana_stake_program::stake_state::StakeState;
use solana_transaction_status::{Reward, Rewards};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::u64;
use solana_client::rpc_response::RpcVoteAccountInfo;

pub mod caching;

const SLOT_OFFSET: u64 = 20;

/// Maximum number of epochs to look back, INCLUSIVE of the current epoch.
const MAX_EPOCH_LOOKBACK: u64 = 5;

pub(crate) type VoterEpoch = (Pubkey, Epoch);
type VoterEpochRewardMap = HashMap<VoterEpoch, Reward>;
type VoterEpochApyMap = HashMap<VoterEpoch, f64>;

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

#[derive(Clone, Default, Debug, PartialOrd, PartialEq)]
struct VoterApy {
    current_apy: f64,
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
    cache: &'a RewardsCache, // NOTE: use get_seen_epochs() for "last_rewards_epoch".
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

    /// Exports reward metrics once an epoch.
    pub fn export_rewards(&mut self, epoch_info: &EpochInfo) -> anyhow::Result<()> {
        let epoch = epoch_info.epoch;

        if self.get_rewards_for_epoch(epoch, epoch_info)?.is_some() {
            let staking_apys = self.calculate_staking_rewards(epoch_info)?;

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
    ) -> anyhow::Result<HashMap<Pubkey, VoterApy>> {
        let current_epoch = current_epoch_info.epoch;
        let rewards = self.client.get_vote_accounts()?;
        let mut apys = HashMap::new();

        let epoch_durations: f64 = (current_epoch - MAX_EPOCH_LOOKBACK + 1..=current_epoch)
            .map(|epoch| Ok((epoch, self.epoch_duration_days(epoch, current_epoch_info)?)))
            .collect::<anyhow::Result<BTreeMap<_, _>>>()?
            .values()
            .sum();

        for RpcVoteAccountInfo {
            vote_pubkey, epoch_credits, ..
        } in rewards.current.iter().chain(rewards.delinquent.iter()) {
            let vote_pubkey = vote_pubkey.parse()?;
            // Update cache
            self.cache.add_voter_history(&vote_pubkey, epoch_credits)?;
            let voter_history = self.cache.get_voter_history(&vote_pubkey)?;
            let current_apy = voter_history.get(&current_epoch)
                .map(|v| self.find_apy(current_epoch, current_epoch_info, *v))
                .transpose()?
                .unwrap_or(0.0);
            debug!("Staking APY of {} is {:.4}", &vote_pubkey, &current_apy);
            let average_apy = {
                let mut a = 0.0;
                for epoch in current_epoch - MAX_EPOCH_LOOKBACK + 1..=current_epoch {
                    let apy = voter_history.get(&epoch)
                        .map(|v| self.find_apy(epoch, current_epoch_info, *v))
                        .transpose()?
                        .unwrap_or(0.0);
                    a += apy;
                }
                a / epoch_durations
            };
            apys.insert(vote_pubkey, VoterApy {
                current_apy,
                average_apy
            });
        }

        panic!("Complete");
        Ok(apys)

        // // Filling historical gaps
        // let (mut _rewards, mut apys) = self.fill_historical_epochs(current_epoch_info)?;
        //
        // // Fill current epoch and find APY
        // self.fill_current_epoch_and_find_apy(current_epoch_info, /* &mut rewards, */ &mut apys)
    }

    fn find_apy(&self, epoch: Epoch, epoch_info: &EpochInfo, values: VoterValue) -> anyhow::Result<f64> {
        // We want the `epoch - 1` duration because of roll-over.
        let epoch_duration = self.epoch_duration_days(epoch - 1, &epoch_info)?;
        let (balance, previous_balance) = (values.0, values.1);
        let delta = balance as i64 - previous_balance as i64;
        let epoch_rate = delta as f64 / previous_balance as f64;
        let apr = epoch_rate / epoch_duration * 365.0;
        let epochs_in_year = 365.0 / epoch_duration;
        let apy = f64::powf(1.0 + apr / epochs_in_year, epochs_in_year) - 1.0;
        debug!("Epoch duration: {}", epoch_duration);
        debug!("Balance, previous balance: {}, {}", balance, previous_balance);
        debug!("Delta: {}", delta);
        debug!("Epoch rate: {}", epoch_rate);
        debug!("APR: {}", apr);
        debug!("Epochs in year: {}", epochs_in_year);
        Ok(apy)
    }

    /// Fills `rewards` and `apys` with previous epochs' information, up to `MAX_EPOCH_LOOKBACK` epochs ago.
    // fn fill_historical_epochs(
    //     &self,
    //     current_epoch_info: &EpochInfo,
    // ) -> anyhow::Result<(VoterEpochRewardMap, VoterEpochApyMap)> {
    //     let current_epoch = current_epoch_info.epoch;
    //
    //     let mut rewards = HashMap::new();
    //     let mut apys = HashMap::new();
    //
    //     for epoch in (current_epoch - MAX_EPOCH_LOOKBACK)..current_epoch {
    //         // Historical rewards
    //         let historical_rewards = self
    //             .get_rewards_for_epoch(epoch, current_epoch_info)?
    //             .ok_or_else(|| anyhow!("historical epoch has no rewards"))?;
    //         for reward in historical_rewards {
    //             rewards.insert((reward.pubkey.parse()?, epoch), reward);
    //         }
    //
    //         let historical_apys = self.cache.get_epoch_apy(epoch)?.unwrap_or_default();
    //         apys.extend(
    //             historical_apys
    //                 .into_iter()
    //                 .map(|(_, (voter, apy))| ((voter, epoch), apy)),
    //         );
    //     }
    //
    //     Ok((rewards, apys))
    // }

    /// Fills `rewards` and `accounts` with the current epoch's information, either from the cache or RPC. The cache will be updated.
    ///
    /// FIXME: `rewards` is currently not used. If needed, it can be moved back out of the
    /// comment. Otherwise we should remove it.
    // fn fill_current_epoch_and_find_apy(
    //     &self,
    //     current_epoch_info: &EpochInfo,
    //     // rewards: &mut PkEpochRewardMap,
    //     apys: &mut VoterEpochApyMap,
    // ) -> anyhow::Result<HashMap<Pubkey, VoterApy>> {
    //     let current_epoch = current_epoch_info.epoch;
    //
    //     let current_rewards = self
    //         .get_rewards_for_epoch(current_epoch, current_epoch_info)?
    //         .ok_or_else(|| anyhow!("current epoch has no rewards"))?;
    //
    //     // Extract into staking rewards and validator rewards.
    //     let staking_rewards = current_rewards.into_iter().filter_map(|r| {
    //         if r.reward_type != Some(RewardType::Staking) {
    //             None
    //         } else if let Ok(pubkey) = r.pubkey.parse() {
    //             Some(StakingReward {
    //                 pubkey,
    //                 lamports: r.lamports,
    //                 post_balance: r.post_balance,
    //             })
    //         } else {
    //             None
    //         }
    //     });
    //
    //     // Fetched pubkeys from cache
    //     let cached_apys = self.cache.get_epoch_apy(current_epoch)?.unwrap_or_default();
    //
    //     // Use cached pubkeys to find what keys we need to query
    //     let cached_pubkeys: BTreeSet<_> = cached_apys.keys().collect();
    //     let to_query: Vec<_> = staking_rewards
    //         .filter(|r| !cached_pubkeys.contains(&r.pubkey))
    //         .collect();
    //
    //     // Move cached pubkeys into APYs by voter
    //     apys.extend(
    //         cached_apys
    //             .into_iter()
    //             .map(|(_, (voter, apy))| ((voter, current_epoch), apy)),
    //     );
    //
    //     if !to_query.is_empty() {
    //         let mut queried = HashMap::new();
    //
    //         // Seen voters are added here so that an APY calculation occurs is done only once
    //         // for a given voter.
    //         let mut seen_voters = BTreeSet::new();
    //         // Chunk into 100
    //         for chunk in to_query.chunks(100) {
    //             let pubkeys: Vec<_> = chunk.iter().map(|r| r.pubkey).collect();
    //             debug!("Getting multiple accounts");
    //             let account_infos = self.client.get_multiple_accounts(pubkeys.as_slice())?;
    //
    //             // For each response in chunk
    //             for (reward, account_info) in chunk
    //                 .iter()
    //                 .zip(account_infos)
    //                 .flat_map(|(r, oa)| oa.map(|a| (r, a)))
    //             {
    //                 // Calculate APY
    //                 if let Some(StakingApy { voter, percent }) = calculate_staking_apy(
    //                     &account_info,
    //                     &mut seen_voters,
    //                     self.epoch_duration_days(current_epoch, current_epoch_info)?,
    //                     reward.lamports as u64,
    //                     reward.post_balance,
    //                 )? {
    //                     // Insert reward pubkey and voter
    //                     queried.insert(reward.pubkey, (voter, percent));
    //                 }
    //             }
    //
    //             // Write to cache in chunks of 100 at a time.
    //             self.cache.add_epoch_data(current_epoch, queried.clone())?;
    //         }
    //
    //         // Extend accounts by voter
    //         apys.extend(
    //             queried
    //                 .into_iter()
    //                 .map(|(_, (voter, percent))| ((voter, current_epoch), percent)),
    //         );
    //     }
    //
    //     // A mapping of pubkeys to APYs in the preceding `MAX_EPOCH_LOOKBACK` epochs.
    //     let mut voter_epoch_apys: HashMap<Pubkey, BTreeMap<Epoch, f64>> = HashMap::new();
    //     // Fill in the epoch APYs of voters.
    //     for ((voter, epoch), apy) in apys {
    //         voter_epoch_apys
    //             .entry(*voter)
    //             .and_modify(|epoch_apys| {
    //                 epoch_apys.insert(*epoch, *apy);
    //             })
    //             .or_insert_with(|| std::iter::once((*epoch, *apy)).collect());
    //     }
    //
    //     // TODO: Update this part according to changes to `epoch_duration_days`. A local map could
    //     // become redundant if the struct caches it in a field, for example.
    //     let epoch_durations = (current_epoch - MAX_EPOCH_LOOKBACK + 1..=current_epoch)
    //         .map(|epoch| Ok((epoch, self.epoch_duration_days(epoch, current_epoch_info)?)))
    //         .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    //     let duration_max_epoch_lookback: f64 = epoch_durations.values().sum();
    //
    //     let mut voter_apys = HashMap::new();
    //     for (voter, epoch_apys) in voter_epoch_apys {
    //         let mut total_apy = 0.0;
    //         for (epoch, duration) in &epoch_durations {
    //             let apy = *epoch_apys.get(epoch).unwrap_or(&0.0);
    //             total_apy += apy * duration;
    //         }
    //         let average_apy = total_apy / duration_max_epoch_lookback;
    //         let current_apy = *epoch_apys.get(&current_epoch).unwrap_or(&0.0);
    //         voter_apys.insert(
    //             voter,
    //             VoterApy {
    //                 current_apy,
    //                 average_apy,
    //             },
    //         );
    //     }
    //     Ok(voter_apys)
    // }

    fn epoch_duration_days(&self, epoch: Epoch, epoch_info: &EpochInfo) -> anyhow::Result<f64> {
        // TODO: Use some extrapolation magic to find out the epoch length if it's the current epoch
        // If it's the current epoch we can't use the next epoch.
        if epoch == epoch_info.epoch {
            return Ok(3.0);
        }

        if let Some(length) = self.cache.get_epoch_length(epoch)? {
            Ok(length)
        } else {
            debug!("Finding epoch {}", epoch);
            let days_in_epoch = {
                let start_timestamp = self
                    .client
                    .get_blocks(
                        epoch * epoch_info.slots_in_epoch,
                        Some((epoch * epoch_info.slots_in_epoch) + SLOT_OFFSET),
                    )?
                    .get(0)
                    .cloned()
                    .map(|slot| self.client.get_block(slot))
                    .transpose()?
                    .and_then(|x| x.block_time);

                let end_timestamp = self
                    .client
                    .get_blocks(
                        (epoch + 1) * epoch_info.slots_in_epoch,
                        Some(((epoch + 1) * epoch_info.slots_in_epoch) + SLOT_OFFSET),
                    )?
                    .get(0)
                    .cloned()
                    .map(|slot| self.client.get_block(slot))
                    .transpose()?
                    .and_then(|x| x.block_time);

                if let (Some(start_timestamp), Some(end_timestamp)) =
                    (start_timestamp, end_timestamp)
                {
                    (end_timestamp - start_timestamp) as f64 / 86_400_f64
                } else {
                    3.0
                }
            };

            self.cache.add_epoch_length(epoch, days_in_epoch)?;
            Ok(days_in_epoch)
        }
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

// Calculates the staking APY of an `AccountInfo` containing a `StakeState`.
// Returns the calculated APY while registering the delegated voter in `seen_voters`
// for later reference.
// fn calculate_staking_apy(
//     account_info: &Account,
//     seen_voters: &mut BTreeSet<Pubkey>,
//     epoch_duration: f64,
//     lamports: u64,
//     post_balance: u64,
// ) -> anyhow::Result<Option<StakingApy>> {
//     let stake_state: StakeState = bincode::deserialize(&account_info.data)?;
//     if let Some(delegation) = stake_state.delegation() {
//         let percent = if !seen_voters.contains(&delegation.voter_pubkey) && lamports > 0 {
//             let prev_balance = post_balance - lamports;
//             let epoch_rate = lamports as f64 / prev_balance as f64;
//             let apr = epoch_rate / epoch_duration * 365.0;
//             let epochs_in_year = 365.0 / epoch_duration;
//             let apy = f64::powf(1.0 + apr / epochs_in_year, epochs_in_year) - 1.0;
//             debug!(
//                 "Staking APY of {} is {:.4} (APR {:.4})",
//                 delegation.voter_pubkey,
//                 apy * 100.0,
//                 apr * 100.0
//             );
//             seen_voters.insert(delegation.voter_pubkey);
//             apy * 100.0
//         } else {
//             0.0
//         };
//         Ok(Some(StakingApy {
//             voter: delegation.voter_pubkey,
//             percent,
//         }))
//     } else {
//         Ok(None)
//     }
// }
