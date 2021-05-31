## Prometheus Exporter for Solana

This is a Prometheus exporter for [Solana](https://github.com/solana-labs/solana) originally based
on the [Golang original](https://github.com/certusone/solana_exporter) by CertusOne but now
providing additional functionality. It is the basis for Grafana dashboards and status alerts.

### Build

* [Install Rust](https://www.rust-lang.org/tools/install)
* `cargo build --release` or `cargo install`

### Run

Run this as a systemd service by a non-root user with a script like this one:
```
[Unit]
Description=Solana Exporter
After=solana.service
Requires=solana.service

[Service]
User=solana
Restart=always
RestartSec=20
ExecStart=/home/solana/.cargo/bin/solana-exporter

[Install]
WantedBy=multi-user.target
```

### Examples

#### Dashboard

A starting point can be our [default dashboard](./dashboards/rustiq.json).  To display pie charts we
use a [Grafana pie chart plugin](https://grafana.com/grafana/plugins/grafana-piechart-panel/). It
needs to be installed in order for the pie charts to be displayed.

#### Sample output to the Prometheus target endpoint

Some repetitive lines are ellipsed for brevity in the example below.
```
# HELP solana_active_validators Total number of active validators
# TYPE solana_active_validators gauge
solana_active_validators{status="current"} 561
solana_active_validators{status="delinquent"} 51
# HELP solana_active_validators_dc_stake Datacenter of active validators grouped by stake
# TYPE solana_active_validators_dc_stake gauge
solana_active_validators_dc_stake{dc_identifier="11427-US-Austin"} 9672542164238
solana_active_validators_dc_stake{dc_identifier="11524-US-Portland"} 35172007429
solana_active_validators_dc_stake{dc_identifier="12212-CA-Toronto"} 407342367161475
...
# HELP solana_active_validators_isp_count ISP of active validators
# TYPE solana_active_validators_isp_count gauge
solana_active_validators_isp_count{isp_name="Advanced Solutions LLC"} 1
solana_active_validators_isp_count{isp_name="Amazon.com"} 48
solana_active_validators_isp_count{isp_name="CAIW Internet"} 1
...
# HELP solana_active_validators_isp_stake ISP of active validators grouped by stake
# TYPE solana_active_validators_isp_stake gauge
solana_active_validators_isp_stake{isp_name="Advanced Solutions LLC"} 230372233054571
solana_active_validators_isp_stake{isp_name="Amazon.com"} 97432578281165840
solana_active_validators_isp_stake{isp_name="CAIW Internet"} 352505951070073
...
# HELP solana_current_epoch Current epoch
# TYPE solana_current_epoch gauge
solana_current_epoch 186
# HELP solana_current_epoch_first_slot Current epoch's first slot
# TYPE solana_current_epoch_first_slot gauge
solana_current_epoch_first_slot 80665865
# HELP solana_current_epoch_last_slot Current epoch's last slot
# TYPE solana_current_epoch_last_slot gauge
solana_current_epoch_last_slot 81097865
# HELP solana_leader_slots Validated and skipped leader slots per validator
# TYPE solana_leader_slots counter
solana_leader_slots{pubkey="12CUDzb3oe8RBQ4tYGqsuPsCbsVE4KWfktXRihXf8Ggq",status="skipped"} 54
solana_leader_slots{pubkey="12CUDzb3oe8RBQ4tYGqsuPsCbsVE4KWfktXRihXf8Ggq",status="validated"} 146
solana_leader_slots{pubkey="12oRmi8YDbqpkn326MdjwFeZ1bh3t7zVw8Nra2QK2SnR",status="skipped"} 35
solana_leader_slots{pubkey="12oRmi8YDbqpkn326MdjwFeZ1bh3t7zVw8Nra2QK2SnR",status="validated"} 217
...
```
