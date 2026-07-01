#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, BytesN, Env, Map,
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

#[contract]
pub struct ZkpPrivatePay;

#[contractimpl]
impl ZkpPrivatePay {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("ADMIN"), &admin);
        env.storage().instance().set(&symbol_short!("TXS"), &Map::<BytesN<32>, TxRecord>::new(&env));
        env.storage().instance().set(&symbol_short!("NULLS"), &Map::<BytesN<32>, bool>::new(&env));
        env.storage().instance().set(&symbol_short!("WL"), &Map::<Address, bool>::new(&env));
    }

    pub fn whitelist(env: Env, institution: Address) {
        let admin: Address = env.storage().instance().get(&symbol_short!("ADMIN")).unwrap();
        admin.require_auth();
        let mut wl: Map<Address, bool> = env.storage().instance().get(&symbol_short!("WL")).unwrap();
        wl.set(institution, true);
        env.storage().instance().set(&symbol_short!("WL"), &wl);
    }

    pub fn submit(env: Env, sender: Address, commitment: BytesN<32>,
        nullifier: BytesN<32>, audit_ref_hash: BytesN<32>) {
        sender.require_auth();
        let mut txs: Map<BytesN<32>, TxRecord> = env.storage().instance().get(&symbol_short!("TXS")).unwrap();
        let record = TxRecord { commitment: commitment.clone(), sender, timestamp: env.ledger().timestamp(),
            status: PaymentStatus::Pending, nullifier, audit_ref_hash };
        txs.set(commitment, record);
        env.storage().instance().set(&symbol_short!("TXS"), &txs);
    }
}
