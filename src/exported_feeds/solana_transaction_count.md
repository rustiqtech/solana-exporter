# `solana_transaction_count`

## Description
Total number of confirmed transactions since genesis.

## Sample output
```
solana_transaction_count 23854763230
```

## Example usage
```
rate(solana_transaction_count[5m])
```
returns a time series of the average transactions per second (TPS) in the cluster, calculated by a rolling average over
5 minutes.