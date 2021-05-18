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
use solana_client::rpc_client::RpcClient;
use std::{error::Error as StdError, fmt::Debug, net::SocketAddr, time::Duration};

pub mod gauges;
pub mod geolocation;

/// Application config.
struct Config {
    /// Solana RPC address.
    rpc: String,
    /// Prometheus target socket address.
    target: SocketAddr,
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

        // Get metrics we need
        let vote_accounts = client.get_vote_accounts()?;
        let epoch_info = client.get_epoch_info()?;
        let nodes = client.get_cluster_nodes()?;

        gauges
            .export_vote_accounts(&vote_accounts)
            .log_err("Failed to export vote account metrics")?;
        gauges
            .export_epoch_info(&epoch_info)
            .log_err("Failed to export epoch info metrics")?;
        gauges
            .export_ip_addresses(&nodes, &vote_accounts)
            .log_err("Failed to export IP address info metrics")?;
    }
}
