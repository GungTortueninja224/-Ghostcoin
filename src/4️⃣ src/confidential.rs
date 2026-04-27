use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;

fn get_h_point() -> RistrettoPoint {
    let mut hasher = Sha256::new();
    hasher.update(b"privacy_chain_H_point_v1");
    let bytes = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Scalar::from_bytes_mod_order(arr) * RISTRETTO_BASEPOINT_POINT
}

pub struct PedersenCommitment {
    pub commitment: RistrettoPoint,
    pub blinding:   Scalar,
    pub amount:     u64,
}

impl PedersenCommitment {
    pub fn new(amount: u64) -> Self {
        let mut rng = OsRng;
        let blinding   = Scalar::random(&mut rng);
        let commitment = Self::compute(amount, blinding);
        Self { commitment, blinding, amount }
    }

    pub fn compute(amount: u64, blinding: Scalar) -> RistrettoPoint {
        let h = get_h_point();
        Scalar::from(amount) * h + blinding * RISTRETTO_BASEPOINT_POINT
    }
}

pub struct ConfidentialTxResult {
    pub input_commitment:  RistrettoPoint,
    pub output_commitment: RistrettoPoint,
    pub change_commitment: RistrettoPoint,
    pub output_blinding:   Scalar,
    pub change_blinding:   Scalar,
}

pub fn create_confidential_tx(
    input_amount:   u64,
    input_blinding: Scalar,
    send_amount:    u64,
) -> Option<ConfidentialTxResult> {
    if send_amount > input_amount { return None; }
    let change_amount = input_amount - send_amount;
    let mut rng = OsRng;
    let output_blinding = Scalar::random(&mut rng);
    let change_blinding = Scalar::random(&mut rng);
    Some(ConfidentialTxResult {
        input_commitment:  PedersenCommitment::compute(input_amount,  input_blinding),
        output_commitment: PedersenCommitment::compute(send_amount,   output_blinding),
        change_commitment: PedersenCommitment::compute(change_amount, change_blinding),
        output_blinding,
        change_blinding,
    })
}