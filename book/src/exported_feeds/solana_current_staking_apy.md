# `solana_current_staking_apy`

## Description
Staking validator APY based on last epoch's performance (in percent).

## Sample output

```
TODO: Remove whitelist!
```

## Remarks
Be sure to understand this gauge's behaviour
when [`pubkey_whitelist` is modified](../basics/configuration.md#important-note-on-pubkey_whitelist).

## Caching
At the beginning of each epoch, the exporter fetches all reward transactions from the starting slots of the epoch. The
staking rewards, and the duration of the *previous* epoch, are used to calculate the APY of the current epoch. This is
only ever done once per epoch.