use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;

pub struct RingSignature {
    pub key_image: RistrettoPoint,
    pub c: Vec<Scalar>,
    pub r: Vec<Scalar>,
}

fn hash_points(points: &[RistrettoPoint], message: &[u8]) -> Scalar {
    let mut hasher = Sha256::new();
    for p in points { hasher.update(p.compress().as_bytes()); }
    hasher.update(message);
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hasher.finalize());
    Scalar::from_bytes_mod_order(bytes)
}

fn hash_to_point(point: &RistrettoPoint) -> RistrettoPoint {
    let mut hasher = Sha256::new();
    hasher.update(point.compress().as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hasher.finalize());
    Scalar::from_bytes_mod_order(bytes) * RISTRETTO_BASEPOINT_POINT
}

pub fn sign(
    message: &[u8], ring: &[RistrettoPoint],
    signer_key: Scalar, signer_idx: usize,
) -> RingSignature {
    let mut rng = OsRng;
    let n = ring.len();
    let key_image = signer_key * hash_to_point(&ring[signer_idx]);
    let mut c = vec![Scalar::ZERO; n];
    let mut r = vec![Scalar::ZERO; n];
    let alpha  = Scalar::random(&mut rng);
    let next   = (signer_idx + 1) % n;
    c[next]    = hash_points(&[alpha * RISTRETTO_BASEPOINT_POINT,
                                alpha * hash_to_point(&ring[signer_idx])], message);
    let mut i  = next;
    while i != signer_idx {
        r[i]       = Scalar::random(&mut rng);
        let l_i    = r[i] * RISTRETTO_BASEPOINT_POINT + c[i] * ring[i];
        let r_i    = r[i] * hash_to_point(&ring[i]) + c[i] * key_image;
        let next_i = (i + 1) % n;
        c[next_i]  = hash_points(&[l_i, r_i], message);
        i          = next_i;
    }
    r[signer_idx] = alpha - c[signer_idx] * signer_key;
    RingSignature { key_image, c, r }
}

pub fn verify(message: &[u8], ring: &[RistrettoPoint], sig: &RingSignature) -> bool {
    let n = ring.len();
    let mut c_check = sig.c[0];
    for i in 0..n {
        let l_i = sig.r[i] * RISTRETTO_BASEPOINT_POINT + c_check * ring[i];
        let r_i = sig.r[i] * hash_to_point(&ring[i]) + c_check * sig.key_image;
        c_check = hash_points(&[l_i, r_i], message);
        if i < n - 1 { c_check = sig.c[i + 1]; }
    }
    c_check == sig.c[0]
}

pub fn is_double_spend(used_images: &[RistrettoPoint], sig: &RingSignature) -> bool {
    used_images.contains(&sig.key_image)
}