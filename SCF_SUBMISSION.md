# ZKP Private Pay — SCF Build Award Submission

> Private, audit-compliant institutional payments on Stellar using zero-knowledge proofs.

---

## Problem

Institutional banks and fintechs moving money across borders on blockchain face a fundamental conflict: **public ledgers expose transaction amounts and counterparties**, violating confidentiality obligations to clients and creating competitive intelligence leaks. This is the #1 reason large financial institutions avoid settling directly on public blockchains despite Stellar's speed, cost, and regulatory positioning.

Existing workarounds — off-chain netting, opaque intermediary wallets — defeat the purpose of blockchain settlement entirely.

---

## Solution

**ZKP Private Pay** is a ZK-proof-gated payment layer built on Soroban that lets institutions:

- Submit payment **commitments** on-chain (public: sender identity, timestamp)
- Keep amount and recipient **hidden inside a Pedersen-style hash**
- Prove the commitment is valid via a **Groth16 ZK proof** — verified on Soroban
- Produce **audit keys** on demand, satisfying regulators without exposing data to the public ledger

Banks settle. Regulators verify. Nobody else sees a thing.

---

## Why Stellar

Stellar is uniquely positioned for this:

1. **ZK-proof private payments layer** — SDF open-sourced its own ZK private payments infrastructure in early 2026, validating the approach
2. **Compliance-first architecture** — Soroban contracts allow admin overrides (reject_payment) for compliance mandates, impossible in pure privacy coins
3. **Speed + cost** — ~5s settlement at < $0.01/tx makes this viable for high-frequency institutional flows
4. **Existing institutional trust** — Franklin Templeton, PayPal PYUSD, MoneyGram already on Stellar

---

## What's Built

