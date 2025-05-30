use std::{collections::HashMap, marker::PhantomData, ops::Mul};

use arith::Field;
use gkr_engine::Transcript;
use gkr_hashers::FiatShamirHasher;
use polynomials::EqPolynomial;
use crate::re_orion::{
    parameters::*,
    MerkleTree, MerkleTreeAPI, Encoder,
    utils::*,
    codeswitch::*,
};

pub struct OrionInstance<WitF, EvalF, ResF, H> 
where 
    WitF: Field, 
    EvalF: Field + Mul<WitF, Output = ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    H: FiatShamirHasher,
{
    srs: OrionSRS<WitF, EvalF, ResF>,
    commitments: HashMap<Vec<u8>, OrionCommitInstance<WitF, EvalF, ResF, H>>,
    scratch: OrionScratchPad<EvalF, ResF, H>,
}

impl<WitF, EvalF, ResF, H> OrionInstance<WitF, EvalF, ResF, H> 
where 
    WitF: Field, 
    EvalF: Field + Mul<WitF, Output = ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    H: FiatShamirHasher,
{
    pub fn new(wit_len: usize) -> Self {
        let srs = OrionSRS::<WitF, EvalF, ResF>::new(wit_len);
        let scratch = OrionScratchPad::<EvalF, ResF, H>::new(srs.msg_len, srs.encoder.code_len());
        Self {
            srs,
            commitments: HashMap::new(),
            scratch,
        }
    }

    pub fn commit(&mut self, wit: &[WitF]) -> Vec<u8> {
        let c = OrionCommitInstance::<WitF, EvalF, ResF, H>::new(wit, self.srs.msg_len, &self.srs.encoder);
        let res = c.commit();
        self.commitments.insert(res.clone(), c);
        res
    }

    pub fn open(
        &mut self, 
        commitment: &[u8],
        eval_point: &[EvalF],
        transcript: &mut impl Transcript,
    ) -> OrionOpening<WitF, ResF> {
        if let Some(instance) = self.commitments.get_mut(commitment) {
            let r0 = &mut self.scratch.r0;
            let r1 = &mut self.scratch.r1;
            let eq_head = &mut self.scratch.eq_head;
            let eq_tail = &mut self.scratch.eq_tail;
            EqPolynomial::<EvalF>::eq_eval_at(&eval_point[..COLUMN_LG], &EvalF::ONE, r0, eq_head, eq_tail);
            EqPolynomial::<EvalF>::eq_eval_at(&eval_point[COLUMN_LG..], &EvalF::ONE, r1, eq_head, eq_tail);
            instance.open(r0, r1, &self.srs.air, &mut self.scratch.instance_scratch, transcript)
        }
        else {
            panic!("the commitment does not exist")
        }
    }

    pub fn verify(
        &mut self,
        commitment: &[u8],
        eval_point: &[EvalF],
        opening: &OrionOpening<WitF, ResF>,
        transcript: &mut impl Transcript,
    ) -> bool {
        let r0 = &mut self.scratch.r0;
        let r1 = &mut self.scratch.r1;
        let eq_head = &mut self.scratch.eq_head;
        let eq_tail = &mut self.scratch.eq_tail;
        EqPolynomial::<EvalF>::eq_eval_at(&eval_point[..COLUMN_LG], &EvalF::ONE, r0, eq_head, eq_tail);
        EqPolynomial::<EvalF>::eq_eval_at(&eval_point[COLUMN_LG..], &EvalF::ONE, r1, eq_head, eq_tail);
        OrionCommitInstance::verify(commitment, r0, r1, opening, &self.srs.air, &mut self.scratch.instance_scratch, transcript)
    }
}

pub struct OrionSRS<WitF, EvalF, ResF> 
where
    WitF: Field,
    EvalF: Field,
    ResF: Field,
{
    msg_len: usize,
    encoder: Encoder<WitF::UnitField>,
    air: CodeSwitchAir<WitF, EvalF, ResF>,
}

