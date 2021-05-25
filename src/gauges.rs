use crate::geolocation::api::{MaxMindAPIKey, MAXMIND_CITY_URI};
use crate::geolocation::caching::GeoCache;
use crate::geolocation::get_rpc_contact_ip;
use anyhow::Context;
use futures::TryFutureExt;
use geoip2_city::CityApiResponse;
use log::debug;
use prometheus_exporter::prometheus::{
    register_gauge_vec, register_int_gauge, register_int_gauge_vec, GaugeVec, IntGauge, IntGaugeVec,
};
use solana_client::rpc_response::{RpcContactInfo, RpcVoteAccountInfo, RpcVoteAccountStatus};
use solana_sdk::epoch_info::EpochInfo;
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};

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
    pub leader_slots: IntGaugeVec,
    pub isp_count: IntGaugeVec,
    pub isp_by_stake: IntGaugeVec,
    // Connection pool for querying
    client: reqwest::Client,
}

impl PrometheusGauges {
    pub fn new() -> Self {
        Self {
            active_validators: register_int_gauge_vec!(
                "solana_active_validators",
                "Total number of active validators",
                &["state"]
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
            leader_slots: register_int_gauge_vec!(
                "solana_leader_slots",
                "Leader slots per validator ordered by skip rate",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            isp_count: register_int_gauge_vec!(
                "solana_active_validators_isp_count",
                "ISP of active validators",
                &["isp_name"]
            )
            .unwrap(),
            isp_by_stake: register_int_gauge_vec!(
                "solana_active_validators_isp_stake",
                "ISP of active validators grouped by stake",
                &["isp_name"]
            )
            .unwrap(),
            client: reqwest::Client::new(),
        }
    }

    pub fn export_vote_accounts(&self, vote_accounts: &RpcVoteAccountStatus) -> anyhow::Result<()> {
        self.active_validators
            .get_metric_with_label_values(&["current"])
            .map(|m| m.set(vote_accounts.current.len() as i64))?;

        self.active_validators
            .get_metric_with_label_values(&["delinquent"])
            .map(|m| m.set(vote_accounts.delinquent.len() as i64))?;

        for v in &vote_accounts.current {
            self.is_delinquent
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(0.))?;
        }
        for v in &vote_accounts.delinquent {
            self.is_delinquent
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(1.))?;
        }
        for v in vote_accounts
            .current
            .iter()
            .chain(vote_accounts.delinquent.iter())
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
        }

        Ok(())
    }

    pub fn export_epoch_info(&self, epoch_info: &EpochInfo) -> anyhow::Result<()> {
        let first_slot = epoch_info.absolute_slot as i64;
        let last_slot = first_slot + epoch_info.slots_in_epoch as i64;

        self.transaction_count
            .set(epoch_info.transaction_count.unwrap_or_default() as i64);
        self.slot_height.set(epoch_info.absolute_slot as i64);
        self.current_epoch.set(epoch_info.epoch as i64);
        self.current_epoch_first_slot.set(first_slot);
        self.current_epoch_last_slot.set(last_slot);

        Ok(())
    }

    // For now, this will export the IP addresses of active voting accounts with a node.
    // TODO: This needs to actually export to a Prometheus gauge.
    pub async fn export_ip_addresses(
        &self,
        nodes: &[RpcContactInfo],
        vote_accounts: &RpcVoteAccountStatus,
        api_key: &MaxMindAPIKey,
        cache: &GeoCache,
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

        // Separate cached from uncached
        let (cached, uncached): (Vec<RpcInfoMaybeGeo>, Vec<RpcInfoMaybeGeo>) = validator_nodes
            .into_iter()
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
        println!(
            "Uncached addresses: {:?}, cached addresses: {:?}.",
            &uncached.len(),
            &geolocations.len()
        );

        let mut uncached =
            futures::future::join_all(uncached.into_iter().map(|(contact, vote, _)| {
                // TODO: Consider making this a function? For now this works...
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
                    .basic_auth(api_key.username(), Some(api_key.password()))
                    .send()
                    .and_then(|resp| resp.json::<CityApiResponse>())
                    .and_then(|json: CityApiResponse| async { Ok((contact, vote, json)) })
            }))
            .await
            .into_iter()
            .collect::<reqwest::Result<Vec<RpcInfoGeo>>>()?;

        // Add API requested data into database
        for (contact, _, city) in &uncached {
            cache.add_ip_address(&get_rpc_contact_ip(contact).unwrap(), &city.clone().into())?;
            debug!("Caching into DB {:?}", get_rpc_contact_ip(contact).unwrap());
        }

        // Add API requested data into collection
        geolocations.append(&mut uncached);

        // Gauges
        let mut isp_staked: HashMap<String, u64> = HashMap::new();
        let mut isp_count: HashMap<String, u64> = HashMap::new();

        for (_, validator, city) in &geolocations {
            let isp = &city.traits.isp;

            // solana_active_validators_isp_stake
            let s = isp_staked.entry(isp.clone()).or_default();
            *s += validator.activated_stake;

            // solana_active_validators_isp_count
            let c = isp_count.entry(isp.clone()).or_default();
            *c += 1;

            // TODO: solana_active_validators_data_centre_stake
        }

        // Set gauges
        for (isp, count) in &isp_count {
            self.isp_count
                .get_metric_with_label_values(&[isp])
                .map(|c| c.set(*count as i64))?;
        }

        for (isp, summation) in &isp_staked {
            self.isp_by_stake
                .get_metric_with_label_values(&[isp])
                .map(|c| c.set(*summation as i64))?;
        }

        Ok(())
    }
}

impl Default for PrometheusGauges {
    fn default() -> Self {
        Self::new()
    }
}
