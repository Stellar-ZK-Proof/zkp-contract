# zkp-contract

> Soroban smart contract for ZKP Private Pay — ZK-gated private institutional payments on Stellar

## Stack
- Rust + Soroban SDK 21
- Groth16 proof public-input verification
- Nullifier-based replay protection

## Contract methods

| Method | Access | Description |
|---|---|---|
| `initialize` | Admin | Deploy: set admin + verifier key hash |
| `whitelist_institution` | Admin | Allow an address to submit payments |
| `submit_payment` | Institution | Store commitment + nullifier on-chain |
| `settle_payment` | Anyone | Settle by providing valid ZK proof |
| `reject_payment` | Admin | Compliance override |
| `get_tx` | View | Query a transaction record |
| `is_nullifier_spent` | View | Replay-attack check |

## Build & deploy

```bash
# Build
stellar contract build

# Test
cargo test

# Deploy to testnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/zkp_private_pay.wasm \
  --network testnet \
  --source <YOUR_SECRET_KEY>
```

## Related repos
- [zkp-frontend](https://github.com/Stellar-ZK-Proof/zkp-frontend)
- [zkp-backend](https://github.com/Stellar-ZK-Proof/zkp-backend)