impl<WitF, EvalF, ResF> OrionSRS<WitF, EvalF, ResF> 
where
    WitF: Field,
    EvalF: Field,
    ResF: Field,
{
    pub fn new(wit_len: usize) -> Self {
        let msg_len = ((wit_len + COLUMN_SIZE - 1) >> COLUMN_LG);
        let encoder = Encoder::<WitF::UnitField>::new(msg_len);
        let air = CodeSwitchAir::<WitF, EvalF, ResF>{
            graph_c: encoder.c.clone(),
            graph_d: encoder.d.clone(),
            msg_len: msg_len,
            code_len: encoder.code_len(),
            column_size: COLUMN_SIZE,
            idxs: (0..1500).collect(),
            _marker: PhantomData,
        };
        Self {
            msg_len,
            encoder,
            air,
        }
    }

    #[inline(always)]
    pub fn code_len(&self) -> usize {
        self.encoder.code_len()
    }

}

pub struct OrionCommitInstance<WitF, EvalF, ResF, H> 
where 
    WitF: Field, 
    EvalF: Field + Mul<WitF, Output = ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    H: FiatShamirHasher,
{
    wit: Vec<WitF>,
    width: usize,
    code: Vec<WitF>,
    code_len: usize,
    wit_t: Vec<WitF>,
    c1: Vec<WitF>,
    tree: MerkleTree<H>,
    _marker: PhantomData<(EvalF, ResF)>,
}

