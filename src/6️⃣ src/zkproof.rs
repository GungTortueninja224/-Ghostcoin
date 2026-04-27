use bellman::{Circuit, ConstraintSystem, SynthesisError, groth16};
use bls12_381::Bls12;
use rand::rngs::OsRng;

#[derive(Clone)]
pub struct BalanceCircuit {
    pub input_amount:  Option<u64>,
    pub output_amount: Option<u64>,
    pub change_amount: Option<u64>,
}

impl Circuit<bls12_381::Scalar> for BalanceCircuit {
    fn synthesize<CS: ConstraintSystem<bls12_381::Scalar>>(
        self, cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        let input = cs.alloc(|| "input", || {
            self.input_amount.map(bls12_381::Scalar::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let output = cs.alloc(|| "output", || {
            self.output_amount.map(bls12_381::Scalar::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let change = cs.alloc(|| "change", || {
            self.change_amount.map(bls12_381::Scalar::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        cs.enforce(|| "balance",
            |lc| lc + output + change,
            |lc| lc + CS::one(),
            |lc| lc + input,
        );
        Ok(())
    }
}

pub struct ZkKeys {
    pub proving_key:   groth16::Parameters<Bls12>,
    pub verifying_key: groth16::PreparedVerifyingKey<Bls12>,
}

pub fn setup() -> ZkKeys {
    let mut rng = OsRng;
    let params = groth16::generate_random_parameters::<Bls12, _, _>(
        BalanceCircuit { input_amount: None, output_amount: None, change_amount: None },
        &mut rng,
    ).expect("Setup échoué");
    ZkKeys {
        verifying_key: groth16::prepare_verifying_key(&params.vk),
        proving_key:   params,
    }
}

pub fn prove(keys: &ZkKeys, input: u64, output: u64, change: u64) -> Option<groth16::Proof<Bls12>> {
    if input != output + change { return None; }
    let mut rng = OsRng;
    groth16::create_random_proof(
        BalanceCircuit {
            input_amount:  Some(input),
            output_amount: Some(output),
            change_amount: Some(change),
        },
        &keys.proving_key, &mut rng,
    ).ok()
}

pub fn verify_proof(keys: &ZkKeys, proof: &groth16::Proof<Bls12>) -> bool {
    groth16::verify_proof(&keys.verifying_key, proof, &[]).is_ok()
}