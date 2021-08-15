# Configuration

After running `solana-exporter generate`, a template config file will be created in either the specified location or
the default directory (`~/.solana-exporter`). This page explains the individual variables and how they affect the
exporter.

## Sample configuration

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
  
## Important note on `pubkey_whitelist`
As explained above, a non-empty `pubkey_whitelist` array instructs the exporter to only export information about the
specified pubkeys. However, care should be taken if you modify the whitelist and then reload `solana-exporter`.
In particular, for the following gauges:
- `solana_current_staking_apy`
- `solana_average_staking_apy`

if any newly-appearing (e.g., formerly excluded) pubkeys were not fetched for a particular epoch, then `solana-exporter`
will not attempt to "back-fill" data for either the current or past epochs. Therefore, those pubkeys will be **missing**
from the gauges until the next epoch begins. This is due to the fact that the exporter only scrapes the ledger for
rewards data once at the beginning of every epoch.

## Overriding the config file location

- Standalone program: The default location is `~/.solana-exporter/config.toml`. Override this with the `-c` flag.
- Docker container: Change the bind-mount location.


## Overriding the database location

To speed up processing and reduce unnecessary network traffic, `solana-exporter` uses a persistent database to cache
some requests.

- Standalone program: The default location is `~/.solana-exporter/persistent.db`. Override this with the `-d` flag.
- Docker container: The location cannot be overridden, the exporter expects a database to be mounted in `/exporter/`.