impl<WitF, EvalF, ResF, H> OrionCommitInstance<WitF, EvalF, ResF, H> 
where 
    WitF: Field, 
    EvalF: Field + Mul<WitF, Output = ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    H: FiatShamirHasher,
{
    fn new(wit: &[WitF], msg_len: usize, encoder: &Encoder<WitF::UnitField>) -> Self {
        let mut wit = wit.to_vec();
        let n = msg_len;
        wit.resize(COLUMN_SIZE * n, WitF::ZERO);
        let m = encoder.code_len();
        let mut code = vec![WitF::ZERO; COLUMN_SIZE * m];

        let mut wit_t = vec![WitF::ZERO; wit.len()];
        transpose(&wit, &mut wit_t, n, COLUMN_SIZE, 32);

        for (i, row) in wit.chunks_exact(n).enumerate() {
            encoder.encode(row, &mut code[i * m..], n, 0);
        }
        let mut c1 = vec![WitF::ZERO; code.len()];
        transpose(&code, &mut c1, m, COLUMN_SIZE, 32);

        let mut tree = MerkleTree::new(c1.len());
        tree.build(&c1);
        OrionCommitInstance {
            wit,
            width: n,
            code,
            code_len: m,
            wit_t,
            c1,
            tree,
            _marker: PhantomData,
        }
    }

    fn commit(&self) -> Vec<u8> {
        self.tree.commit()
    }

    fn open(
        &mut self, 
        r0: &[EvalF],
        r1: &[EvalF],
        // eval_point: &[EvalF], 
        air: &CodeSwitchAir<WitF, EvalF, ResF>,
        scratch: &mut OrionInstanceScratchPad<ResF, H>,
        transcript: &mut impl Transcript,
    ) -> OrionOpening<WitF, ResF> 
    {
        // let r0 = &mut scratch.r0;
        // let r1 = &mut scratch.r1;
        let mut gamma: Vec<EvalF> = Vec::with_capacity(r0.len());
        for i in 0..r0.len() {
            gamma.push(transcript.generate_field_element::<EvalF>());
        }
        let alpha = transcript.generate_field_element::<EvalF>();
        for i in 0..r0.len() {
            gamma[i] += alpha * r0[i];
        }

        let y_prime = &mut scratch.y_prime;
        let c_gamma = &mut scratch.c_gamma;
        let y_gamma = &mut scratch.y_gamma;
        
        // let eq_head = &mut scratch.eq_head;
        // let eq_tail = &mut scratch.eq_tail;
        // EqPolynomial::<EvalF>::eq_eval_at(eval_point[..COLUMN_SIZE], EvalF::ONE, r0, eq_head, eq_tail);
        // EqPolynomial::<EvalF>::eq_eval_at(eval_point[COLUMN_SIZE..], EvalF::ONE, r1, eq_head, eq_tail);

        for (j, row) in self.wit_t.chunks_exact(COLUMN_SIZE).enumerate() {
            for (i, &w) in row.iter().enumerate() {
                y_prime[j] += r0[i] * w;
                y_gamma[j] += gamma[i] * w;
            }
        }
        for (j, row) in self.c1.chunks_exact(COLUMN_SIZE).enumerate() {
            for (i, &c) in row.iter().enumerate() {
                c_gamma[j] += gamma[i] * c;
            }
        }
// println!("{:?}", &self.code[..self.code_len]);
// println!("{:?}", c_gamma);
        /*
        for j in 0..self.width {
            for i in 0..COLUMN_SIZE {
                y_prime[j] += r0[i] * self.wit[j];
                y_gamma[j] += gamma[i] * self.wit[j];
            }
        }
        for j in 0..self.code_len {
            for i in 0..COLUMN_SIZE {
                c_gamma[j] += gamma[i] * self.code[j];
            }
        } */

        let tree_gamma = &mut scratch.tree_gamma;
        tree_gamma.build(&c_gamma[..self.code_len]);
        
        let mut y = ResF::ZERO;
        for i in 0..self.width {
            y += y_prime[i] * r1[i];
        }

        let c_gamma_root = tree_gamma.commit();
        transcript.append_u8_slice(&c_gamma_root);
        transcript.append_field_element(&y);

        let mut idxs = Vec::with_capacity(CHALLENGE_SIZE);
        for i in 0..CHALLENGE_SIZE {
            idxs.push(usize::from_le_bytes(transcript.generate_u8_slice(8).try_into().unwrap()) % self.width);
        }

        let mut c_gamma_idx: Vec<ResF> = Vec::with_capacity(idxs.len());
        let mut c_gamma_proof: Vec<Vec<u8>> = Vec::with_capacity(idxs.len());
        let leaves = 1 << tree_gamma.height;
        for &idx in &idxs {
            c_gamma_idx.push(c_gamma[idx]);
            c_gamma_proof.push(tree_gamma.prove(leaves + idx, 1));
        }

        let witness = WitnessForPlonky3{
            y_gamma: &y_gamma,
            y1: &y_prime,
        };
        let public_values = PublicValuesForPlonky3{
            r1: r1,
            y: y,
            // TODO: idx only
            c_gamma: &c_gamma,
            // c_gamma_idx: &c_gamma_idx,
        };
        let proof_cs = prove::<WitF, EvalF, ResF>(air, &witness, &public_values);

        let mut root_idx_proof: Vec<Vec<u8>> = Vec::with_capacity(idxs.len());
        let column_leaf = 1 << (self.tree.height - COLUMN_LG);
        for &i in &idxs {
            root_idx_proof.push(self.tree.prove(column_leaf + i, 1));
        }

        let mut c2: Vec<Vec<WitF>> = Vec::with_capacity(idxs.len());
        for idx in idxs {
            c2.push(self.c1[idx * COLUMN_SIZE..(idx + 1) * COLUMN_SIZE].to_vec());
        }

        OrionOpening {
            proof_cs,

            c_gamma_idx,
            c_gamma_root,
            c_gamma_proof,

            y,

            root_idx_proof,

            c2,
        }
    }

    fn verify(
        commitment: &[u8],
        r0: &[EvalF],
        r1: &[EvalF],
        opening: &OrionOpening<WitF, ResF>,
        air: &CodeSwitchAir<WitF, EvalF, ResF>,
        scratch: &mut OrionInstanceScratchPad<ResF, H>,
        transcript: &mut impl Transcript,
    ) -> bool {
        let hasher = H::new();

        let c_gamma_root = &opening.c_gamma_root;
        let c_gamma = &opening.c_gamma_idx;
        let c_gamma_proof = &opening.c_gamma_proof;
        let mut leaf = vec![0u8; H::DIGEST_SIZE];
        for i in 0..c_gamma.len() {
            leaf.fill(0);
            c_gamma[i].to_bytes(&mut leaf[..ResF::SIZE]);
            hasher.hash_inplace(&mut leaf);
            if !MerkleTree::verify(c_gamma_root, &mut leaf, &c_gamma_proof[i], &hasher) {
                return false;
            }
        }

        let mut gamma: Vec<EvalF> = Vec::with_capacity(r0.len());
        for i in 0..r0.len() {
            gamma.push(transcript.generate_field_element::<EvalF>());
        }
        let alpha = transcript.generate_field_element::<EvalF>();
        for i in 0..r0.len() {
            gamma[i] += alpha * r0[i];
        }
        let c2 = &opening.c2;
        let root_idx_proof = &opening.root_idx_proof;
        let tree = &mut scratch.tree_gamma;
        for i in 0..c2.len() {
            tree.build(&c2[i]);
            if !MerkleTree::verify(commitment, &mut tree.commit(), &root_idx_proof[i], &hasher) {
                return false;
            }
            let mut sum = ResF::ZERO;
            for j in 0..COLUMN_SIZE {
                sum += gamma[j] * c2[i][j];
            }
            if sum != c_gamma[i] {
                return false;;
            }
        }

        // let mut idxs = Vec::with_capacity(CHALLENGE_SIZE);
        // for i in 0..CHALLENGE_SIZE {
        //     idxs.push(usize::from_le_bytes(transcript.generate_u8_slice(8).try_into().unwrap()) % self.width);
        // }

        let public_values = PublicValuesForPlonky3{
            r1: r1,
            y: opening.y,
            // TODO: idx only
            c_gamma: &c_gamma,
            // c_gamma_idx: &c_gamma_idx,
        };

        verify(air, &opening.proof_cs, &public_values)
    }
}

