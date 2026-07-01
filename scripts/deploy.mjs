#!/usr/bin/env node
/**
 * Deploy ZKP Private Pay to Stellar testnet
 * Compatible with @stellar/stellar-sdk v16+
 */
import { createRequire } from "module";
const require = createRequire(import.meta.url);
const {
  Keypair, Networks, rpc, TransactionBuilder,
  Operation, xdr, Address, Contract, StrKey
} = require("@stellar/stellar-sdk");

const { Server, assembleTransaction, Api } = rpc;

const WASM_PATH = process.argv[2] || "target/wasm32-unknown-unknown/release/zkp_private_pay.wasm";
const SECRET    = process.env.SOROBAN_SECRET_KEY;
const RPC_URL   = "https://soroban-testnet.stellar.org";
const NET       = Networks.TESTNET;
const DEPLOYER  = process.env.DEPLOYER_ADDR || "GA2LCOB7EO77Q52NO4R3TJ2UTAV7NG3P7S26QUV2YSMIE7UHNUWKBE7V";
const FEE       = "1000000";

if (!SECRET) { log("SOROBAN_SECRET_KEY not set"); process.exit(1); }

const kp     = Keypair.fromSecret(SECRET);
const server = new Server(RPC_URL, { allowHttp: false });

function log(...a) { process.stderr.write(a.join(" ") + "\n"); }

async function sendTx(tx) {
  const sim = await server.simulateTransaction(tx);
  if (!Api.isSimulationSuccess(sim)) {
    throw new Error("Sim failed: " + JSON.stringify(sim.error || sim));
  }
  const prep = assembleTransaction(tx, sim).build();
  prep.sign(kp);
  const res = await server.sendTransaction(prep);
  if (res.status === "ERROR") throw new Error("Send: " + JSON.stringify(res.errorResult));
  log("  TX:", res.hash, "polling...");
  for (let i = 0; i < 40; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const p = await server.getTransaction(res.hash);
    if (p.status === Api.GetTransactionStatus.SUCCESS) { log("  confirmed ✓"); return p; }
    if (p.status === Api.GetTransactionStatus.FAILED) throw new Error("TX failed: " + res.hash);
  }
  throw new Error("TX timeout: " + res.hash);
}

async function buildTx(op) {
  const acct = await server.getAccount(kp.publicKey());
  return new TransactionBuilder(acct, { fee: FEE, networkPassphrase: NET })
    .addOperation(op).setTimeout(90).build();
}

async function main() {
  const { readFileSync } = await import("fs");
  log("Deployer:", kp.publicKey());
  const wasm = readFileSync(WASM_PATH);
  log("WASM:", wasm.length, "bytes");

  log("Step 1: Upload WASM...");
  const up = await sendTx(await buildTx(Operation.uploadContractWasm({ wasm })));
  if (!up.returnValue) throw new Error("No return from uploadContractWasm");
  const wasmHash = up.returnValue.bytes();
  log("Hash:", wasmHash.toString("hex").slice(0,16) + "...");

  log("Step 2: Create contract instance...");
  const salt = Buffer.alloc(32);
  salt.writeUInt32BE(Math.floor(Date.now() / 1000) & 0xFFFFFFFF, 0);
  const cr = await sendTx(await buildTx(
    Operation.createCustomContract({ wasmHash, salt })
  ));
  if (!cr.returnValue) throw new Error("No return from createCustomContract");
  log("Return type:", cr.returnValue.switch()?.name);

  let contractId;
  try {
    contractId = StrKey.encodeContract(cr.returnValue.address().contractId());
  } catch(e1) {
    log("address() failed:", e1.message, "trying bytes()...");
    try { contractId = StrKey.encodeContract(cr.returnValue.bytes()); }
    catch(e2) { throw new Error("Cannot extract contract ID: " + e2.message); }
  }
  log("Contract:", contractId);

  log("Step 3: Initialize...");
  const c = new Contract(contractId);
  await sendTx(await buildTx(c.call("initialize",
    new Address(DEPLOYER).toScVal(),
    xdr.ScVal.scvBytes(Buffer.alloc(32, 0))
  )));
  log("Initialized ✓");

  log("Step 4: Whitelist deployer...");
  await sendTx(await buildTx(c.call("whitelist_institution", new Address(DEPLOYER).toScVal())));
  log("Whitelisted ✓");

  log("=== DONE ===");
  process.stdout.write(contractId + "\n");
}

main().catch(e => {
  log("FATAL:", e.message);
  log(e.stack || "");
  process.exit(1);
});
