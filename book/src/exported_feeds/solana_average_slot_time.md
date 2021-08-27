# `solana_average_slot_time`

## Description

The average slot time in the current epoch, in seconds.

## Sample output

```
solana_average_slot_time 0.5642300440729666
```

## Remarks
The exporter calculates this metric using the slot index of the current epoch and their respective timestamps.
Therefore, when using whitelists, this gauge will not reflect the performance of the whitelisted vote pubkeys.