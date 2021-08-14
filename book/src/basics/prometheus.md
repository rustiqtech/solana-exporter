# Setting up Prometheus

`solana-exporter` uses Prometheus to export, monitor and aggregate data for use in other utilities. Refer to
[Prometheus](https://prometheus.io/)' documentation on how to get it running on your machine.

If querying a public RPC port, `solana-exporter` can be run from anywhere, not necessarily from the
validator machine.

If querying a private RPC port, install Prometheus on the validator machine. Add
the following snippet to the `scrape_configs` section of the `prometheus.yml` config file:

```yaml
  - job_name: solana
    static_configs:
      - targets: ['localhost:9179']
```

Restart Prometheus. Now the `solana-exporter` metrics should be available to view at
`http://localhost:9179/metrics`. If running on the validator machine, it is highly advisable to only
open the Prometheus datasource port to the Grafana machine. This can be achieved with `nft`. Here is
an example `/etc/nftables.conf`:

```
#!/usr/sbin/nft -f

flush ruleset

table inet filter {
    chain input {
        type filter hook input priority 0;
        # allow connection to Prometheus datasource only locally and from Grafana
        ip saddr { 127.0.0.1, <Grafana IP> } tcp dport 9090 accept
        tcp dport 9090 drop
        # allow connection to Prometheus exporter endpoints only internally
        ip saddr != 127.0.0.1 tcp dport 9100 drop
        ip saddr != 127.0.0.1 tcp dport 9179 drop
    }
    chain forward {
        type filter hook forward priority 0;
    }
    chain output {
        type filter hook output priority 0;
    }
}
```

Note the order of commands. An `accept` clause should appear before the corresponding `drop` clause.

When `solana-exporter` is used on a mainnet validator node, Grafana must always run on a different
machine to circumvent potential DDoS attacks on the validator. In the Grafana dashboard, add the
Prometheus data source `http://<Validator IP>:9090`. Then import the `rustiq.json` using that data source.