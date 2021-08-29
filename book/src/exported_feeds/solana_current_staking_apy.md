# `solana_current_staking_apy`

## Description

The APY of a given vote account pubkey based on last epoch's performance (in percent).

## Sample output

```
solana_current_staking_apy{pubkey="5BAi9YGCipHq4ZcXuen5vagRQqRTVTRszXNqBZC6uBPZ"} 6.449820442689558
solana_current_staking_apy{pubkey="8jxSHbS4qAnh5yueFp4D9ABXubKqMwXqF3HtdzQGuphp"} 6.434121594142694
solana_current_staking_apy{pubkey="F5b1wSUtpaYDnpjLQonCZC7iyFvizLcNqTactZbwSEXK"} 7.195850076956045
solana_current_staking_apy{pubkey="irKsY8c3sQur1XaYuQ811hzsEQJ5Hq3Yu3AAoXYnp8W"} 3.2552769395926884
```

## Remarks
Be sure to understand this gauge's behaviour
when [`pubkey_whitelist` is modified](../basics/configuration.md#important-note-on-pubkey_whitelist).

## Caching
At the beginning of each epoch, the exporter fetches all reward transactions from the starting slots of the epoch. The
staking rewards, and the duration of the *previous* epoch, are used to calculate the APY of the current epoch. This is
only ever done once per epoch.
