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

use crate::gauges::PrometheusGauges;
use clap::{App, Arg};
use log::{debug, error};
use prometheus_exporter::prometheus::Error as PrometheusError;
use solana_client::{rpc_client::RpcClient, rpc_response::RpcVoteAccountStatus};
use solana_sdk::epoch_info::EpochInfo;
use std::{error::Error as StdError, fmt::Debug, net::SocketAddr, time::Duration};

pub mod gauges;

/// Application config.
struct Config {
    /// Solana RPC address.
    rpc: String,
    /// Prometheus target socket address.
    target: SocketAddr,
}

fn export_vote_accounts(
    gauges: &PrometheusGauges,
    vote_accounts: &RpcVoteAccountStatus,
) -> Result<(), PrometheusError> {
    gauges
        .active_validators
        .get_metric_with_label_values(&["current"])
        .map(|m| m.set(vote_accounts.current.len() as i64))?;

    gauges
        .active_validators
        .get_metric_with_label_values(&["delinquent"])
        .map(|m| m.set(vote_accounts.delinquent.len() as i64))?;

    for v in &vote_accounts.current {
        gauges
            .is_delinquent
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(0.))?;
    }
    for v in &vote_accounts.delinquent {
        gauges
            .is_delinquent
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(1.))?;
    }
    for v in vote_accounts
        .current
        .iter()
        .chain(vote_accounts.delinquent.iter())
    {
        gauges
            .activated_stake
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(v.activated_stake as i64))?;
        gauges
            .last_vote
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(v.last_vote as i64))?;
        gauges
            .root_slot
            .get_metric_with_label_values(&[&*v.vote_pubkey])
            .map(|m| m.set(v.root_slot as i64))?;
    }

    Ok(())
}

fn export_epoch_info(
    gauges: &PrometheusGauges,
    epoch_info: &EpochInfo,
) -> Result<(), PrometheusError> {
    let first_slot = epoch_info.absolute_slot as i64;
    let last_slot = first_slot + epoch_info.slots_in_epoch as i64;

    gauges
        .transaction_count
        .set(epoch_info.transaction_count.unwrap_or_default() as i64);
    gauges.slot_height.set(epoch_info.absolute_slot as i64);
    gauges.current_epoch.set(epoch_info.epoch as i64);
    gauges.current_epoch_first_slot.set(first_slot);
    gauges.current_epoch_last_slot.set(last_slot);

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

    let gauges = PrometheusGauges::default();

    loop {
        let _guard = exporter.wait_duration(duration);
        debug!("Updating metrics");
        let vote_account_status = client.get_vote_accounts()?;
        export_vote_accounts(&gauges, &vote_account_status)
            .log_err("Failed to export vote account metrics")?;
        let epoch_info = client.get_epoch_info()?;
        export_epoch_info(&gauges, &epoch_info).log_err("Failed to export epoch info metrics")?;
    }
}
