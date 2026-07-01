#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, BytesN, Env,
    symbol_short,
};

#[contracttype]
#[derive(Clone)]
pub struct SimpleRecord {
    pub sender: Address,
    pub commitment: BytesN<32>,
}

#[contract]
pub struct ZkpPrivatePay;

#[contractimpl]
impl ZkpPrivatePay {
    pub fn store(env: Env, sender: Address, commitment: BytesN<32>) {
        sender.require_auth();
        let record = SimpleRecord { sender, commitment };
        env.storage().instance().set(&symbol_short!("REC"), &record);
    }

    pub fn get(env: Env) -> SimpleRecord {
        env.storage().instance().get(&symbol_short!("REC")).unwrap()
    }
}
