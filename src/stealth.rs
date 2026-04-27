use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use rand::RngCore;

fn random_scalar() -> Scalar {
    let mut rng = OsRng;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    Scalar::from_bytes_mod_order(bytes)
}

pub struct RecipientKeypair {
    pub scan_private:  Scalar,
    pub scan_public:   RistrettoPoint,
    pub spend_private: Scalar,
    pub spend_public:  RistrettoPoint,
}

impl RecipientKeypair {
    pub fn generate() -> Self {
        let scan_private  = random_scalar();
        let spend_private = random_scalar();
        Self {
            scan_public:  scan_private  * RISTRETTO_BASEPOINT_POINT,
            spend_public: spend_private * RISTRETTO_BASEPOINT_POINT,
            scan_private,
            spend_private,
        }
    }
}

pub struct StealthPayment {
    pub one_time_address: RistrettoPoint,
    pub ephemeral_public: RistrettoPoint,
}

pub fn generate_stealth_address(recipient: &RecipientKeypair) -> StealthPayment {
    let ephemeral_private = random_scalar();
    let ephemeral_public  = ephemeral_private * RISTRETTO_BASEPOINT_POINT;
    let shared_secret     = ephemeral_private * recipient.scan_public;
    let hs                = hash_to_scalar(&shared_secret);
    let one_time_address  = hs * RISTRETTO_BASEPOINT_POINT + recipient.spend_public;
    StealthPayment { one_time_address, ephemeral_public }
}

pub fn scan_for_payment(recipient: &RecipientKeypair, payment: &StealthPayment) -> bool {
    let shared_secret = recipient.scan_private * payment.ephemeral_public;
    let hs            = hash_to_scalar(&shared_secret);
    let expected      = hs * RISTRETTO_BASEPOINT_POINT + recipient.spend_public;
    expected == payment.one_time_address
}

pub fn hash_to_scalar(point: &RistrettoPoint) -> Scalar {
    let mut hasher = Sha256::new();
    hasher.update(point.compress().as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Scalar::from_bytes_mod_order(bytes)
}