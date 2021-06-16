## Prometheus Exporter for Solana

This is a Prometheus exporter for [Solana](https://github.com/solana-labs/solana) originally based
on the [Golang original](https://github.com/certusone/solana_exporter) by CertusOne but now
providing additional functionality. It is the basis for Grafana dashboards and status alerts.

### Metrics

The tool exports the following metrics:

- `solana_active_validators`: Total number of active validators.
- `solana_validator_delinquent`: Whether a validator is delinquent.
- `solana_validator_activated_stake`: Activated stake of a validator.
- `solana_validator_last_vote`: Last voted slot of a validator.
- `solana_validator_root_slot`: The root slot of a validator.
- `solana_transaction_count`: Total number of confirmed transactions since genesis.
- `solana_slot_height`: Last confirmed slot height.
- `solana_current_epoch`: Current epoch.
- `solana_current_epoch_first_slot`: Current epoch's first slot.
- `solana_current_epoch_last_slot`: Current epoch's last slot.
- `solana_active_validators_isp_count`: ISP of active validators.
- `solana_active_validators_isp_stake`: ISP of active validators grouped by stake.
- `solana_active_validators_dc_stake`: Datacenter of active validators grouped by stake.
- `solana_leader_slots`: Validated and skipped leader slots per validator.
- `solana_skipped_slot_percent`: Skipped slot percentage per validator.

### Build

* [Install Rust](https://www.rust-lang.org/tools/install)
* `cargo build --release` or `cargo install`

### Setup

By default `solana-exporter` takes input config from `~/.solana-exporter/config.toml`. If it doesn't
exist it can be generated with template values. Here is a demo example of a config file that works with the public mainnet RPC server and doesn't require running a validator or opening a MaxMind account:
```
rpc = 'https://api.mainnet-beta.solana.com/'
target = '0.0.0.0:9179'
pubkey_whitelist = []
```

Here is a production config template for monitoring the entire network via the local validator RPC port and getting server location data from MaxMind:
```
rpc = 'http://localhost:8899/'
target = '0.0.0.0:9179'
pubkey_whitelist = []

[maxmind]
username = 'username'
password = 'password'
```

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

### Prometheus and Grafana Setup

If querying a public RPC port, `solana-exporter` can be run from anywhere, not necessarily from the
validator machine. If querying a private RPC port, install Prometheus on the validator machine. Add
the following snippet to the `scrape_configs` section of the `prometheus.yml` config file:

```
  - job_name: solana
    static_configs:
      - targets: ['localhost:9179']
```

Restart Prometheus. Now the `solana-exporter` metrics should be available to view at
`http://localhost:9179/metrics`. If running on the validator machine, it is highly advisable to only
open the metrics ports to the Grafana machine. This can be achieved with `iptables`:

```sh
sudo iptables -A INPUT -p tcp -s <Grafana IP address> --dport 9179 -j ACCEPT
sudo iptables -A INPUT -p tcp -s 0.0.0.0/0 --dport 9179 -j DROP
```

Note the order of commands. The `ACCEPT` clause should appear first in the output of
```
sudo iptables -L
```
and the `DROP` clause second. Similar clauses should be added for any other open Prometheus metrics
port.

When `solana-exporter` is used on a mainnet validator node, Grafana must always run on a different
machine to circumvent potential DDoS attacks on the validator. In the Grafana dashboard, add the
Prometheus data source `http://<Validator IP>:9090`. Then import the [default
dashboard](./dashboards/rustiq.json) using that data source.

To display pie charts we use a [Grafana pie chart
plugin](https://grafana.com/grafana/plugins/grafana-piechart-panel/). Prior to Grafana v8 it needed
to be installed in order for the pie charts to be displayed. Starting from v8 pie charts are
included.

### Examples

#### Dashboard

A starting point can be our [default dashboard](./dashboards/rustiq.json).

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