struct OrionInstanceScratchPad<F: Field, H: FiatShamirHasher> {
    y_prime: Vec<F>,
    c_gamma: Vec<F>,
    y_gamma: Vec<F>,
    tree_gamma: MerkleTree<H>,
}

impl<F: Field, H: FiatShamirHasher> OrionInstanceScratchPad<F, H> {
    fn new(n: usize, m: usize) -> Self {
        Self {
            y_prime: vec![F::ZERO; n],
            c_gamma: vec![F::ZERO; m],
            y_gamma: vec![F::ZERO; n],
            tree_gamma: MerkleTree::new(m.max(COLUMN_SIZE)),
        }
    }
}

pub struct OrionScratchPad<EvalF: Field, ResF: Field, H: FiatShamirHasher> {
    instance_scratch: OrionInstanceScratchPad<ResF, H>,
    r0: Vec<EvalF>,
    r1: Vec<EvalF>,
    eq_head: Vec<EvalF>,
    eq_tail: Vec<EvalF>,
}

impl<EvalF: Field, ResF: Field, H: FiatShamirHasher> OrionScratchPad<EvalF, ResF, H> {
    fn new(n: usize, m: usize) -> Self {
        Self {
            instance_scratch: OrionInstanceScratchPad::<ResF, H>::new(n, m),
            r0: vec![EvalF::ZERO; COLUMN_SIZE],
            r1: vec![EvalF::ZERO; n],
            eq_head: vec![EvalF::ZERO; n * COLUMN_SIZE],
            eq_tail: vec![EvalF::ZERO; n * COLUMN_SIZE],
        }
    }
}

pub struct OrionOpening<F: Field, ResF: Field> {
    proof_cs: Vec<u8>,

    c_gamma_idx: Vec<ResF>,
    c_gamma_root: Vec<u8>,
    c_gamma_proof: Vec<Vec<u8>>,

    y: ResF,

    root_idx_proof: Vec<Vec<u8>>,

    c2: Vec<Vec<F>>,
}