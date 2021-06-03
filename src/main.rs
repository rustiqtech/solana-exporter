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

use crate::config::{ExporterConfig, CONFIG_FILE_NAME};
use crate::gauges::PrometheusGauges;
use crate::geolocation::caching::{GeoCache, GEO_DB_CACHE_TREE_NAME};
use crate::persistent_database::{PersistentDatabase, DATABASE_FILE_NAME};
use crate::slots::SkippedSlotsMonitor;
use clap::{App, Arg};
use log::{debug, error};
use solana_client::rpc_client::RpcClient;
use std::path::Path;
use std::{fmt::Debug, fs, time::Duration};

pub mod config;
pub mod gauges;
pub mod geolocation;
pub mod persistent_database;
pub mod slots;

/// Name of directory where solana-exporter will store information
pub const EXPORTER_DATA_DIR: &str = ".solana-exporter";

/// Current version of `solana-exporter`
pub const SOLANA_EXPORTER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Command line configs passed in.
struct CliConfig {
    database_override: Option<String>,
    config_override: Option<String>,
}

/// Gets config parameters from the command line.
fn cli() -> anyhow::Result<CliConfig> {
    let matches = App::new("Solana Prometheus Exporter")
        .version("0.1")
        .author("Vladimir Komendantskiy <komendantsky@gmail.com>")
        .about("Publishes Solana validator metrics to Prometheus")
        .arg(
            Arg::with_name("database")
                .short("d")
                .long("database")
                .value_name("FILE")
                .help("Specify a persistent database location")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Specify a config file location")
                .takes_value(true),
        )
        .get_matches();

    let database_location = matches.value_of("database").map(|s| s.to_string());
    let config_location = matches.value_of("config").map(|c| c.to_string());

    Ok(CliConfig {
        database_override: database_location,
        config_override: config_location,
    })
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    // Read from CLI arguments
    let cli_config = cli()?;
    let persistent_database = {
        // Use override from CLI or default.
        let location = cli_config
            .database_override
            .map(|s| Path::new(&s).to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap()
                    .join(EXPORTER_DATA_DIR)
                    .join(DATABASE_FILE_NAME)
            });

        PersistentDatabase::new(&location)
    }?;

    let config = {
        // Use override from CLI or default.
        let location = cli_config
            .config_override
            .map(|s| Path::new(&s).to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap()
                    .join(EXPORTER_DATA_DIR)
                    .join(CONFIG_FILE_NAME)
            });

        let file_contents = fs::read_to_string(location)?;

        toml::from_str::<ExporterConfig>(&file_contents)
    }?;

    let exporter = prometheus_exporter::start(config.target)?;
    let duration = Duration::from_secs(1);
    let client = RpcClient::new(config.rpc);
    let geolocation_cache = GeoCache::new(persistent_database.tree(GEO_DB_CACHE_TREE_NAME)?);
    let gauges = PrometheusGauges::new();
    let mut skipped_slots_monitor =
        SkippedSlotsMonitor::new(&client, &gauges.leader_slots, &gauges.skipped_slot_percent);

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
            .export_ip_addresses(&nodes, &vote_accounts, &config.maxmind, &geolocation_cache)
            .await
            .log_err("Failed to export IP address info metrics")?;
        skipped_slots_monitor
            .export_skipped_slots(&epoch_info)
            .log_err("Failed to export skipped slots")?;
    }
}
