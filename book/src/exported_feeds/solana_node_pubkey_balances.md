# `solana_node_pubkey_balances`

## Description

Balances of node identity accounts in lamports. Those balances are used to fund vote transactions
produced by the voting nodes. Since a zero balance would lead to the node identity account being
garbage-collected, one can set a reminder alarm to refill that account.

## Sample output

```
solana_node_pubkey_balances{pubkey="4YGgmwyqztpJeAi3pzHQ4Gf9cWrMHCjZaWeWoCK6zz6X"} 6792793021
solana_node_pubkey_balances{pubkey="FoigPJ6kL6Gth5Er6t9d1Nkh96Skadqw63Ciyjxc1f8H"} 33408113791
solana_node_pubkey_balances{pubkey="G2TBEh2ahNGS9tGnuBNyDduNjyfUtGhMcssgRb8b6KfH"} 170569140828
solana_node_pubkey_balances{pubkey="zeroT6PTAEjipvZuACTh1mbGCqTHgA6i1ped9DcuidX"} 224893658626
```
