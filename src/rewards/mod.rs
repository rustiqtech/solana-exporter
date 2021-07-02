use crate::rewards::caching::{EpochHistory, RewardsCache};
use anyhow::anyhow;
use log::debug;
use prometheus_exporter::prometheus::{GaugeVec, IntGaugeVec};
use solana_client::rpc_client::RpcClient;
use solana_runtime::bank::RewardType;
use solana_sdk::account::Account;
use solana_sdk::{clock::Epoch, epoch_info::EpochInfo, pubkey::Pubkey};
use solana_stake_program::stake_state::StakeState;
use solana_transaction_status::{Reward, Rewards};
use std::collections::{HashMap, HashSet};
use std::{collections::BTreeSet, convert::TryInto, u64};

pub mod caching;
pub mod caching_metadata;

const SLOT_OFFSET: u64 = 20;

/// Maximum number of epochs to look back, INCLUSIVE of the current epoch.
const MAX_EPOCH_LOOKBACK: u64 = 5;

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
        if self.cache.seen_epoch(epoch)? {
            return Ok(());
        }

        if let Some(rewards) = self.get_rewards_for_epoch(epoch, epoch_info)? {
            // FIXME: replace the constant with the previous epoch duration when using only one
            // epoch, and with the average of all used epochs if using several.
            let epoch_duration = 2.5;
            let staking_apys = self.calculate_staking_rewards(epoch_info, epoch_duration)?;

            for s in staking_apys {
                self.staking_apys
                    .get_metric_with_label_values(&[&s.voter])
                    .map(|c| c.set(s.percent))?;
            }

            // TODO: Write a function that generates validator rewards
            // for v in validator_rewards {
            //     self.validator_rewards
            //         .get_metric_with_label_values(&[&v.voter])
            //         .map(|c| c.set(v.lamports as i64))?;
            // }
        }
        Ok(())
    }

    fn calculate_staking_rewards(
        &self,
        current_epoch_info: &EpochInfo,
        epoch_duration: f64,
    ) -> anyhow::Result<HashSet<StakingApy>> {
        let current_epoch = current_epoch_info.epoch;
        let mut rewards = HashMap::new();
        let mut accounts = HashMap::new();

        // Fill current epoch
        self.fill_current_epoch(current_epoch_info, &mut rewards, &mut accounts)?;

        // Filling historical gaps
        for epoch in (current_epoch - MAX_EPOCH_LOOKBACK)..current_epoch {
            let epoch_history = self.cache.get_epoch(epoch)?;
            if let Some(epoch_history) = epoch_history {
                for (pubkey, reward) in epoch_history.rewards {
                    rewards.insert((Pubkey::new(pubkey.as_bytes()), epoch), reward);
                }
                for (pubkey, account) in epoch_history.account_info {
                    accounts.insert((pubkey, epoch), account);
                }
            }
        }

        todo!("missing staking apy calculation using multiple epochs")
    }

    /// Fills `rewards` and `accounts` with the current epoch's information, either from the cache or RPC.
    fn fill_current_epoch(
        &self,
        current_epoch_info: &EpochInfo,
        rewards: &mut HashMap<(Pubkey, Epoch), Reward>,
        accounts: &mut HashMap<(Pubkey, Epoch), Option<Account>>,
    ) -> anyhow::Result<()> {
        let current_epoch = current_epoch_info.epoch;

        if let Some(epoch_history) = self.cache.get_epoch(current_epoch)? {
            for (pubkey, reward) in epoch_history.rewards {
                rewards.insert((Pubkey::new(pubkey.as_bytes()), current_epoch), reward);
            }
            for (pubkey, account) in epoch_history.account_info {
                accounts.insert((pubkey, current_epoch), account);
            }
        } else {
            let current_rewards = self
                .get_rewards_for_epoch(current_epoch, current_epoch_info)?
                .ok_or_else(|| anyhow!("current epoch has no rewards"))?;

            // Extract into staking rewards and validator rewards.
            let staking_rewards = current_rewards
                .into_iter()
                .filter(|reward| reward.reward_type == Some(RewardType::Staking))
                .collect::<Vec<_>>();

            for staking_reward in staking_rewards.iter().cloned() {
                // Insert into reward mapping
                rewards.insert(
                    (Pubkey::new(staking_reward.pubkey.as_bytes()), current_epoch),
                    staking_reward.clone(),
                );

                // Pre-fill accounts with Nones
                accounts.insert(
                    (Pubkey::new(staking_reward.pubkey.as_bytes()), current_epoch),
                    None,
                );
            }

            // Pre-fill pubkey-accounts with Nones.
            let mut pka: HashMap<(_, _), _> = staking_rewards
                .iter()
                .map(|reward| ((Pubkey::new(reward.pubkey.as_bytes()), current_epoch), None))
                .collect();

            // Chunk into 100
            for chunk in staking_rewards.chunks(100) {
                // Convert pubkey into Pubkey struct
                let pubkeys = chunk
                    .iter()
                    .map(|reward| Pubkey::new(reward.pubkey.as_bytes()))
                    .collect::<Vec<_>>();

                let account_infos = self.client.get_multiple_accounts(&pubkeys)?;

                // Insert account into into HashMap
                for account_info in account_infos.into_iter().flatten() {
                    pka.insert((account_info.owner, current_epoch), Some(account_info));
                }
            }

            // Write to cache
            self.cache
                .add_account_data(current_epoch, &pka.values().cloned().collect::<Vec<_>>())?;

            // Extend accounts
            accounts.extend(pka);
        }

        Ok(())
    }

    /// Gets the rewards for `epoch` given the current `epoch_info`, potentially from the cache if available. If not, RPC calls will be made. The result will be cached.
    /// Returns `Ok(None)` if there haven't been any rewards in the given epoch yet, `Ok(Some(rewards))` if there have, and
    /// otherwise returns an error.
    fn get_rewards_for_epoch(
        &self,
        epoch: Epoch,
        epoch_info: &EpochInfo,
    ) -> anyhow::Result<Option<Rewards>> {
        if let Some(epoch_history) = self.cache.get_epoch(epoch)? {
            Ok(Some(
                epoch_history.rewards.values().cloned().collect::<Vec<_>>(),
            ))
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
