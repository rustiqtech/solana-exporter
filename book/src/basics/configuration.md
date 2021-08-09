# Configuration

After running `solana-exporter generate`, a template config file will be created.

```toml
rpc = 'http://localhost:8899'
target = '0.0.0.0:9179'
pubkey_whitelist = []

[maxmind]
username = 'username'
password = 'password'
```

- `rpc` - the location of the JSON-RPC node. This can be a local RPC node, or a public one.
    - *Remark: Public nodes usually have a rate-limiting policy in place that makes usage with `solana-exporter`
      difficult (e.g., delayed response times).*
- `target` - the target address/port to export Prometheus gauges to.
- `pubkey_whitelist` - an array of addresses, if not empty, will instruct the exporter to only export statistics about
  the specified addresses.
- `[maxmind]` - The exporter can optionally use
  MaxMind's [GeoIP2 Precision City Service](https://www.maxmind.com/en/geoip2-precision-city-service) to export
  decentralisation-related metrics. However, this requires you to sign up for a MaxMind account and regularly top-up
  your account with credits.
    - `username` - the username of the API key.
    - `password` - the password of the API key.