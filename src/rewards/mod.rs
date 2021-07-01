use crate::rewards::caching::RewardsCache;
use anyhow::anyhow;
use log::debug;
use prometheus_exporter::prometheus::{GaugeVec, IntGaugeVec};
use solana_client::rpc_client::RpcClient;
use solana_runtime::bank::RewardType;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo};
use solana_stake_program::stake_state::StakeState;
use solana_transaction_status::{Reward, Rewards};
use std::{collections::BTreeSet, convert::TryInto, u64};

pub mod caching;
pub mod caching_metadata;

const SLOT_OFFSET: u64 = 20;

struct StakingApy {
    voter: String,
    percent: f64,
}

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
    cache: &'a RewardsCache<'a>, // NOTE: use get_seen_epochs() for "last_rewards_epoch".
}

impl<'a> RewardsMonitor<'a> {
    /// Initialises a new rewards monitor.
    pub fn new(
        client: &'a RpcClient,
        staking_apys: &'a GaugeVec,
        validator_rewards: &'a IntGaugeVec,
        rewards_cache: &'a RewardsCache<'a>,
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
        if epoch <= self.cache.get_last_seen_epoch()?.unwrap_or(0) {
            return Ok(());
        }
        if let Some(rewards) = self.get_rewards_for_epoch(epoch, epoch_info.clone())? {
            // Add epoch rewards to database
            self.cache.add_epoch_rewards(epoch, &rewards)?;

            // FIXME: replace 3.0 with the previous epoch duration when using only one epoch, and with the average of all used epochs if using several.
            let epochs_in_year = 365.0 / 3.0;
            let (staking_apys, validator_rewards) =
                self.process_rewards(rewards, epochs_in_year)?;

            for s in staking_apys {
                self.staking_apys
                    .get_metric_with_label_values(&[&s.voter])
                    .map(|c| c.set(s.percent))?;
            }

            for v in validator_rewards {
                self.validator_rewards
                    .get_metric_with_label_values(&[&v.voter])
                    .map(|c| c.set(v.lamports as i64))?;
            }
        }
        Ok(())
    }

    /// Splits rewards into reward type categories and does post-processing.
    // FIXME: Make this work across multiple epochs, potentially grabbing data from the cache.
    fn process_rewards(
        &self,
        rewards: Rewards,
        epochs_in_year: f64,
    ) -> anyhow::Result<(Vec<StakingApy>, Vec<ValidatorReward>)> {
        debug!("Processing rewards");
        let mut staking_seen_voters = BTreeSet::new();
        let mut staking_apys = Vec::new();

        let (staking_rewards, other_rewards): (Vec<_>, Vec<_>) = rewards
            .into_iter()
            .partition(|r| r.reward_type == Some(RewardType::Staking));
        let validator_rewards: Vec<_> = other_rewards
            .into_iter()
            .filter(|r| r.reward_type == Some(RewardType::Voting))
            .map(|r| ValidatorReward {
                voter: r.pubkey,
                lamports: r.post_balance,
            })
            .collect();

        for chunk in staking_rewards.chunks(100) {
            let pubkeys: Vec<_> = chunk
                .iter()
                .filter_map(|r| r.pubkey.as_str().try_into().ok())
                .collect();
            let account_infos = self.client.get_multiple_accounts(&pubkeys)?;

            for (
                Reward {
                    lamports,
                    post_balance,
                    ..
                },
                maybe_account_info,
            ) in chunk.into_iter().zip(account_infos.into_iter())
            {
                if let Some(account_info) = maybe_account_info {
                    let stake_state: StakeState = bincode::deserialize(&account_info.data)?;
                    if let Some(delegation) = stake_state.delegation() {
                        let voter = format!("{}", delegation.voter_pubkey);
                        if !staking_seen_voters.contains(&voter) && *lamports > 0 {
                            // TODO: Figure out what needs to be stored in the cache such that APY calculations can be reconstructed easily
                            let lamports = *lamports as u64;
                            let prev_balance = post_balance - lamports;
                            let epoch_rate = lamports as f64 / prev_balance as f64;
                            let apy = 100.0
                                * (f64::powf(1.0 + epoch_rate / epochs_in_year, epochs_in_year)
                                    - 1.0);
                            debug!(
                                "Staking APY of {} is {} with epoch rate {}",
                                voter, apy, epoch_rate
                            );
                            staking_apys.push(StakingApy {
                                voter: voter.clone(),
                                percent: apy,
                            });
                            staking_seen_voters.insert(voter);
                        }
                    }
                }
            }
        }
        Ok((staking_apys, validator_rewards))
    }

    /// Gets the rewards for `epoch` given the current `epoch_info`. Returns `Ok(None)` if there
    /// haven't been any rewards in the given epoch yet, `Ok(Some(rewards))` if there have, and
    /// otherwise returns an error.
    fn get_rewards_for_epoch(
        &self,
        epoch: Epoch,
        epoch_info: EpochInfo,
    ) -> anyhow::Result<Option<Rewards>> {
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
            Ok(Some(self.client.get_block(block)?.rewards))
        } else if end_slot.is_none() {
            // Possibly not yet computed the first block.
            Ok(None)
        } else {
            Err(anyhow!("no blocks found"))
        }
    }
}
