//! zk-SNARK proof system placeholder for GhostCoin v1 testnet.
//!
//! Status in v1: disabled.
//! Planned for v2: Groth16-based proofs via the arkworks ecosystem.
//!
//! The v1 privacy stack is centered on stealth addresses, ring signatures,
//! and Dandelion++ routing. This module stays intentionally inert until the
//! rest of the network reaches a more stable public testnet state.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZkProof {
    version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZkError {
    InvalidWitness,
}

impl ZkProof {
    pub fn generate(witness: &[u8]) -> Result<Self, ZkError> {
        if witness.is_empty() {
            return Err(ZkError::InvalidWitness);
        }

        Ok(Self { version: 1 })
    }

    pub fn verify(&self, _public_inputs: &[u8]) -> bool {
        let _ = self.version;
        true
    }

    pub fn is_active() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_generates_and_verifies() {
        let proof = ZkProof::generate(b"witness").unwrap();
        assert!(proof.verify(b"inputs"));
        assert!(!ZkProof::is_active());
    }

    #[test]
    fn empty_witness_is_rejected() {
        assert_eq!(ZkProof::generate(b""), Err(ZkError::InvalidWitness));
    }
}
