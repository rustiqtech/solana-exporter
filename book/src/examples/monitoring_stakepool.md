# Monitoring a stakepool

Although `solana-exporter` can export statistics about an entire cluster, many of its gauges can be limited to a
particular subset of pubkeys. This can be useful for when the specified RPC node has rate-limits, and only a few pubkeys
need to be monitored.

## Example `config.toml`

```toml
rpc = 'https://api.testnet.solana.com'
target = '0.0.0.0:9179'
pubkey_whitelist = [
    "8RsYRsi6f3hiK4EhyLS22Cy5KkrNbuidVYmsaYR1Xx78",
    "9YVpEeZf8uBoUtzCFC6SSFDDqPt16uKFubNhLvGxeUDy",
    "NNetet8BiymZxMBWLRPCcNGcBPZDBeEcpgtfTSwdFPX",
    "8E9KWWqX1JMNu1YC3NptLA6M8cGqWRTccrF6T1FDnYRJ"
]

[maxmind]
username = 'username'
password = 'password'
```

This is a sample configuration file that instructs `solana-exporter` to only export statistics related to the specified
pubkeys, using a public RPC node. By not fetching information about all validators, it is possible to avoid slow scrape
times due to rate limiting.

[Be sure to understand the behaviour of APY gauges when modifying `pubkey_whitelist`](../basics/configuration.md#important-note-on-pubkey_whitelist)
.

## Monitoring using Grafana

After `solana-exporter` has been appropriately configured, set up [Prometheus](../basics/prometheus.md) and
[Grafana](../basics/grafana.md) with the sample dashboard supplied. You should see statistics on your stakepool.

TODO: Images