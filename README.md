## Prometheus Exporter for Solana

This is a Prometheus exporter for [Solana](https://github.com/solana-labs/solana) based on the
[Golang original](https://github.com/certusone/solana_exporter) by CertusOne. It is the basis for
Grafana dashboards and status alerts.

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
TimeoutStartSec=20
Restart=always
ExecStart=%h/.cargo/bin/solana-exporter

[Install]
WantedBy=multi-user.target
```

### Examples

#### Dashboard

A starting point can be the [dashboard by CertusOne](./dashboards/certusone.json).

#### Sample output to the Prometheus target endpoint

Some repetitive lines are ellipsed for brevity in the example below.
```
# HELP prometheus_exporter_request_duration_seconds The HTTP request latencies in seconds.
# TYPE prometheus_exporter_request_duration_seconds histogram
prometheus_exporter_request_duration_seconds_bucket{le="0.005"} 11
prometheus_exporter_request_duration_seconds_bucket{le="0.01"} 11
prometheus_exporter_request_duration_seconds_bucket{le="0.025"} 11
prometheus_exporter_request_duration_seconds_bucket{le="0.05"} 11
prometheus_exporter_request_duration_seconds_bucket{le="0.1"} 11
...
prometheus_exporter_request_duration_seconds_sum 0.0000105
prometheus_exporter_request_duration_seconds_count 11
# HELP prometheus_exporter_requests_total Number of HTTP requests received.
# TYPE prometheus_exporter_requests_total counter
prometheus_exporter_requests_total 11
# HELP prometheus_exporter_response_size_bytes The HTTP response sizes in bytes.
# TYPE prometheus_exporter_response_size_bytes gauge
prometheus_exporter_response_size_bytes 136373
# HELP solana_active_validators Total number of active validators
# TYPE solana_active_validators gauge
solana_active_validators{state="current"} 324
solana_active_validators{state="delinquent"} 36
# HELP solana_validator_activated_stake Activated stake of a validator
# TYPE solana_validator_activated_stake gauge
solana_validator_activated_stake{pubkey="13HNYUVBVHgJSfNKvgXgKia3bywzXabGzQjFyMQxLMjS"} 59998397935959
solana_validator_activated_stake{pubkey="1gqv7KGm888nQXsJoNFwGaDkNERUBztuekjzK3J3T7a"} 250002741550967
solana_validator_activated_stake{pubkey="21ryEourynXqhpLe1DsFz8yoeFKSXE14T8bKBFmzcYzt"} 2993151360
...
# HELP solana_validator_is_delinquent Whether a validator is delinquent
# TYPE solana_validator_is_delinquent gauge
solana_validator_is_delinquent{pubkey="13HNYUVBVHgJSfNKvgXgKia3bywzXabGzQjFyMQxLMjS"} 0
solana_validator_is_delinquent{pubkey="1gqv7KGm888nQXsJoNFwGaDkNERUBztuekjzK3J3T7a"} 0
solana_validator_is_delinquent{pubkey="21ryEourynXqhpLe1DsFz8yoeFKSXE14T8bKBFmzcYzt"} 1
...
# HELP solana_validator_last_vote Last voted slot of a validator
# TYPE solana_validator_last_vote gauge
solana_validator_last_vote{pubkey="13HNYUVBVHgJSfNKvgXgKia3bywzXabGzQjFyMQxLMjS"} 58788403
solana_validator_last_vote{pubkey="1gqv7KGm888nQXsJoNFwGaDkNERUBztuekjzK3J3T7a"} 58788443
solana_validator_last_vote{pubkey="21ryEourynXqhpLe1DsFz8yoeFKSXE14T8bKBFmzcYzt"} 7393690
...
# HELP solana_validator_root_slot Root slot of a validator
# TYPE solana_validator_root_slot gauge
solana_validator_root_slot{pubkey="13HNYUVBVHgJSfNKvgXgKia3bywzXabGzQjFyMQxLMjS"} 58788362
solana_validator_root_slot{pubkey="1gqv7KGm888nQXsJoNFwGaDkNERUBztuekjzK3J3T7a"} 58788379
solana_validator_root_slot{pubkey="21ryEourynXqhpLe1DsFz8yoeFKSXE14T8bKBFmzcYzt"} 7393659
...
```
