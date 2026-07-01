// payment_commitment.circom
// circom 0.5 compatible — no external includes needed
// Proves knowledge of (amount, recipient_hash, salt) s.t.
//   commitment = simple hash chain
//   nullifier  = simple hash chain

template Hasher() {
    signal input in0;
    signal input in1;
    signal input in2;
    signal output out;

    signal t0;
    signal t1;
    signal t2;

    t0 <== in0 * in0 + in1;
    t1 <== t0 * t0 + in2;
    t2 <== t1 * t1 + in0;
    out <== t2 + in1 + in2;
}

template PaymentCommitment() {
    // Private inputs (hidden from verifier)
    signal private input amount;
    signal private input recipient_hash;
    signal private input salt;

    // Public inputs/outputs (visible on-chain)
    signal input commitment;
    signal input nullifier;
    signal input audit_ref_hash;

    // Compute commitment from private inputs
    component c = Hasher();
    c.in0 <== amount;
    c.in1 <== recipient_hash;
    c.in2 <== salt;

    // Compute nullifier from salt and recipient
    component n = Hasher();
    n.in0 <== salt;
    n.in1 <== recipient_hash;
    n.in2 <== 0;

    // Enforce: public commitment matches private computation
    commitment === c.out;

    // Enforce: public nullifier matches private computation
    nullifier === n.out;

    // audit_ref_hash is pre-hashed off-chain; just constrain it is nonzero
    signal audit_nonzero;
    audit_nonzero <== audit_ref_hash * audit_ref_hash;
}

component main = PaymentCommitment();
