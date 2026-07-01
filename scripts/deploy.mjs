#!/usr/bin/env node
/**
 * Node.js deploy script — bypasses stellar-cli identity system entirely.
 * Uses @stellar/stellar-sdk directly with SOROBAN_SECRET_KEY env var.
 */
import { readFileSync } from "fs";
import {
  Keypair, Networks, SorobanRpc, TransactionBuilder,
  BASE_FEE, Operation, xdr, Address, Contract, StrKey,
} from "@stellar/stellar-sdk";

const WASM_PATH = process.argv[2] || "target/wasm32-unknown-unknown/release/zkp_private_pay.wasm";
const SECRET    = process.env.SOROBAN_SECRET_KEY;
const RPC_URL   = "https://soroban-testnet.stellar.org";
const NETWORK   = Networks.TESTNET;
const DEPLOYER  = process.env.DEPLOYER_ADDR || "GCETFRUEIILOIRR52A4S32SASLEI63LAL4KATVAKXZTLHHOXSOA3V6JS";

if (!SECRET) { console.error("SOROBAN_SECRET_KEY not set"); process.exit(1); }

const kp     = Keypair.fromSecret(SECRET);
const server = new SorobanRpc.Server(RPC_URL);

async function sendTx(tx) {
  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error("Sim: " + sim.error);
  const prep = SorobanRpc.assembleTransaction(tx, sim).build();
  prep.sign(kp);
  const res = await server.sendTransaction(prep);
  if (res.status === "ERROR") throw new Error("Send: " + JSON.stringify(res.errorResult));
  let poll = await server.getTransaction(res.hash);
  for (let i = 0; poll.status === "NOT_FOUND" && i < 40; i++) {
    await new Promise(r => setTimeout(r, 2000));
    poll = await server.getTransaction(res.hash);
  }
  if (poll.status !== "SUCCESS") throw new Error("TX failed: " + res.hash);
  return poll;
}

async function buildTx(ops) {
  const acct = await server.getAccount(kp.publicKey());
  const tx = new TransactionBuilder(acct, { fee: String(1000000), networkPassphrase: NETWORK });
  for (const op of ops) tx.addOperation(op);
  return tx.setTimeout(90).build();
}

async function main() {
  log("Deployer:", kp.publicKey());
  log("Reading WASM:", WASM_PATH);
  const wasm = readFileSync(WASM_PATH);
  log("WASM size:", wasm.length, "bytes");

  // 1. Upload WASM
  log("Step 1: Upload WASM...");
  const uploadTx = await buildTx([Operation.uploadContractWasm({ wasm })]);
  const uploadRes = await sendTx(uploadTx);
  const wasmHashVal = uploadRes.returnValue;
  if (!wasmHashVal) throw new Error("No wasm hash returned");
  const wasmHashBytes = wasmHashVal.bytes();
  log("WASM hash:", wasmHashBytes.toString("hex").slice(0, 16) + "...");

  // 2. Create contract instance
  log("Step 2: Create contract instance...");
  const salt = Buffer.alloc(32);
  salt.writeUInt32BE(Math.floor(Date.now() / 1000), 0);
  const createTx = await buildTx([
    Operation.createCustomContract({
      wasmHash: wasmHashBytes,
      address: new Address(kp.publicKey()),
      salt,
    })
  ]);
  const createRes = await sendTx(createTx);
  const returnVal = createRes.returnValue;
  if (!returnVal) throw new Error("No return value from createCustomContract");
  
  // Extract contract ID from address ScVal
  const contractAddress = returnVal.address();
  const contractBytes   = contractAddress.contractId();
  const contractId      = StrKey.encodeContract(contractBytes);
  log("Contract deployed:", contractId);

  // 3. Initialize
  log("Step 3: Initialize contract...");
  const contract = new Contract(contractId);
  const initTx = await buildTx([
    contract.call("initialize",
      new Address(DEPLOYER).toScVal(),
      xdr.ScVal.scvBytes(Buffer.alloc(32, 0x00))
    )
  ]);
  await sendTx(initTx);
  log("Contract initialized ✓");

  // 4. Whitelist deployer
  log("Step 4: Whitelist deployer...");
  const wlTx = await buildTx([
    contract.call("whitelist_institution", new Address(DEPLOYER).toScVal())
  ]);
  await sendTx(wlTx);
  log("Deployer whitelisted ✓");

  // Final output — only this line to stdout
  console.log(contractId);
}

function log(...args) { console.error(...args); }
main().catch(e => { console.error("DEPLOY ERROR:", e.message); process.exit(1); });
