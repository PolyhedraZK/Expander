use std::{collections::HashMap, marker::PhantomData, ops::Mul};

use arith::Field;
use codeswitch::{CodeSwitchAir, P3FieldConfig, P3Multiply, CHALLENGE_SIZE};
use encoder::Encoder;
use gkr_engine::Transcript;
use gkr_hashers::FiatShamirHasher;
use polynomials::EqPolynomial;
use crate::re_orion::{
    MerkleTree, MerkleTreeAPI,
    utils::*,
};

const COLUMN_LG: usize = 7;
const COLUMN_SIZE: usize = 1 << COLUMN_LG;
// const CHALLENGE_SIZE: usize = 1500;

// TODO: SIZE and PACK_SIZE to const
pub struct OrionInstance<WitF, CodeF, EvalF, ResF, H> 
where 
    WitF: Field, 
    CodeF: Field + From<WitF>, 
    EvalF: Field<UnitField = ResF::UnitField> + Mul<WitF, Output = ResF> + Mul<CodeF, Output = ResF> + P3Multiply<ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    ResF::UnitField: Mul<WitF, Output = CodeF> + Mul<CodeF, Output = CodeF> + P3FieldConfig,
    H: FiatShamirHasher,
{
    srs: OrionSRS<EvalF, ResF>,
    commitments: HashMap<Vec<u8>, OrionCommitInstance<WitF, CodeF, EvalF, ResF, H>>,
    scratch: OrionScratchPad<EvalF, ResF, H>,
}

