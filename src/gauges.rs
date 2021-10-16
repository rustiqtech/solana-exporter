use crate::config::Whitelist;
use crate::geolocation::api::MaxMindAPIKey;
use crate::geolocation::api::MAXMIND_CITY_URI;
use crate::geolocation::caching::GeolocationCache;
use crate::geolocation::get_rpc_contact_ip;
use crate::geolocation::identifier::DatacenterIdentifier;
use crate::rpc_extra::with_first_block;
use anyhow::{anyhow, Context};
use futures::TryFutureExt;
use geoip2_city::CityApiResponse;
use log::{debug, error};
use prometheus_exporter::prometheus::{
    register_gauge, register_gauge_vec, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, Gauge, GaugeVec, IntCounterVec, IntGauge, IntGaugeVec,
};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_client::rpc_response::{RpcContactInfo, RpcVoteAccountInfo, RpcVoteAccountStatus};
use solana_sdk::epoch_info::EpochInfo;
use solana_transaction_status::{TransactionDetails, UiTransactionEncoding};
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};

/// Label used for the status value
pub const STATUS_LABEL: &str = "status";
/// Label used for public key
pub const PUBKEY_LABEL: &str = "pubkey";

pub struct PrometheusGauges {
    pub active_validators: IntGaugeVec,
    pub is_delinquent: GaugeVec,
    pub activated_stake: IntGaugeVec,
    pub last_vote: IntGaugeVec,
    pub root_slot: IntGaugeVec,
    pub transaction_count: IntGauge,
    pub slot_height: IntGauge,
    pub current_epoch: IntGauge,
    pub current_epoch_first_slot: IntGauge,
    pub current_epoch_last_slot: IntGauge,
    pub isp_count: IntGaugeVec,
    pub dc_count: IntGaugeVec,
    pub isp_by_stake: IntGaugeVec,
    pub dc_by_stake: IntGaugeVec,
    pub leader_slots: IntCounterVec,
    pub skipped_slot_percent: GaugeVec,
    pub current_staking_apy: GaugeVec,
    pub average_staking_apy: GaugeVec,
    pub staking_commission: IntGaugeVec,
    pub validator_rewards: IntGaugeVec,
    pub node_pubkey_balances: IntGaugeVec,
    pub node_versions: IntGaugeVec,
    pub nodes: IntGauge,
    pub average_slot_time: Gauge,
    // Connection pool for querying
    client: reqwest::Client,
    vote_accounts_whitelist: Whitelist,
}

