# `solana_active_validators_dc_stake`

## Description
The sum of stakes held by active validators, grouped by their datacenter location. Given `dc_identifier` is a
semi-unique identifier assigned to each datacenter (see below), the value is the sum of stake (in lamports)
in active validators who have a node IP address inside said datacenter.

## Sample output
```
solana_active_validators_dc_stake{dc_identifier="11524-US-Portland"} 945063491010
solana_active_validators_dc_stake{dc_identifier="12212-CA-Toronto"} 393894227085511
solana_active_validators_dc_stake{dc_identifier="132203-US-Santa Clara"} 138086443681311
solana_active_validators_dc_stake{dc_identifier="13830-US-Dallas"} 386674347619072
solana_active_validators_dc_stake{dc_identifier="138982-CN"} 3058846690875748
solana_active_validators_dc_stake{dc_identifier="14618-US"} 6114765640910
solana_active_validators_dc_stake{dc_identifier="14618-US-Ashburn"} 210033359690576
solana_active_validators_dc_stake{dc_identifier="15169-BE-Brussels"} 737964600374534
```

## Remarks
This gauge will not be exported if no MaxMind API key is present in `config.toml`.

The identifier is of the format:
```
{AS number}-{ISO-3166-1 Alpha-2 code}-{City name}
```
if a city name is available, otherwise:
```
{AS number}-{ISO-3166-1 Alpha-2 code}
```

## Caching
The output of this gauge relies on cached data; the exporter retains the geolocation information of an IP address
for one week before considering it stale and re-acquiring it from MaxMind.