use anyhow::anyhow;
use prometheus_exporter::prometheus::GaugeVec;
use solana_client::rpc_client::RpcClient;
use solana_runtime::bank::RewardType;
use solana_sdk::{
    clock::Epoch, epoch_info::EpochInfo, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
};
use solana_stake_program::stake_state::StakeState;
use solana_transaction_status::{Reward, Rewards};
use std::{collections::BTreeSet, convert::TryFrom, u64};

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
    validator_rewards: &'a GaugeVec,
    /// The last epoch in which rewards were recorded. 0 if uninitialised.
    ///
    /// FIXME: cached epoch credit history.
    last_rewards_epoch: Epoch,
}

impl<'a> RewardsMonitor<'a> {
    /// Initialises a new rewards monitor.
    pub fn new(
        client: &'a RpcClient,
        staking_apys: &'a GaugeVec,
        validator_rewards: &'a GaugeVec,
    ) -> Self {
        Self {
            client,
            staking_apys,
            validator_rewards,
            last_rewards_epoch: 0,
        }
    }

    /// Exports reward metrics once an epoch.
    pub fn export_rewards(&mut self, epoch_info: &EpochInfo) -> anyhow::Result<()> {
        let epoch = epoch_info.epoch;
        // FIXME: check seen epochs in the cache.
        if epoch <= self.last_rewards_epoch {
            return Ok(());
        }
        if let Some(rewards) = self.get_rewards_for_epoch(epoch, Some(epoch_info.clone()))? {
            // FIXME: replace 3.0 with the previous epoch duration when using only one epoch, and
            // with the average of all used epochs if using several.
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
                    .map(|c| c.set(v.lamports as f64 / LAMPORTS_PER_SOL as f64))?;
            }
            // FIXME: add to seen epochs in the cache.
            self.last_rewards_epoch = epoch;
        }
        Ok(())
    }

    /// Splits rewards into reward type categories and does post-processing.
    fn process_rewards(
        &self,
        rewards: Rewards,
        epochs_in_year: f64,
    ) -> anyhow::Result<(Vec<StakingApy>, Vec<ValidatorReward>)> {
        let mut staking_seen_voters = BTreeSet::new();
        let mut staking_apys = Vec::new();
        let mut validator_rewards = Vec::new();

        for Reward {
            pubkey,
            lamports,
            post_balance,
            reward_type,
        } in rewards
        {
            match reward_type {
                Some(RewardType::Staking) => {
                    let account_info = self
                        .client
                        .get_account(&Pubkey::try_from(pubkey.as_ref())?)?;
                    let stake_state: StakeState = bincode::deserialize(&account_info.data)?;
                    if let Some(delegation) = stake_state.delegation() {
                        let voter = format!("{}", delegation.voter_pubkey);
                        if !staking_seen_voters.contains(&voter) && lamports > 0 {
                            let lamports = lamports as u64;
                            let prev_balance = post_balance - lamports;
                            let epoch_rate = lamports as f64 / prev_balance as f64;
                            let apy = 100.0
                                * (f64::powf(1.0 + epoch_rate / epochs_in_year, epochs_in_year)
                                    - 1.0);
                            staking_apys.push(StakingApy {
                                voter: voter.clone(),
                                percent: apy,
                            });
                            staking_seen_voters.insert(voter);
                        }
                    }
                }
                Some(RewardType::Voting) => validator_rewards.push(ValidatorReward {
                    voter: pubkey,
                    lamports: post_balance,
                }),
                _ => (), // TODO other reward types
            }
        }
        Ok((staking_apys, validator_rewards))
    }

    /// Gets the rewards for `epoch` optionally given `epoch_info`. Returns `Ok(None)` if there
    /// haven't been any rewards in the current epoch yet, `Ok(Some(rewards))` if there have, and
    /// otherwise returns an error.
    fn get_rewards_for_epoch(
        &self,
        epoch: Epoch,
        epoch_info: Option<EpochInfo>,
    ) -> anyhow::Result<Option<Rewards>> {
        let info = epoch_info.unwrap_or(self.client.get_epoch_info()?);

        // Convert epoch number to slot
        let start_slot = epoch * info.slots_in_epoch;

        // We cannot use an excessively large range if the epoch just started. There is a chance that
        // the end slot has not been reached and strange behaviour will occur.
        // If this is the current epoch and less than `SLOT_OFFSET` slots have elapsed, then do not define an
        // end_slot for use in the RPC call.
        let end_slot = if info.epoch == epoch && info.slot_index < SLOT_OFFSET {
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
