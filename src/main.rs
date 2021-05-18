// Copyright 2021 Vladimir Komendantskiy
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use clap::{App, Arg};
use log::{debug, error};
use once_cell::sync::Lazy;
use prometheus_exporter::prometheus::{
    register_gauge_vec, register_int_gauge, register_int_gauge_vec, Error as PrometheusError,
    GaugeVec, IntGauge, IntGaugeVec,
};
use solana_client::{rpc_client::RpcClient, rpc_response::RpcVoteAccountStatus};
use solana_sdk::epoch_info::EpochInfo;
use std::{error::Error as StdError, fmt::Debug, net::SocketAddr, time::Duration};

const PUBKEY_LABEL: &str = "pubkey";

static ACTIVE_VALIDATORS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "solana_active_validators",
        "Total number of active validators",
        &["state"]
    )
    .unwrap()
});

static IS_DELINQUENT: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "solana_validator_delinquent",
        "Whether a validator is delinquent",
        &[PUBKEY_LABEL]
    )
    .unwrap()
});

static ACTIVATED_STAKE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "solana_validator_activated_stake",
        "Activated stake of a validator",
        &[PUBKEY_LABEL]
    )
    .unwrap()
});

static LAST_VOTE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "solana_validator_last_vote",
        "Last voted slot of a validator",
        &[PUBKEY_LABEL]
    )
    .unwrap()
});

static ROOT_SLOT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "solana_validator_last_vote",
        "Last voted slot of a validator",
        &[PUBKEY_LABEL]
    )
    .unwrap()
});

static TRANSACTION_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "solana_transaction_count",
        "Total number of confirmed transactions since genesis"
    )
    .unwrap()
});

static SLOT_HEIGHT: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("solana_slot_height", "Last confirmed slot height").unwrap());

static CURRENT_EPOCH: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("solana_current_epoch", "Current epoch").unwrap());

static CURRENT_EPOCH_FIRST_SLOT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "solana_current_epoch_first_slot",
        "Current epoch's first slot"
    )
    .unwrap()
});

static CURRENT_EPOCH_LAST_SLOT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "solana_current_epoch_last_slot",
        "Current epoch's last slot"
    )
    .unwrap()
});

static LEADER_SLOTS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "solana_leader_slots",
        "Leader slots per validator ordered by skip rate",
        &[PUBKEY_LABEL]
    )
    .unwrap()
});

/// Application config.
struct Config {
    /// Solana RPC address.
    rpc: String,
    /// Prometheus target socket address.
    target: SocketAddr,
}

fn export_vote_accounts(vote_accounts: &RpcVoteAccountStatus) -> Result<(), PrometheusError> {
    ACTIVE_VALIDATORS
        .get_metric_with_label_values(&["current"])
        .map(|m| m.set(vote_accounts.current.len() as i64))?;
    ACTIVE_VALIDATORS
        .get_metric_with_label_values(&["delinquent"])
        .map(|m| m.set(vote_accounts.delinquent.len() as i64))?;
    for v in &vote_accounts.current {
        IS_DELINQUENT
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(0.))?;
    }
    for v in &vote_accounts.delinquent {
        IS_DELINQUENT
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(1.))?;
    }
    for v in vote_accounts
        .current
        .iter()
        .chain(vote_accounts.delinquent.iter())
    {
        ACTIVATED_STAKE
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(v.activated_stake as i64))?;
        LAST_VOTE
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(v.last_vote as i64))?;
        ROOT_SLOT
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(v.root_slot as i64))?;
    }

    Ok(())
}

fn export_epoch_info(epoch_info: &EpochInfo) -> Result<(), PrometheusError> {
    let first_slot = epoch_info.absolute_slot as i64;
    let last_slot = first_slot + epoch_info.slots_in_epoch as i64;
    TRANSACTION_COUNT.set(epoch_info.transaction_count.unwrap_or_default() as i64);
    SLOT_HEIGHT.set(epoch_info.absolute_slot as i64);
    CURRENT_EPOCH.set(epoch_info.epoch as i64);
    CURRENT_EPOCH_FIRST_SLOT.set(first_slot);
    CURRENT_EPOCH_LAST_SLOT.set(last_slot);

    Ok(())
}

/// Gets config parameters from the command line.
fn cli() -> Result<Config, Box<dyn StdError>> {
    let matches = App::new("Solana Prometheus Exporter")
        .version("0.1")
        .author("Vladimir Komendantskiy <komendantsky@gmail.com>")
        .about("Publishes Solana validator metrics to Prometheus")
        .arg(
            Arg::with_name("rpc")
                .short("r")
                .long("rpc")
                .value_name("ADDRESS")
                .default_value("http://localhost:8899")
                .help("Solana RPC endpoint address")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .value_name("ADDRESS")
                .default_value("0.0.0.0:9179")
                .help("Prometheus target endpoint address")
                .takes_value(true),
        )
        .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let target: SocketAddr = matches.value_of("target").unwrap().parse()?;
    let rpc: String = matches.value_of("rpc").unwrap().to_owned();

    Ok(Config { rpc, target })
}

/// Error result logger.
trait LogErr {
    /// Logs the error result.
    fn log_err(self, msg: &str) -> Self;
}

impl<T, E: Debug> LogErr for Result<T, E> {
    fn log_err(self, msg: &str) -> Self {
        self.map_err(|e| {
            error!("{}: {:?}", msg, e);
            e
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let config = cli()?;
    let exporter = prometheus_exporter::start(config.target)?;
    let duration = Duration::from_secs(1);
    let client = RpcClient::new(config.rpc);

    loop {
        let _guard = exporter.wait_duration(duration);
        debug!("Updating metrics");
        let vote_account_status = client.get_vote_accounts()?;
        export_vote_accounts(&vote_account_status)
            .log_err("Failed to export vote account metrics")?;
        let epoch_info = client.get_epoch_info()?;
        export_epoch_info(&epoch_info).log_err("Failed to export epoch info metrics")?;
    }
}