impl PrometheusGauges {
    /// Makes new set of gauges.
    pub fn new(vote_accounts_whitelist: Whitelist) -> Self {
        Self {
            active_validators: register_int_gauge_vec!(
                "solana_active_validators",
                "Total number of active validators",
                &[STATUS_LABEL]
            )
            .unwrap(),
            is_delinquent: register_gauge_vec!(
                "solana_validator_delinquent",
                "Whether a validator is delinquent",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            activated_stake: register_int_gauge_vec!(
                "solana_validator_activated_stake",
                "Activated stake of a validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            last_vote: register_int_gauge_vec!(
                "solana_validator_last_vote",
                "Last voted slot of a validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            root_slot: register_int_gauge_vec!(
                "solana_validator_root_slot",
                "The root slot of a validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            transaction_count: register_int_gauge!(
                "solana_transaction_count",
                "Total number of confirmed transactions since genesis"
            )
            .unwrap(),
            slot_height: register_int_gauge!("solana_slot_height", "Last confirmed slot height")
                .unwrap(),
            current_epoch: register_int_gauge!("solana_current_epoch", "Current epoch").unwrap(),
            current_epoch_first_slot: register_int_gauge!(
                "solana_current_epoch_first_slot",
                "Current epoch's first slot"
            )
            .unwrap(),
            current_epoch_last_slot: register_int_gauge!(
                "solana_current_epoch_last_slot",
                "Current epoch's last slot"
            )
            .unwrap(),
            isp_count: register_int_gauge_vec!(
                "solana_active_validators_isp_count",
                "ISP of active validators",
                &["isp_name"]
            )
            .unwrap(),
            dc_count: register_int_gauge_vec!(
                "solana_active_validators_dc_count",
                "Datacenters of active validators",
                &["dc_identifier"]
            )
            .unwrap(),
            isp_by_stake: register_int_gauge_vec!(
                "solana_active_validators_isp_stake",
                "ISP of active validators grouped by stake",
                &["isp_name"]
            )
            .unwrap(),
            dc_by_stake: register_int_gauge_vec!(
                "solana_active_validators_dc_stake",
                "Datacenter of active validators grouped by stake",
                &["dc_identifier"]
            )
            .unwrap(),
            leader_slots: register_int_counter_vec!(
                "solana_leader_slots",
                "Validated and skipped leader slots per validator",
                &[PUBKEY_LABEL, STATUS_LABEL]
            )
            .unwrap(),
            skipped_slot_percent: register_gauge_vec!(
                "solana_skipped_slot_percent",
                "Skipped slot percentage per validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            current_staking_apy: register_gauge_vec!(
                "solana_current_staking_apy",
                "Staking validator APY based on last epoch's performance, in percent",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            average_staking_apy: register_gauge_vec!(
                "solana_average_staking_apy",
                "Staking validator APY averaged over a few past epochs, in percent",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            staking_commission: register_int_gauge_vec!(
                "solana_staking_commission",
                "Commission charged by staked validators",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            validator_rewards: register_int_gauge_vec!(
                "solana_validator_rewards",
                "Cumulative validator rewards in lamports",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            node_pubkey_balances: register_int_gauge_vec!(
                "solana_node_pubkey_balances",
                "Balance of node pubkeys",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            node_versions: register_int_gauge_vec!(
                "solana_node_versions",
                "Count of node versions",
                &["version"]
            )
            .unwrap(),
            nodes: register_int_gauge!("solana_nodes", "Number of nodes").unwrap(),
            average_slot_time: register_gauge!("solana_average_slot_time", "Average slot time")
                .unwrap(),
            client: reqwest::Client::new(),
            vote_accounts_whitelist,
        }
    }

    /// Exports gauges for vote accounts
    pub fn export_vote_accounts(&self, vote_accounts: &RpcVoteAccountStatus) -> anyhow::Result<()> {
        self.active_validators
            .get_metric_with_label_values(&["current"])
            .map(|m| {
                m.set(
                    vote_accounts
                        .current
                        .iter()
                        .filter(|rpc| self.vote_accounts_whitelist.contains(&rpc.vote_pubkey))
                        .count() as i64,
                )
            })?;

        self.active_validators
            .get_metric_with_label_values(&["delinquent"])
            .map(|m| {
                m.set(
                    vote_accounts
                        .delinquent
                        .iter()
                        .filter(|rpc| self.vote_accounts_whitelist.contains(&rpc.vote_pubkey))
                        .count() as i64,
                )
            })?;

        for v in vote_accounts
            .current
            .iter()
            .filter(|rpc| self.vote_accounts_whitelist.contains(&rpc.vote_pubkey))
        {
            self.is_delinquent
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(0.))?;
        }

        for v in vote_accounts
            .delinquent
            .iter()
            .filter(|rpc| self.vote_accounts_whitelist.contains(&rpc.vote_pubkey))
        {
            self.is_delinquent
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(1.))?;
        }

        for v in vote_accounts
            .current
            .iter()
            .chain(vote_accounts.delinquent.iter())
            .filter(|rpc| self.vote_accounts_whitelist.contains(&rpc.vote_pubkey))
        {
            self.activated_stake
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.activated_stake as i64))?;
            self.last_vote
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.last_vote as i64))?;
            self.root_slot
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.root_slot as i64))?;
            self.staking_commission
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.commission as i64))?;
        }

        Ok(())
    }

    /// Exports gauges for epoch
    pub fn export_epoch_info(
        &self,
        epoch_info: &EpochInfo,
        client: &RpcClient,
    ) -> anyhow::Result<()> {
        let first_slot = epoch_info.absolute_slot - epoch_info.slot_index;
        let last_slot = first_slot + epoch_info.slots_in_epoch;

        self.transaction_count
            .set(epoch_info.transaction_count.unwrap_or_default() as i64);
        self.slot_height.set(epoch_info.absolute_slot as i64);
        self.current_epoch.set(epoch_info.epoch as i64);
        self.current_epoch_first_slot.set(first_slot as i64);
        self.current_epoch_last_slot.set(last_slot as i64);

        with_first_block(client, epoch_info.epoch, |block| {
            let average_slot_time = (OffsetDateTime::now_utc().unix_timestamp()
                - client
                    .get_block_with_config(
                        block,
                        RpcBlockConfig {
                            encoding: Some(UiTransactionEncoding::Base64),
                            transaction_details: Some(TransactionDetails::None),
                            rewards: Some(false),
                            commitment: None,
                        },
                    )?
                    .block_time
                    .unwrap()) as f64
                / (epoch_info.slot_index) as f64;
            self.average_slot_time.set(average_slot_time);
            Ok(Some(()))
        })?;

        Ok(())
    }

    /// Exports information about nodes
    pub fn export_nodes_info(
        &self,
        nodes: &[RpcContactInfo],
        client: &RpcClient,
        node_whitelist: &Whitelist,
    ) -> anyhow::Result<()> {
        // Balance of node pubkeys. Only exported if a whitelist is set!
        if !node_whitelist.0.is_empty() {
            let balances = nodes
                .iter()
                .filter(|rpc| node_whitelist.contains(&rpc.pubkey))
                .map(|rpc| {
                    Ok((
                        rpc.pubkey.clone(),
                        client.get_balance(&rpc.pubkey.parse()?)?,
                    ))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            for (pubkey, balance) in balances {
                self.node_pubkey_balances
                    .get_metric_with_label_values(&[&pubkey])
                    .map(|c| c.set(balance as i64))?;
            }
        }

        let nodes = nodes
            .iter()
            .filter(|rpc| node_whitelist.contains(&rpc.pubkey))
            .collect::<Vec<_>>();

        // Export number of nodes
        self.nodes.set(nodes.len() as i64);

        // Tally of node versions
        let versions: HashMap<String, u32> = nodes.iter().fold(HashMap::new(), |mut map, rpc| {
            *map.entry(rpc.version.clone().unwrap_or_else(|| "unknown".to_string()))
                .or_insert(0) += 1;
            map
        });

        for (version, count) in versions {
            self.node_versions
                .get_metric_with_label_values(&[&version])
                .map(|c| c.set(count as i64))?;
        }

        Ok(())
    }

    /// Exports gauges for geolocation of validators
    pub async fn export_ip_addresses(
        &self,
        nodes: &[RpcContactInfo],
        vote_accounts: &RpcVoteAccountStatus,
        cache: &GeolocationCache,
        maxmind: &MaxMindAPIKey,
        node_whitelist: &Whitelist,
    ) -> anyhow::Result<()> {
        // Define all types here
        type RpcInfo = (RpcContactInfo, RpcVoteAccountInfo);
        type RpcInfoMaybeGeo = (RpcContactInfo, RpcVoteAccountInfo, Option<CityApiResponse>);
        type RpcInfoGeo = (RpcContactInfo, RpcVoteAccountInfo, CityApiResponse);

        // All nodes that are validators
        let validator_nodes = {
            // Mapping of pubkey -> vote account info.
            // Convert into a mapping to avoid O(n) searches
            let pubkeys = vote_accounts
                .current
                .iter()
                .cloned()
                .map(|vote| (vote.node_pubkey.to_string(), vote))
                .collect::<HashMap<String, RpcVoteAccountInfo>>();

            nodes
                .iter()
                .cloned()
                .filter_map(|contact| {
                    pubkeys
                        .get(&contact.pubkey)
                        .map(|vote| (contact, vote.clone()))
                })
                .collect::<Vec<RpcInfo>>()
        };

        // If whitelist exists, remove all non-listed pubkeys
        let validator_nodes = validator_nodes
            .into_iter()
            .filter(|(contact, _)| node_whitelist.contains(&contact.pubkey));

        // Separate cached data from uncached data
        let (cached, uncached): (Vec<RpcInfoMaybeGeo>, Vec<RpcInfoMaybeGeo>) = validator_nodes
            .map(|(contact, vote)| {
                let cached = cache
                    .fetch_ip_address_with_invalidation(
                        &get_rpc_contact_ip(&contact).with_context(|| {
                            format!("Validator node has no IP: {:?} {:?}", contact, vote)
                        })?,
                        |date| date + Duration::week() < OffsetDateTime::now_utc().date(),
                    )?
                    .map(|geo| geo.response);
                Ok((contact, vote, cached))
            })
            .collect::<anyhow::Result<Vec<RpcInfoMaybeGeo>>>()?
            .into_iter()
            .partition(|(_, _, db)| db.is_some());

        let mut geolocations = cached
            .into_iter()
            // This unwrap is safe because we have already partitioned into Some and Nones.
            .map(|(c, v, db)| (c, v, db.unwrap()))
            .collect::<Vec<RpcInfoGeo>>();

        // For uncached, request them from maxmind.
        debug!(
            "Uncached addresses: {:?}, cached addresses: {:?}.",
            &uncached.len(),
            &geolocations.len()
        );

        let (uncached_ok, uncached_err): (Vec<_>, Vec<_>) =
            futures::future::join_all(uncached.into_iter().map(|(contact, vote, _)| {
                debug!(
                    "Contacting Maxmind for: {:?}",
                    get_rpc_contact_ip(&contact).unwrap()
                );

                self.client
                    .get(format!(
                        "{}/{}",
                        MAXMIND_CITY_URI,
                        get_rpc_contact_ip(&contact).unwrap()
                    ))
                    .basic_auth(maxmind.username(), Some(maxmind.password()))
                    .send()
                    .and_then(|resp| resp.json::<CityApiResponse>())
                    .and_then(|json: CityApiResponse| async { Ok((contact, vote, json)) })
            }))
            .await
            .into_iter()
            .collect::<Vec<reqwest::Result<RpcInfoGeo>>>()
            .into_iter()
            .partition(Result::is_ok);

        let mut uncached = uncached_ok
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        let uncached_err = uncached_err
            .into_iter()
            .map(Result::unwrap_err)
            .collect::<Vec<reqwest::Error>>();

        for err in uncached_err {
            error!("{:?}", anyhow!(err));
        }

        // Add API requested data into database
        for (contact, _, city) in &uncached {
            cache.add_ip_address(&get_rpc_contact_ip(contact).unwrap(), &city.clone().into())?;
            debug!("Caching into DB {:?}", get_rpc_contact_ip(contact).unwrap());
        }

        // Add API requested data into collection
        geolocations.append(&mut uncached);

        // Gauges
        let mut isp_count: HashMap<String, u64> = HashMap::new();
        let mut dc_count: HashMap<DatacenterIdentifier, u64> = HashMap::new();
        let mut isp_staked: HashMap<String, u64> = HashMap::new();
        let mut dc_staked: HashMap<DatacenterIdentifier, u64> = HashMap::new();

        for (_, validator, city) in &geolocations {
            let isp = &city.traits.isp;

            // solana_active_validators_isp_count
            let c = isp_count.entry(isp.clone()).or_default();
            *c += 1;

            // solana_active_validators_dc_count
            let v = dc_count.entry(city.clone().into()).or_default();
            *v += 1;

            // solana_active_validators_isp_stake
            let s = isp_staked.entry(isp.clone()).or_default();
            *s += validator.activated_stake;

            // solana_active_validators_dc_stake
            let dc = dc_staked.entry(city.clone().into()).or_default();
            *dc += validator.activated_stake;
        }

        // Set gauges
        for (isp, count) in &isp_count {
            self.isp_count
                .get_metric_with_label_values(&[isp])
                .map(|c| c.set(*count as i64))?;
        }

        for (dc_id, count) in &dc_count {
            self.dc_count
                .get_metric_with_label_values(&[&dc_id.to_string()])
                .map(|c| c.set(*count as i64))?;
        }

        for (isp, staked) in &isp_staked {
            self.isp_by_stake
                .get_metric_with_label_values(&[isp])
                .map(|c| c.set(*staked as i64))?;
        }

        for (dc_id, staked) in &dc_staked {
            self.dc_by_stake
                .get_metric_with_label_values(&[&dc_id.to_string()])
                .map(|c| c.set(*staked as i64))?;
        }

        Ok(())
    }
}

impl Default for PrometheusGauges {
    fn default() -> Self {
        Self::new(Whitelist::default())
    }
}
