mod block;
mod blockchain;
mod stealth;
mod confidential;
mod ring_signature;
mod zkproof;

use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use rand::rngs::OsRng;

fn main() {
    let mut rng = OsRng;

    println!("🚀 Privacy Chain — Démarrage\n");

    // Blockchain de base
    let mut chain = blockchain::Blockchain::new();
    chain.add_block("Bloc 1".to_string());
    chain.add_block("Bloc 2".to_string());
    println!("✅ Blockchain valide : {}", chain.is_valid());

    // Stealth addresses
    let bob = stealth::RecipientKeypair::generate();
    let payment = stealth::generate_stealth_address(&bob);
    println!("✅ Stealth : Bob trouve son paiement : {}",
        stealth::scan_for_payment(&bob, &payment));

    // Confidential transactions
    let input = confidential::PedersenCommitment::new(100);
    let ct = confidential::create_confidential_tx(100, input.blinding, 70)
        .expect("CT échouée");
    println!("✅ Confidential TX : balance ok : {}",
        ct.output_commitment + ct.change_commitment == ct.input_commitment);

    // Ring signatures
    let keys: Vec<Scalar> = (0..4).map(|_| Scalar::random(&mut rng)).collect();
    let ring: Vec<_> = keys.iter().map(|k| k * RISTRETTO_BASEPOINT_POINT).collect();
    let sig = ring_signature::sign(b"test", &ring, keys[0], 0);
    println!("✅ Ring Signature valide : {}",
        ring_signature::verify(b"test", &ring, &sig));

    // zk-SNARKs
    println!("\n⚙️  Setup zk-SNARKs (quelques secondes)...");
    let zk = zkproof::setup();
    let proof = zkproof::prove(&zk, 100, 70, 30).expect("Preuve échouée");
    println!("✅ zk-SNARK valide : {}", zkproof::verify_proof(&zk, &proof));

    println!("\n🎉 Tout fonctionne !");
}