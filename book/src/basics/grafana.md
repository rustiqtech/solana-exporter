# Grafana

After `solana-exporter` exports gauges and metrics to Prometheus, it may be useful to visualise these metrics.
[Grafana](https://grafana.com/) allows you to create custom dashboards using Prometheus as a data source.

The repository includes a basic dashboard (`rustiq.json`) that shows the basic gauges that `solana-exporter` can
produce, as well as an advanced dashboard (`rustiq2.json`) that shows off more gauges that `solana-exporter` produces.

TODO: Screenshots of the basic and advanced dashboard.

For an in-depth explanation of each exported gauge, see then next chapter