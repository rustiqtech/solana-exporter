# `solana_active_validators_isp_stake`

## Description
The sum of stakes held by active validators, grouped by their ISP. Given `isp_name` is the name of the ISP returned by
MaxMind, the value is the sum of stake (in lamports) in active validators who have a node IP address belonging that ISP.

## Sample output
```
solana_active_validators_isp_stake{isp_name="7heaven LLC"} 188227282425345
solana_active_validators_isp_stake{isp_name="Adman LLC"} 6189723438718
solana_active_validators_isp_stake{isp_name="Advanced Solutions LLC"} 118047122517759
solana_active_validators_isp_stake{isp_name="Alibaba"} 164798876213690
solana_active_validators_isp_stake{isp_name="Amazon"} 6114765640910
solana_active_validators_isp_stake{isp_name="Amazon.com"} 135796402624727330
solana_active_validators_isp_stake{isp_name="Beeline"} 180442822928047
```

## Remarks
This gauge will not be exported if no MaxMind API key is present in `config.toml`.

## Caching
The output of this gauge relies on cached data; the exporter retains the geolocation information of an IP address
for one week before considering it stale and re-acquiring it from MaxMind.