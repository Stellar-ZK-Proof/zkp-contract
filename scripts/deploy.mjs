#!/usr/bin/env node
/**
 * Node.js deploy script for ZKP Private Pay
 * Uses @stellar/stellar-sdk directly - no stellar-cli needed
 */
import { readFileSync } from "fs";
import sdk from "@stellar/stellar-sdk";

const { Keypair, Networks, SorobanRpc, TransactionBuilder, Operation, xdr, Address, Contract, StrKey } = sdk;

const WASM_PATH   = process.argv[2] || "target/wasm32-unknown-unknown/release/zkp_private_pay.wasm";
const SECRET      = process.env.SOROBAN_SECRET_KEY;
const RPC_URL     = "https://soroban-testnet.stellar.org";
const NET         = Networks.TESTNET;
const DEPLOYER    = process.env.DEPLOYER_ADDR || "GA2LCOB7EO77Q52NO4R3TJ2UTAV7NG3P7S26QUV2YSMIE7UHNUWKBE7V";
const FEE         = "1000000"; // 0.1 XLM

if (!SECRET) { log("ERROR: SOROBAN_SECRET_KEY not set"); process.exit(1); }

const kp     = Keypair.fromSecret(SECRET);
const server = new SorobanRpc.Server(RPC_URL, { allowHttp: false });

function log(...a) { process.stderr.write(a.join(" ") + "\n"); }

async function sendTx(tx) {
  const sim = await server.simulateTransaction(tx);
  if (!SorobanRpc.Api.isSimulationSuccess(sim)) {
    throw new Error("Simulation failed: " + JSON.stringify(sim.error || sim));
  }
  const prep = SorobanRpc.assembleTransaction(tx, sim).build();
  prep.sign(kp);
  
  const res = await server.sendTransaction(prep);
  if (res.status === "ERROR") throw new Error("Send error: " + JSON.stringify(res.errorResult));
  
  log("  TX hash:", res.hash, "- polling...");
  for (let i = 0; i < 40; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const poll = await server.getTransaction(res.hash);
    if (poll.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
      log("  TX confirmed ✓");
      return poll;
    }
    if (poll.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error("TX failed: " + res.hash);
    }
  }
  throw new Error("TX timeout: " + res.hash);
}

async function buildTx(op) {
  const acct = await server.getAccount(kp.publicKey());
  return new TransactionBuilder(acct, { fee: FEE, networkPassphrase: NET })
    .addOperation(op)
    .setTimeout(90)
    .build();
}

async function main() {
  log("Deployer:", kp.publicKey());
  log("WASM path:", WASM_PATH);
  
  const wasm = readFileSync(WASM_PATH);
  log("WASM size:", wasm.length, "bytes");

  // Step 1: Upload WASM
  log("Step 1: Uploading WASM...");
  const uploadRes = await sendTx(await buildTx(
    Operation.uploadContractWasm({ wasm })
  ));
  
  if (!uploadRes.returnValue) throw new Error("No return value from uploadContractWasm");
  const wasmHash = uploadRes.returnValue.bytes();
  log("WASM hash:", wasmHash.toString("hex"));

  // Step 2: Create contract instance
  log("Step 2: Creating contract instance...");
  const salt = Buffer.alloc(32);
  const ts = Math.floor(Date.now() / 1000);
  salt.writeUInt32BE(ts & 0xFFFFFFFF, 0);
  salt.writeUInt32BE((ts >> 32) & 0xFFFFFFFF, 4);
  
  const createRes = await sendTx(await buildTx(
    Operation.createCustomContract({ wasmHash, salt })
  ));
  
  if (!createRes.returnValue) throw new Error("No return value from createCustomContract");
  // The return value is an ScVal address
  const addrScVal = createRes.returnValue;
  log("Return value type:", addrScVal._switch.name);
  
  let contractId;
  try {
    // Try to get contract ID from the address ScVal
    const contractAddr = addrScVal.address();
    const contractBytes = contractAddr.contractId();
    contractId = StrKey.encodeContract(contractBytes);
  } catch (e) {
    log("Could not extract from address ScVal, trying bytes:", e.message);
    // Maybe it returns bytes directly
    try {
      const contractBytes = addrScVal.bytes();
      contractId = StrKey.encodeContract(contractBytes);
    } catch (e2) {
      throw new Error("Cannot extract contract ID: " + e2.message + "\nScVal: " + JSON.stringify(addrScVal));
    }
  }
  log("Contract ID:", contractId);

  // Step 3: Initialize
  log("Step 3: Initializing...");
  const contract = new Contract(contractId);
  await sendTx(await buildTx(
    contract.call("initialize",
      new Address(DEPLOYER).toScVal(),
      xdr.ScVal.scvBytes(Buffer.alloc(32, 0x00))
    )
  ));
  log("Initialized ✓");

  // Step 4: Whitelist
  log("Step 4: Whitelisting deployer...");
  await sendTx(await buildTx(
    contract.call("whitelist_institution", new Address(DEPLOYER).toScVal())
  ));
  log("Whitelisted ✓");

  log("=== DEPLOYMENT COMPLETE ===");
  // Only contract ID to stdout
  process.stdout.write(contractId + "\n");
}

main().catch(e => {
  log("FATAL ERROR:", e.message);
  if (e.stack) log(e.stack);
  process.exit(1);
});
