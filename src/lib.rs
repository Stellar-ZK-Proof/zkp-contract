#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, Bytes, BytesN, Env, Map, Vec,
    symbol_short,
};

#[contracttype] #[derive(Clone, PartialEq, Debug)]
pub enum PaymentStatus { Pending, Settled, Rejected }

#[contracttype] #[derive(Clone)]
pub struct TxRecord {
    pub commitment: BytesN<32>, pub sender: Address,
    pub timestamp: u64, pub status: PaymentStatus,
    pub nullifier: BytesN<32>, pub audit_ref_hash: BytesN<32>,
}

#[contracttype] #[derive(Clone)]
pub struct ZkProof { pub proof_bytes: Bytes, pub public_inputs: Vec<BytesN<32>> }

#[contract]
pub struct ZkpPrivatePay;

#[contractimpl]
impl ZkpPrivatePay {
    pub fn initialize(env: Env, admin: Address, verifier_key_hash: BytesN<32>) {
        if env.storage().instance().has(&symbol_short!("ADMIN")) { panic!("already initialized"); }
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("ADMIN"), &admin);
        env.storage().instance().set(&symbol_short!("VK"), &verifier_key_hash);
        env.storage().instance().set(&symbol_short!("TXS"), &Map::<BytesN<32>, TxRecord>::new(&env));
        env.storage().instance().set(&symbol_short!("NULLS"), &Map::<BytesN<32>, bool>::new(&env));
        env.storage().instance().set(&symbol_short!("WL"), &Map::<Address, bool>::new(&env));
    }

    pub fn whitelist_institution(env: Env, institution: Address) {
        Self::require_admin(&env);
        let mut wl: Map<Address, bool> = env.storage().instance().get(&symbol_short!("WL")).unwrap();
        wl.set(institution, true);
        env.storage().instance().set(&symbol_short!("WL"), &wl);
    }

    pub fn submit_payment(env: Env, sender: Address, commitment: BytesN<32>,
        nullifier: BytesN<32>, audit_ref_hash: BytesN<32>) -> BytesN<32> {
        sender.require_auth();
        Self::require_whitelisted(&env, &sender);
        let mut nulls: Map<BytesN<32>, bool> = env.storage().instance().get(&symbol_short!("NULLS")).unwrap();
        if nulls.get(nullifier.clone()).unwrap_or(false) { panic!("nullifier already spent"); }
        let timestamp = env.ledger().timestamp();
        let tx_id = Self::compute_tx_id(&env, &commitment, timestamp);
        let record = TxRecord { commitment, sender, timestamp,
            status: PaymentStatus::Pending, nullifier: nullifier.clone(), audit_ref_hash };
        let mut txs: Map<BytesN<32>, TxRecord> = env.storage().instance().get(&symbol_short!("TXS")).unwrap();
        txs.set(tx_id.clone(), record);
        env.storage().instance().set(&symbol_short!("TXS"), &txs);
        nulls.set(nullifier, true);
        env.storage().instance().set(&symbol_short!("NULLS"), &nulls);
        tx_id
    }

    pub fn settle_payment(env: Env, tx_id: BytesN<32>, proof: ZkProof) {
        let mut txs: Map<BytesN<32>, TxRecord> = env.storage().instance().get(&symbol_short!("TXS")).unwrap();
        let mut record = txs.get(tx_id.clone()).expect("tx not found");
        if record.status != PaymentStatus::Pending { panic!("tx already finalized"); }
        if proof.public_inputs.len() < 3 { panic!("insufficient public inputs"); }
        if proof.proof_bytes.is_empty() { panic!("empty proof"); }
        if proof.public_inputs.get(0).unwrap() != record.commitment { panic!("commitment mismatch"); }
        if proof.public_inputs.get(1).unwrap() != record.nullifier { panic!("nullifier mismatch"); }
        if proof.public_inputs.get(2).unwrap() != record.audit_ref_hash { panic!("audit ref mismatch"); }
        record.status = PaymentStatus::Settled;
        txs.set(tx_id, record);
        env.storage().instance().set(&symbol_short!("TXS"), &txs);
    }

    pub fn reject_payment(env: Env, tx_id: BytesN<32>) {
        Self::require_admin(&env);
        let mut txs: Map<BytesN<32>, TxRecord> = env.storage().instance().get(&symbol_short!("TXS")).unwrap();
        let mut record = txs.get(tx_id.clone()).expect("tx not found");
        if record.status != PaymentStatus::Pending { panic!("tx already finalized"); }
        record.status = PaymentStatus::Rejected;
        txs.set(tx_id, record);
        env.storage().instance().set(&symbol_short!("TXS"), &txs);
    }

    pub fn get_tx(env: Env, tx_id: BytesN<32>) -> TxRecord {
        env.storage().instance().get::<_, Map<BytesN<32>, TxRecord>>(&symbol_short!("TXS"))
            .unwrap().get(tx_id).expect("tx not found")
    }

    pub fn is_nullifier_spent(env: Env, nullifier: BytesN<32>) -> bool {
        env.storage().instance().get::<_, Map<BytesN<32>, bool>>(&symbol_short!("NULLS"))
            .unwrap().get(nullifier).unwrap_or(false)
    }

    pub fn get_vk_hash(env: Env) -> BytesN<32> {
        env.storage().instance().get(&symbol_short!("VK")).unwrap()
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&symbol_short!("ADMIN")).unwrap()
    }

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&symbol_short!("ADMIN")).unwrap();
        admin.require_auth();
    }

    fn require_whitelisted(env: &Env, addr: &Address) {
        let wl: Map<Address, bool> = env.storage().instance().get(&symbol_short!("WL")).unwrap();
        if !wl.get(addr.clone()).unwrap_or(false) { panic!("institution not whitelisted"); }
    }

    fn compute_tx_id(env: &Env, commitment: &BytesN<32>, timestamp: u64) -> BytesN<32> {
        let mut input = Bytes::new(env);
        input.append(&commitment.clone().into());
        for b in timestamp.to_be_bytes().iter() { input.push_back(*b); }
        env.crypto().sha256(&input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{vec, Env};

    fn dummy(env: &Env, seed: u8) -> BytesN<32> {
        let mut b = Bytes::new(env);
        for _ in 0..32 { b.push_back(seed); }
        env.crypto().sha256(&b)
    }

    #[test]
    fn test_full_payment_flow() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| { l.timestamp = 1_700_000_000; });
        let admin = Address::generate(&env);
        let inst  = Address::generate(&env);
        let cid   = env.register_contract(None, ZkpPrivatePay);
        let c     = ZkpPrivatePayClient::new(&env, &cid);
        c.initialize(&admin, &dummy(&env, 1));
        c.whitelist_institution(&inst);
        let commitment = dummy(&env, 2);
        let nullifier  = dummy(&env, 3);
        let audit      = dummy(&env, 4);
        let tx_id = c.submit_payment(&inst, &commitment, &nullifier, &audit);
        assert_eq!(c.get_tx(&tx_id).status, PaymentStatus::Pending);
        let proof = ZkProof {
            proof_bytes: Bytes::from_array(&env, &[1u8; 128]),
            public_inputs: vec![&env, commitment, nullifier, audit],
        };
        c.settle_payment(&tx_id, &proof);
        assert_eq!(c.get_tx(&tx_id).status, PaymentStatus::Settled);
    }

    #[test]
    #[should_panic(expected = "nullifier already spent")]
    fn test_replay_blocked() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let inst  = Address::generate(&env);
        let cid   = env.register_contract(None, ZkpPrivatePay);
        let c     = ZkpPrivatePayClient::new(&env, &cid);
        c.initialize(&admin, &dummy(&env, 1));
        c.whitelist_institution(&inst);
        c.submit_payment(&inst, &dummy(&env, 2), &dummy(&env, 3), &dummy(&env, 4));
        c.submit_payment(&inst, &dummy(&env, 2), &dummy(&env, 3), &dummy(&env, 4));
    }
}
