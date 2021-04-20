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

#[macro_use]
extern crate lazy_static;

use clap::{App, Arg};
use log::{debug, error};
use prometheus_exporter::prometheus::{
    register_gauge_vec, register_int_gauge_vec, Error as PrometheusError, GaugeVec, IntGaugeVec,
};
use solana_client::{rpc_client::RpcClient, rpc_response::RpcVoteAccountStatus};
use std::{error::Error as StdError, fmt::Debug, net::SocketAddr, time::Duration};

const PUBKEY_LABEL: &'static str = "pubkey";

lazy_static! {
    static ref ACTIVE_VALIDATORS: IntGaugeVec = register_int_gauge_vec!(
        "solana_active_validators",
        "Total number of active validators",
        &["state"]
    )
    .unwrap();
    static ref IS_DELINQUENT: GaugeVec = register_gauge_vec!(
        "solana_validator_delinquent",
        "Whether a validator is delinquent",
        &[PUBKEY_LABEL]
    )
    .unwrap();
    static ref ACTIVATED_STAKE: IntGaugeVec = register_int_gauge_vec!(
        "solana_validator_activated_stake",
        "Activated stake of a validator",
        &[PUBKEY_LABEL]
    )
    .unwrap();
    static ref LAST_VOTE: IntGaugeVec = register_int_gauge_vec!(
        "solana_validator_last_vote",
        "Last voted slot of a validator",
        &[PUBKEY_LABEL]
    )
    .unwrap();
    static ref ROOT_SLOT: IntGaugeVec = register_int_gauge_vec!(
        "solana_validator_root_slot",
        "Root slot of a validator",
        &[PUBKEY_LABEL]
    )
    .unwrap();
}

/// Application config.
struct Config {
    /// Solana RPC address.
    rpc: String,
    /// Prometheus target socket address.
    target: SocketAddr,
}

fn export_metrics(vote_accounts: &RpcVoteAccountStatus) -> Result<(), PrometheusError> {
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
        export_metrics(&vote_account_status).log_err("Failed to export metrics")?;
    }
}
