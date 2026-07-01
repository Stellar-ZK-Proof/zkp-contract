//! ZKP Private Pay — Soroban Smart Contract
//!
//! Implements ZK-proof-gated institutional payment settlements on Stellar.
//! A payment is only executed when a valid ZK proof is submitted alongside
//! it, enabling private, audit-compliant transfers between institutions.
//!
//! Proof verification is done via a pluggable verifier key stored on-chain.
//! In production this would integrate with a Groth16 / PLONK verifier WASM
//! co-contract; here we model the interface and store commitment hashes.

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, Map, Symbol, Vec, log,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------
const ADMIN_KEY: Symbol = Symbol::short("ADMIN");
const VK_KEY: Symbol = Symbol::short("VK");           // verifier key hash
const TX_STORE: Symbol = Symbol::short("TXS");        // Map<BytesN<32>, TxRecord>
const NULLIFIERS: Symbol = Symbol::short("NULLS");    // Map<BytesN<32>, bool>
const INSTITUTION_WHITELIST: Symbol = Symbol::short("INSTLIST");

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Status of a private payment
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum PaymentStatus {
    Pending,
    Settled,
    Rejected,
}

/// On-chain record of a ZKP-gated payment commitment
#[contracttype]
#[derive(Clone)]
pub struct TxRecord {
    /// Pedersen commitment to (amount, recipient, salt)
    pub commitment: BytesN<32>,
    /// Sender institution (public — compliance anchor)
    pub sender: Address,
    /// Timestamp of submission
    pub timestamp: u64,
    /// Settlement status
    pub status: PaymentStatus,
    /// Nullifier spent to prevent replay
    pub nullifier: BytesN<32>,
    /// Optional audit memo hash (e.g. SWIFT ref hashed)
    pub audit_ref_hash: BytesN<32>,
}

/// ZK proof bundle submitted by the prover
#[contracttype]
#[derive(Clone)]
pub struct ZkProof {
    /// Groth16 / PLONK compressed proof bytes
    pub proof_bytes: Bytes,
    /// Public inputs: [commitment, nullifier, audit_ref_hash]
    pub public_inputs: Vec<BytesN<32>>,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct ZkpPrivatePay;

#[contractimpl]
impl ZkpPrivatePay {
    // -----------------------------------------------------------------------
    // Initialisation
    // -----------------------------------------------------------------------

    /// Deploy and set the contract admin and initial verifier key hash.
    pub fn initialize(env: Env, admin: Address, verifier_key_hash: BytesN<32>) {
        if env.storage().instance().has(&ADMIN_KEY) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&VK_KEY, &verifier_key_hash);

        // Init empty stores
        let txs: Map<BytesN<32>, TxRecord> = Map::new(&env);
        env.storage().instance().set(&TX_STORE, &txs);
        let nulls: Map<BytesN<32>, bool> = Map::new(&env);
        env.storage().instance().set(&NULLIFIERS, &nulls);
        let whitelist: Map<Address, bool> = Map::new(&env);
        env.storage().instance().set(&INSTITUTION_WHITELIST, &whitelist);

        log!(&env, "ZKP Private Pay initialized, admin={}", admin);
    }

    // -----------------------------------------------------------------------
    // Admin
    // -----------------------------------------------------------------------

    /// Update the verifier key (e.g. after a trusted setup ceremony).
    pub fn update_vk(env: Env, new_vk_hash: BytesN<32>) {
        Self::require_admin(&env);
        env.storage().instance().set(&VK_KEY, &new_vk_hash);
        log!(&env, "VK updated");
    }

    /// Whitelist an institution address allowed to submit payments.
    pub fn whitelist_institution(env: Env, institution: Address) {
        Self::require_admin(&env);
        let mut wl: Map<Address, bool> =
            env.storage().instance().get(&INSTITUTION_WHITELIST).unwrap();
        wl.set(institution.clone(), true);
        env.storage().instance().set(&INSTITUTION_WHITELIST, &wl);
        log!(&env, "Institution whitelisted: {}", institution);
    }

    /// Remove an institution from the whitelist.
    pub fn delist_institution(env: Env, institution: Address) {
        Self::require_admin(&env);
        let mut wl: Map<Address, bool> =
            env.storage().instance().get(&INSTITUTION_WHITELIST).unwrap();
        wl.remove(institution.clone());
        env.storage().instance().set(&INSTITUTION_WHITELIST, &wl);
    }

