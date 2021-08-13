# `solana_validator_delinquent`

## Description
Whether a validator is delinquent. Given a `pubkey` label, the value (0 = false, 1 = true) shows whether a validator is delinquent.

## Sample output

```
solana_validator_delinquent{pubkey="13DmkMhdpmJJu7nU2ozAyPiKuopZbYShMHV3JAA7YVYC"} 1
solana_validator_delinquent{pubkey="13HNYUVBVHgJSfNKvgXgKia3bywzXabGzQjFyMQxLMjS"} 0
solana_validator_delinquent{pubkey="13zyX9jfGy1RvM28LcdqfLwR4VSowXx6whAL6AcFERCk"} 0
solana_validator_delinquent{pubkey="14YCghb1uYPreALx6arirtPAnoGghoPH2Ac6gCmNQdq7"} 0
solana_validator_delinquent{pubkey="1gqv7KGm888nQXsJoNFwGaDkNERUBztuekjzK3J3T7a"} 0
solana_validator_delinquent{pubkey="21ryEourynXqhpLe1DsFz8yoeFKSXE14T8bKBFmzcYzt"} 1
```

## Example usage
`count(solana_validator_delinquent == 0)` will return a time series of all active validators.

`count(solana_validator_delinquent == 1)` returns a time series of all delinquent validators. 

These two queries are equivalent to [solana_active_validators](solana_active_validators.md).
