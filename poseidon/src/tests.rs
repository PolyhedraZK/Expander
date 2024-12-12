use arith::Field;
use mersenne31::M31x16;

use crate::{PoseidonM31Params, PoseidonM31State};

#[test]
fn test_poseidon_m31() {
    let mut rng = rand::thread_rng();
    let param = PoseidonM31Params::new(&mut rand::thread_rng());
    let mut state = PoseidonM31State {
        state: M31x16::random_unsafe(&mut rng),
    };

    param.permute(&mut state)
}
