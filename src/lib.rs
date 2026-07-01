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
    pub fn ping(_env: Env) -> u32 { 42 }
}