    // -----------------------------------------------------------------------
    // Core payment flow
    // -----------------------------------------------------------------------

    /// Submit a private payment commitment. The sender is revealed (compliance)
    /// but amount and recipient are hidden in the `commitment`.
    ///
    /// Returns the tx_id (Blake2b hash of commitment + timestamp).
    pub fn submit_payment(
        env: Env,
        sender: Address,
        commitment: BytesN<32>,
        nullifier: BytesN<32>,
        audit_ref_hash: BytesN<32>,
    ) -> BytesN<32> {
        sender.require_auth();
        Self::require_whitelisted(&env, &sender);

        // Nullifier must be fresh
        let mut nulls: Map<BytesN<32>, bool> =
            env.storage().instance().get(&NULLIFIERS).unwrap();
        if nulls.get(nullifier.clone()).unwrap_or(false) {
            panic!("nullifier already spent");
        }

        let timestamp = env.ledger().timestamp();
        let tx_id = Self::compute_tx_id(&env, &commitment, timestamp);

        let record = TxRecord {
            commitment,
            sender,
            timestamp,
            status: PaymentStatus::Pending,
            nullifier: nullifier.clone(),
            audit_ref_hash,
        };

        let mut txs: Map<BytesN<32>, TxRecord> =
            env.storage().instance().get(&TX_STORE).unwrap();
        txs.set(tx_id.clone(), record);
        env.storage().instance().set(&TX_STORE, &txs);

        // Reserve nullifier slot
        nulls.set(nullifier, true);
        env.storage().instance().set(&NULLIFIERS, &nulls);

        log!(&env, "Payment submitted, tx_id={:?}", tx_id);
        tx_id
    }

    /// Settle a pending payment by providing a valid ZK proof.
    ///
    /// In production the proof is verified against the on-chain VK using a
    /// co-contract call. Here we verify that:
    ///  1. The proof public inputs match the on-chain commitment & nullifier.
    ///  2. proof_bytes are non-empty (placeholder for full verifier call).
    pub fn settle_payment(env: Env, tx_id: BytesN<32>, proof: ZkProof) {
        let mut txs: Map<BytesN<32>, TxRecord> =
            env.storage().instance().get(&TX_STORE).unwrap();
        let mut record = txs.get(tx_id.clone()).expect("tx not found");

        if record.status != PaymentStatus::Pending {
            panic!("tx already settled or rejected");
        }

        // Validate public inputs match stored commitment + nullifier
        if proof.public_inputs.len() < 3 {
            panic!("insufficient public inputs");
        }
        let pi_commitment = proof.public_inputs.get(0).unwrap();
        let pi_nullifier = proof.public_inputs.get(1).unwrap();
        let pi_audit = proof.public_inputs.get(2).unwrap();

        if pi_commitment != record.commitment {
            panic!("commitment mismatch");
        }
        if pi_nullifier != record.nullifier {
            panic!("nullifier mismatch");
        }
        if pi_audit != record.audit_ref_hash {
            panic!("audit ref mismatch");
        }

        // Proof must contain bytes (full verifier call goes here in prod)
        if proof.proof_bytes.is_empty() {
            panic!("empty proof");
        }

        // TODO(prod): call verifier co-contract
        // let verified: bool = env.invoke_contract(
        //     &verifier_contract_id,
        //     &Symbol::short("verify"),
        //     vec![&env, proof.proof_bytes.into_val(&env),
        //               proof.public_inputs.into_val(&env),
        //               vk_hash.into_val(&env)],
        // );
        // if !verified { panic!("invalid proof"); }

        record.status = PaymentStatus::Settled;
        txs.set(tx_id.clone(), record);
        env.storage().instance().set(&TX_STORE, &txs);

        log!(&env, "Payment settled: {:?}", tx_id);
    }

