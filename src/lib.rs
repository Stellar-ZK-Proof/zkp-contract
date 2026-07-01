#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol, symbol_short};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn ping(env: Env) -> Symbol {
        symbol_short!("pong")
    }
}
