# `solana_active_validators_isp_count`

## Description
The count of activate validators, grouped by their ISP. Given the `isp_name` label is the name of the ISP returned by MaxMind, the value is the
number of validators with a node IP address belonging to that ISP.

## Sample output
```
solana_active_validators_isp_count{isp_name="7heaven LLC"} 1
solana_active_validators_isp_count{isp_name="Adman LLC"} 1
solana_active_validators_isp_count{isp_name="Advanced Solutions LLC"} 1
solana_active_validators_isp_count{isp_name="Alibaba"} 1
solana_active_validators_isp_count{isp_name="Amazon"} 1
solana_active_validators_isp_count{isp_name="Amazon.com"} 61
solana_active_validators_isp_count{isp_name="Beeline"} 1
```

## Remarks
This gauge will not be exported if no MaxMind API key is present in `config.toml`.
