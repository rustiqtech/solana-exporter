# `solana_current_staking_apy`

## Description

Staking validator APY averaged over a few past epochs (in percent).

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

To calculate the average staking APY, the exporter fetches the stored staking APY of the past few epochs and uses them.
If a validator pubkey does not appear for a particular past epoch, then that epoch is excluded from calculation -
instead of being treated as 0%.