    /// Reject a pending payment (admin only — compliance override).
    pub fn reject_payment(env: Env, tx_id: BytesN<32>) {
        Self::require_admin(&env);
        let mut txs: Map<BytesN<32>, TxRecord> =
            env.storage().instance().get(&TX_STORE).unwrap();
        let mut record = txs.get(tx_id.clone()).expect("tx not found");
        if record.status != PaymentStatus::Pending {
            panic!("tx already finalized");
        }
        record.status = PaymentStatus::Rejected;
        txs.set(tx_id.clone(), record);
        env.storage().instance().set(&TX_STORE, &txs);
        log!(&env, "Payment rejected: {:?}", tx_id);
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub fn get_tx(env: Env, tx_id: BytesN<32>) -> TxRecord {
        let txs: Map<BytesN<32>, TxRecord> =
            env.storage().instance().get(&TX_STORE).unwrap();
        txs.get(tx_id).expect("tx not found")
    }

    pub fn is_nullifier_spent(env: Env, nullifier: BytesN<32>) -> bool {
        let nulls: Map<BytesN<32>, bool> =
            env.storage().instance().get(&NULLIFIERS).unwrap();
        nulls.get(nullifier).unwrap_or(false)
    }

    pub fn get_vk_hash(env: Env) -> BytesN<32> {
        env.storage().instance().get(&VK_KEY).unwrap()
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN_KEY).unwrap()
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();
    }

    fn require_whitelisted(env: &Env, addr: &Address) {
        let wl: Map<Address, bool> =
            env.storage().instance().get(&INSTITUTION_WHITELIST).unwrap();
        if !wl.get(addr.clone()).unwrap_or(false) {
            panic!("institution not whitelisted");
        }
    }

    /// Derive a deterministic tx_id from commitment bytes + ledger timestamp.
    fn compute_tx_id(env: &Env, commitment: &BytesN<32>, timestamp: u64) -> BytesN<32> {
        let mut input = Bytes::new(env);
        input.append(&commitment.clone().into());
        // Append timestamp bytes
        let ts_bytes = timestamp.to_be_bytes();
        for b in ts_bytes.iter() {
            input.push_back(*b);
        }
        env.crypto().sha256(&input)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{vec, Env};

    fn create_test_env() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let institution = Address::generate(&env);
        (env, admin, institution)
    }

    fn dummy_hash(env: &Env, seed: u8) -> BytesN<32> {
        let mut b = Bytes::new(env);
        for _ in 0..32 {
            b.push_back(seed);
        }
        env.crypto().sha256(&b)
    }

    #[test]
    fn test_initialize_and_whitelist() {
        let (env, admin, institution) = create_test_env();
        let contract_id = env.register_contract(None, ZkpPrivatePay);
        let client = ZkpPrivatePayClient::new(&env, &contract_id);

        let vk_hash = dummy_hash(&env, 0xAB);
        client.initialize(&admin, &vk_hash);
        client.whitelist_institution(&institution);

        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.get_vk_hash(), vk_hash);
    }

    #[test]
    fn test_submit_and_settle_payment() {
        let (env, admin, institution) = create_test_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
            ..Default::default()
        });

        let contract_id = env.register_contract(None, ZkpPrivatePay);
        let client = ZkpPrivatePayClient::new(&env, &contract_id);

        let vk_hash = dummy_hash(&env, 0x01);
        client.initialize(&admin, &vk_hash);
        client.whitelist_institution(&institution);

        let commitment = dummy_hash(&env, 0x02);
        let nullifier = dummy_hash(&env, 0x03);
        let audit_ref = dummy_hash(&env, 0x04);

        let tx_id =
            client.submit_payment(&institution, &commitment, &nullifier, &audit_ref);

        let record = client.get_tx(&tx_id);
        assert_eq!(record.status, PaymentStatus::Pending);

        // Build proof with matching public inputs
        let proof = ZkProof {
            proof_bytes: Bytes::from_array(&env, &[1u8; 128]),
            public_inputs: vec![&env, commitment.clone(), nullifier.clone(), audit_ref.clone()],
        };
        client.settle_payment(&tx_id, &proof);

        let settled = client.get_tx(&tx_id);
        assert_eq!(settled.status, PaymentStatus::Settled);
    }

    #[test]
    #[should_panic(expected = "nullifier already spent")]
    fn test_replay_attack_blocked() {
        let (env, admin, institution) = create_test_env();
        let contract_id = env.register_contract(None, ZkpPrivatePay);
        let client = ZkpPrivatePayClient::new(&env, &contract_id);

        client.initialize(&admin, &dummy_hash(&env, 0x01));
        client.whitelist_institution(&institution);

        let commitment = dummy_hash(&env, 0x02);
        let nullifier = dummy_hash(&env, 0x03);
        let audit_ref = dummy_hash(&env, 0x04);

        client.submit_payment(&institution, &commitment, &nullifier, &audit_ref);
        // Second attempt with same nullifier should panic
        client.submit_payment(&institution, &commitment, &nullifier, &audit_ref);
    }
}