impl<WitF, CodeF, EvalF, ResF, H> OrionInstance<WitF, CodeF, EvalF, ResF, H> 
where 
    WitF: Field, 
    CodeF: Field + From<WitF>, 
    EvalF: Field<UnitField = ResF::UnitField> + Mul<WitF, Output = ResF> + Mul<CodeF, Output = ResF> + P3Multiply<ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    ResF::UnitField: Mul<WitF, Output = CodeF> + Mul<CodeF, Output = CodeF> + P3FieldConfig,
    H: FiatShamirHasher,
{
    pub fn new(wit_len: usize) -> Self {
        let srs = OrionSRS::<EvalF, ResF>::new(wit_len);
        let scratch = OrionScratchPad::<EvalF, ResF, H>::new(srs.msg_len, srs.encoder.code_len);
        Self {
            srs,
            commitments: HashMap::new(),
            scratch,
        }
    }

    pub fn commit(&mut self, wit: &[WitF]) -> Vec<u8> {
        let c = OrionCommitInstance::<WitF, CodeF, EvalF, ResF, H>::new(wit, self.srs.msg_len, &self.srs.encoder);
        let res = c.commit();
        self.commitments.insert(res.clone(), c);
        res
    }

    pub fn open(
        &mut self, 
        commitment: &[u8],
        eval_point: &[EvalF],
        transcript: &mut impl Transcript,
    ) -> OrionOpening<CodeF, ResF> {
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
        opening: &OrionOpening<CodeF, ResF>,
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

pub struct OrionSRS<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF>,
    ResF: Field,
    ResF::UnitField: P3FieldConfig,
{
    msg_len: usize,
    encoder: Encoder<ResF::UnitField>,
    air: CodeSwitchAir<EvalF, ResF>,
}

impl<EvalF, ResF> OrionSRS<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF>,
    ResF: Field,
    ResF::UnitField: P3FieldConfig,
{
    pub fn new(wit_len: usize) -> Self {
        let msg_len = ((wit_len + COLUMN_SIZE - 1) >> COLUMN_LG);
        let encoder = Encoder::<ResF::UnitField>::new(msg_len);
        let air = CodeSwitchAir::<EvalF, ResF>::new(
            &encoder,
            msg_len,
            COLUMN_SIZE,
            (0..1500).map(|x| x % encoder.code_len).collect(),
        );
        Self {
            msg_len,
            encoder,
            air,
        }
    }

    #[inline(always)]
    pub fn code_len(&self) -> usize {
        self.encoder.code_len
    }

}

pub struct OrionCommitInstance<WitF, CodeF, EvalF, ResF, H> 
where 
    WitF: Field, 
    CodeF: Field + From<WitF>, 
    EvalF: Field<UnitField = ResF::UnitField> + Mul<WitF, Output = ResF> + Mul<CodeF, Output = ResF> + P3Multiply<ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    ResF::UnitField: Mul<WitF, Output = CodeF> + Mul<CodeF, Output = CodeF> + P3FieldConfig,
    H: FiatShamirHasher,
{
    wit: Vec<WitF>,
    width: usize,
    code: Vec<CodeF>,
    code_len: usize,
    wit_t: Vec<WitF>,
    c1: Vec<CodeF>,
    tree: MerkleTree<H>,
    _marker: PhantomData<(EvalF, ResF)>,
}

impl<WitF, CodeF, EvalF, ResF, H> OrionCommitInstance<WitF, CodeF, EvalF, ResF, H> 
where 
    WitF: Field, 
    CodeF: Field + From<WitF>, 
    EvalF: Field<UnitField = ResF::UnitField> + Mul<WitF, Output = ResF> + Mul<CodeF, Output = ResF> + P3Multiply<ResF>, 
    ResF: Field + Mul<EvalF, Output = ResF>, 
    ResF::UnitField: Mul<WitF, Output = CodeF> + Mul<CodeF, Output = CodeF> + P3FieldConfig,
    
    H: FiatShamirHasher,
{
    fn new(wit: &[WitF], msg_len: usize, encoder: &Encoder<ResF::UnitField>) -> Self {
        let mut wit = wit.to_vec();
        let n = msg_len;
        wit.resize(COLUMN_SIZE * n, WitF::ZERO);
        let m = encoder.code_len;
        let mut code = vec![CodeF::ZERO; COLUMN_SIZE * m];

        let mut wit_t = vec![WitF::ZERO; wit.len()];
        transpose(&wit, &mut wit_t, n, COLUMN_SIZE, 32);

        for (i, row) in wit.chunks_exact(n).enumerate() {
            encoder.encode(row, &mut code[i * m..], n);
        }
        let mut c1 = vec![CodeF::ZERO; code.len()];
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
        air: &CodeSwitchAir<EvalF, ResF>,
        scratch: &mut OrionInstanceScratchPad<ResF, H>,
        transcript: &mut impl Transcript,
    ) -> OrionOpening<CodeF, ResF> 
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
        
        y_prime.fill(ResF::ZERO);
        c_gamma.fill(ResF::ZERO);
        y_gamma.fill(ResF::ZERO);

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

        let tree_gamma = &mut scratch.tree_gamma;
        tree_gamma.build(&c_gamma[..self.code_len]);
        
        let mut y = ResF::ZERO;
        for i in 0..self.width {
            y += y_prime[i] * r1[i];
        }

let mut tmp = ResF::ZERO;
for i in 0..self.width {
    for j in 0..COLUMN_SIZE {
        tmp += r0[j] * r1[i] * self.wit[j * self.width + i];
    }
}
println!("test y {:?} ? {:?}", tmp, y);

        let c_gamma_root = tree_gamma.commit();
        transcript.append_u8_slice(&c_gamma_root);
        transcript.append_field_element(&y);

        let idxs = &air.idxs;
        // let mut idxs = Vec::with_capacity(CHALLENGE_SIZE);
        // for i in 0..CHALLENGE_SIZE {
        //     idxs.push(usize::from_le_bytes(transcript.generate_u8_slice(8).try_into().unwrap()) % self.width);
        // }

        let mut c_gamma_idx: Vec<ResF> = Vec::with_capacity(idxs.len());
        let mut c_gamma_proof: Vec<Vec<u8>> = Vec::with_capacity(idxs.len());
        let leaves = 1 << tree_gamma.height;
        for &idx in idxs.iter() {
            c_gamma_idx.push(c_gamma[idx]);
            c_gamma_proof.push(tree_gamma.prove(leaves + idx, 1));
        }

        let proof_cs = air.prove(&y_gamma, &y_prime, &c_gamma, &c_gamma_idx, &r1, &y, );

        let mut root_idx_proof: Vec<Vec<u8>> = Vec::with_capacity(idxs.len());
        let column_leaf = 1 << (self.tree.height - COLUMN_LG);
        for &i in idxs.iter() {
            root_idx_proof.push(self.tree.prove(column_leaf + i, 1));
        }

        let mut c2: Vec<Vec<CodeF>> = Vec::with_capacity(idxs.len());
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
        air: &CodeSwitchAir<EvalF, ResF>,
        scratch: &mut OrionInstanceScratchPad<ResF, H>,
        transcript: &mut impl Transcript,
    ) -> bool {
        let hasher = H::new();

        let c_gamma_root = &opening.c_gamma_root;
        let c_gamma = &opening.c_gamma_idx;
        let c_gamma_proof = &opening.c_gamma_proof;
        let mut leaf = vec![0u8; H::DIGEST_SIZE];
        let mut f = vec![0u8; ResF::SIZE];
        for i in 0..c_gamma.len() {
            leaf.fill(0);
            c_gamma[i].to_bytes(&mut f);
            hasher.hash(&mut leaf, &f);
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

        air.verify(&opening.proof_cs, &c_gamma,&r1, &opening.y)
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

pub struct OrionOpening<CodeF: Field, ResF: Field> {
    proof_cs: Vec<u8>,

    c_gamma_idx: Vec<ResF>,
    c_gamma_root: Vec<u8>,
    c_gamma_proof: Vec<Vec<u8>>,

    y: ResF,

    root_idx_proof: Vec<Vec<u8>>,

    c2: Vec<Vec<CodeF>>,
}