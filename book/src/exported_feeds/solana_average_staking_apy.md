# `solana_average_staking_apy`

## Description

The APY of a given vote account pubkey averaged over a few past epochs (in percent).

## Sample output

```
solana_average_staking_apy{pubkey="5BAi9YGCipHq4ZcXuen5vagRQqRTVTRszXNqBZC6uBPZ"} 2.544996416812742
solana_average_staking_apy{pubkey="8jxSHbS4qAnh5yueFp4D9ABXubKqMwXqF3HtdzQGuphp"} 2.5342297952374553
solana_average_staking_apy{pubkey="F5b1wSUtpaYDnpjLQonCZC7iyFvizLcNqTactZbwSEXK"} 2.8351747690563456
solana_average_staking_apy{pubkey="irKsY8c3sQur1XaYuQ811hzsEQJ5Hq3Yu3AAoXYnp8W"} 1.7458550503327919
```

## Remarks

Be sure to understand this gauge's behaviour
when [`vote_account_whitelist` is modified](../basics/configuration.md#important-note-on-vote_account_whitelist-and-staking_account_whitelist).

## Caching

At the beginning of each epoch, the exporter fetches all reward transactions from the starting slots of the epoch. The
staking rewards, and the duration of the *previous* epoch, are used to calculate the APY of the current epoch. This is
only ever done once per epoch.

To calculate the average staking APY, the exporter fetches the stored staking APY of the past few epochs and uses them.
If a validator pubkey does not appear for a particular past epoch, then that epoch is excluded from calculation -
instead of being treated as 0%.