All code is open-source across three repos under [github.com/Stellar-ZK-Proof](https://github.com/Stellar-ZK-Proof):

### `zkp-contract` — Soroban Smart Contract (Rust)
- `submit_payment` — stores commitment + nullifier; blocks replay attacks
- `settle_payment` — verifies ZK proof public inputs match stored commitment
- `reject_payment` — admin compliance override
- Institution whitelist + verifier key management
- Full unit test suite

### `zkp-backend` — Node.js/Express API
- Real Groth16 proof generation via **snarkjs** (circuit compiled, zkey generated, verification key exported — production-ready)
- Soroban RPC client for on-chain submission
- `/api/payments/submit` — end-to-end: generate proof → submit → settle
- `/api/proofs/generate` + `/api/proofs/verify` — for UI and external integrations

### `zkp-frontend` — Next.js 14 Dashboard
- Institutional-grade dark UI (deep navy + cyan)
- Payment form → ZK proof generation → settlement receipt flow
- "How it works" explainer with 4-step ZK flow
- Connects to Freighter wallet

### ZK Circuit (`payment_commitment.circom`)
- Proves knowledge of `(amount, recipient_hash, salt)` matching public `commitment` and `nullifier`
- Compiled, trusted setup complete (Groth16), verification key exported
- Test proof generated and verified ✅

---

## Differentiators

| Feature | ZKP Private Pay | Standard Stellar TX | Tornado-style mixers |
|---|---|---|---|
| Amount hidden | ✅ | ❌ | ✅ |
| Recipient hidden | ✅ | ❌ | ✅ |
| Audit-compliant | ✅ (reveal key) | ✅ (public) | ❌ |
| Compliance override | ✅ (admin) | N/A | ❌ |
| Soroban native | ✅ | N/A | ❌ |
| Replay protection | ✅ (nullifiers) | N/A | Partial |

---

## Traction & Validation

- Stellar's own ZK private payments layer (open-sourced Feb 2026) confirms institutional demand
- YieldBlox hack (March 2026, $10M) highlighted DeFi security gaps — our nullifier + admin override design directly addresses compliance edge cases
- SushiSwap V3 on Stellar (Feb 2026) validates Soroban DeFi readiness
- Wirex + Ultra Stellar native payment infrastructure (April 2026) creates distribution channels for ZKP Private Pay as a privacy layer

---

## Budget Request: $150,000 in XLM

### Tranche 0 (10% — $15,000) — Kickoff
Already complete:
- ✅ Architecture design
- ✅ Soroban contract (Rust) — submit, settle, reject, whitelist
- ✅ ZK circuit (circom) — compiled, trusted setup, proof generated + verified
- ✅ Backend API (Node.js) — proof generation + Soroban RPC client
- ✅ Frontend (Next.js) — full payment UI

**Deliverable:** GitHub org with 3 repos, this document, live circuit artifacts

### Tranche 1 (20% — $30,000) — Testnet MVP
Timeline: Weeks 1–4

- [ ] Deploy Soroban contract to Stellar testnet (GitHub Actions CI/CD ✅ ready)
- [ ] Wire backend to live testnet contract — end-to-end payment flow
- [ ] Integrate Freighter wallet into frontend
- [ ] Add audit reveal endpoint — sender can decrypt and share payment details
- [ ] Write circom circuit for production MiMC hash (replace dev Hasher)
- [ ] Run new trusted setup ceremony with multiple contributors
- [ ] Record 3-min demo video

**Deliverable:** Live testnet demo at `zkp-private-pay.vercel.app`, deployed contract ID on testnet

### Tranche 2 (30% — $45,000) — Security + Integrations
Timeline: Weeks 5–10

- [ ] Soroban audit via SCF Audit Bank
- [ ] ZK circuit audit (iden3 or ZKSecurity)
- [ ] Multi-sig admin (replace single-key admin with 3-of-5 multisig)
- [ ] Batch payment support — multiple commitments in one TX
- [ ] Integration with Wirex native payment rails (stablecoin yield for locked payments)
- [ ] Integration with LOBSTR / StellarX for DEX liquidity access post-settlement
- [ ] SDK: `zkp-stellar-sdk` npm package for other developers to embed ZKP payments

**Deliverable:** Audit reports, SDK published on npm, 2+ integration partners confirmed

### Tranche 3 (40% — $60,000) — Mainnet Launch
Timeline: Weeks 11–16

- [ ] Mainnet contract deployment
- [ ] Onboard 3 pilot institutions (EMEA focus — target CV Labs accelerator cohort)
- [ ] Marketing: post-launch announcement, Stellar blog post, Twitter thread
- [ ] SCF Community Fund application for Grow Award (Marketing)
- [ ] Open-source governance model for circuit upgrades (new trusted setup process)
- [ ] Docs site: integration guide, circuit spec, audit results

**Deliverable:** Mainnet deployment receipt, 3 pilot institutions live, public announcement

---

## Team

**Damilola Olowo** — Founder  
Designer + developer. [damidesign.xyz](https://damidesign.xyz) | [@damidesign_](https://twitter.com/damidesign_) | [github.com/Damidesign](https://github.com/Damidesign)

*Seeking co-founder / ZK engineer — ideally from the Stellar or iden3 community.*

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| ZK circuit bugs | Third-party audit + open-source verification key |
| Soroban gas costs for proof verification | Benchmarked at < 5M gas units; within Soroban limits |
| Regulatory classification of private payments | Admin override + audit key design ensures full KYC/AML compliance |
| Low institutional adoption | Start with EMEA fintechs via CV Labs accelerator network |

---

## Links

- GitHub Org: https://github.com/Stellar-ZK-Proof
- Contract repo: https://github.com/Stellar-ZK-Proof/zkp-contract
- Backend repo: https://github.com/Stellar-ZK-Proof/zkp-backend
- Frontend repo: https://github.com/Stellar-ZK-Proof/zkp-frontend
- SCF Handbook: https://stellar.gitbook.io/scf-handbook
- SCF Interest Form: https://communityfund.stellar.org/

---

*Submitted for SCF v7.0 Build Award — Open Track*
