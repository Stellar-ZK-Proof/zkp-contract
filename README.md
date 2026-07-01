# zkp-contract

> Soroban smart contract for ZKP Private Pay — ZK-gated private institutional payments on Stellar

[![Build & Deploy](https://github.com/Stellar-ZK-Proof/zkp-contract/actions/workflows/build-deploy.yml/badge.svg)](https://github.com/Stellar-ZK-Proof/zkp-contract/actions/workflows/build-deploy.yml)

## Overview

This contract stores **payment commitments** on Stellar. Amount and recipient stay hidden inside a hash — only the sender is public (for compliance). A Groth16 ZK proof must be provided to settle any payment, proving the commitment is well-formed without revealing private inputs.

## Contract methods

| Method | Access | Description |
|---|---|---|
| `initialize` | One-time | Set admin + verifier key hash |
| `whitelist_institution` | Admin | Allow an address to submit payments |
| `submit_payment` | Institution | Store commitment + nullifier on-chain |
| `settle_payment` | Anyone | Settle by providing valid ZK proof |
| `reject_payment` | Admin | Compliance override |
| `get_tx` | View | Query a transaction record |
| `is_nullifier_spent` | View | Replay-attack check |

## ZK Circuit

The circuit is in `circuits/payment_commitment.circom`.

**Private inputs:** `amount`, `recipient_hash`, `salt`  
**Public inputs:** `commitment`, `nullifier`, `audit_ref_hash`

Trusted setup is complete. Test proof and public signals are committed in `circuits/`.

## Build & Deploy

### Local (requires Rust 1.81+ and Stellar CLI)

```bash
# Install Stellar CLI
cargo install --locked stellar-cli@22.0.1 --features opt

# Build
stellar contract build

# Test
cargo test

# Deploy to testnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/zkp_private_pay.wasm \
  --source <YOUR_KEY_NAME> \
  --network testnet
```

### GitHub Actions (recommended)

Push to `main` → contract is built and tested automatically.  
To deploy to testnet, trigger the **Build & Deploy** workflow manually with `deploy: true`.

Add `STELLAR_SECRET_KEY` to your repo's GitHub Secrets first.

## Related repos
- [zkp-frontend](https://github.com/Stellar-ZK-Proof/zkp-frontend)
- [zkp-backend](https://github.com/Stellar-ZK-Proof/zkp-backend)

## SCF Submission
See [SCF_SUBMISSION.md](./SCF_SUBMISSION.md) for the full SCF Build Award pitch.
