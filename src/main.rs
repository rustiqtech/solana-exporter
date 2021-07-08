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
use crate::geolocation::api::MaxMindAPIKey;
use crate::geolocation::caching::{GeolocationCache, GEO_DB_CACHE_TREE_NAME};
use crate::persistent_database::{PersistentDatabase, DATABASE_FILE_NAME};
use crate::slots::SkippedSlotsMonitor;
use anyhow::Context;
use clap::{load_yaml, App};
use log::{debug, warn};
use solana_client::rpc_client::RpcClient;
use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::{fs, time::Duration};

pub mod config;
pub mod epoch_credits;
pub mod gauges;
pub mod geolocation;
pub mod persistent_database;
pub mod slots;

/// Name of directory where solana-exporter will store information
pub const EXPORTER_DATA_DIR: &str = ".solana-exporter";

/// Current version of `solana-exporter`
pub const SOLANA_EXPORTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    // Read from CLI arguments
    let yaml = load_yaml!("cli.yml");
    let cli_configs = App::from_yaml(yaml).get_matches();

    // Subcommands
    match cli_configs.subcommand() {
        ("generate", Some(sc)) => {
            let template_config = ExporterConfig {
                rpc: "http://localhost:8899".to_string(),
                target: SocketAddr::new("0.0.0.0".parse()?, 9179),
                maxmind: Some(MaxMindAPIKey::new("username", "password")),
                pubkey_whitelist: HashSet::default(),
            };

            let location = sc
                .value_of("output")
                .map(|s| Path::new(s).to_path_buf())
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .unwrap()
                        .join(EXPORTER_DATA_DIR)
                        .join(CONFIG_FILE_NAME)
                });

            // Only attempt to create .solana-exporter, if user specified location then don't try
            // to create directories
            if sc.value_of("output").is_none() {
                create_dir_all(&location.parent().unwrap())?;
            }

            let mut file = File::create(location)?;
            file.write_all(toml::to_string_pretty(&template_config)?.as_ref())?;
            std::process::exit(0);
        }

        (_, _) => {}
    }

    let persistent_database = {
        // Use override from CLI or default.
        let location = cli_configs
            .value_of("database")
            .map(|s| Path::new(s).to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap()
                    .join(EXPORTER_DATA_DIR)
                    .join(DATABASE_FILE_NAME)
            });

        // Show warning if database not found, since sled will make a new file?
        if !location.exists() {
            warn!("Database could not found at specified location. A new one will be generated!")
        }

        PersistentDatabase::new(&location)
    }?;

    let config = {
        // Use override from CLI or default.
        let location = cli_configs
            .value_of("config")
            .map(|s| Path::new(s).to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap()
                    .join(EXPORTER_DATA_DIR)
                    .join(CONFIG_FILE_NAME)
            });

        let file_contents = fs::read_to_string(location).context(
            "Could not find config file in specified location. \
If running for the first time, run `solana-exporter generate` to initialise the config file \
and then put real values there.",
        )?;

        toml::from_str::<ExporterConfig>(&file_contents)
    }?;

    let exporter = prometheus_exporter::start(config.target)?;
    let duration = Duration::from_secs(1);
    let client = RpcClient::new(config.rpc.clone());
    let geolocation_cache =
        GeolocationCache::new(persistent_database.tree(GEO_DB_CACHE_TREE_NAME)?);
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
            .context("Failed to export vote account metrics")?;
        gauges
            .export_epoch_info(&epoch_info)
            .context("Failed to export epoch info metrics")?;
        gauges.export_nodes_info(&nodes, &client, &config.pubkey_whitelist)?;
        if let Some(maxmind) = config.maxmind.clone() {
            // If the MaxMind API is configured, submit queries for any uncached IPs.
            gauges
                .export_ip_addresses(
                    &nodes,
                    &vote_accounts,
                    &geolocation_cache,
                    &config.pubkey_whitelist,
                    &maxmind,
                )
                .await
                .context("Failed to export IP address info metrics")?;
        }
        skipped_slots_monitor
            .export_skipped_slots(&epoch_info)
            .context("Failed to export skipped slots")?;
    }
}
