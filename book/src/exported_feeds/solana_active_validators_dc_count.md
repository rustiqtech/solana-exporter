# `solana_active_validators_dc_count`

## Description
The count of activate validators, grouped by their datacenter. A `dc_identifier` is a semi-unique
identifier assigned to each datacenter (see Remarks), the value is the count of active validators
who have their node IP address inside said datacenter.

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
