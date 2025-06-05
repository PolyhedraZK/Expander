use arith::Field;
use p3_field::Field as P3Field;
use rand::{Rng, RngCore};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serdes::{ExpSerde, SerdeResult};
use std::any::type_name;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::ops::Mul;
use std::path::Path;

pub const TARGET_DISTANCE: f64 = 0.07;
pub const DISTANCE_THRESHOLD: usize = ((1.0 / TARGET_DISTANCE) - 1.0) as usize;
const RS_RATE: u32 = 2;
const ALPHA: f64 = 0.238;
const BETA: f64 = 0.1205;
const R: f64 = 1.72;
const C_SIZE: usize = 10;
const D_SIZE: usize = 20;

#[derive(Debug, Clone)]
pub struct BiGraph<F> {
    pub l_degree: usize,
    pub l_size: usize,
    pub r_size: usize,
    pub edge: Vec<Vec<(usize, F)>>, // l: [..l_size], r: [l_size..]
}

impl<F: Field> BiGraph<F> {
    fn generate(l_size: usize, r_size: usize, degree: usize) -> Self {
        let mut edge: Vec<Vec<(usize, F)>> = Vec::with_capacity(l_size + r_size);
        for i in 0..l_size {
            edge.push(Vec::with_capacity(degree));
        }
        for i in 0..r_size {
            edge.push(Vec::new());
        }
        let mut rng = StdRng::from_seed([226; 32]);
        for i in 0..l_size {
            for j in 0..degree {
                let to = rng.next_u32() as usize % r_size;
                let mut bytes = vec![0u8; F::SIZE];
                rng.fill_bytes(&mut bytes);
                let w = F::from_uniform_bytes(&bytes);
                edge[i].push((to, w));
                edge[to + l_size].push((i, w));
            }
        }

        Self {
            l_degree: degree,
            l_size,
            r_size,
            edge,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Encoder<F> {
    pub code_len: usize,
    pub c: Vec<BiGraph<F>>,
    pub d: Vec<BiGraph<F>>,
}

impl<F: Field> Encoder<F> {
    pub fn new(n: usize) -> Self {
        let mut logn = 0;
        while (1 << logn) < n {
            logn += 1;
        }
        let filename = format!("{}_{}", type_name::<F>(), logn);
        if Path::new(&filename).exists() {
            let filebytes = fs::read(filename).unwrap();
            // TODO: serde
            // Self::deserialize_from(filebytes)
            Encoder {
                code_len: 0, 
                c: vec![], 
                d: vec![],
            }
        }
        else {
            let mut ret = Encoder {
                code_len: 0, 
                c: vec![], 
                d: vec![],
            };
            ret.generate(1 << logn, 0);
println!("bi-graph generated");
            // TODO: serde
            // let file = std::fs::File::create(filename).unwrap();
            // let writer = std::io::BufWriter::new(file);
            // ret.serialize_into(writer).unwrap();
            ret
        }
    }

    fn generate(&mut self, n: usize, dep: usize) -> usize {
        if n <= DISTANCE_THRESHOLD {
            self.d = Vec::with_capacity(dep);
            self.code_len = n;
            return self.code_len;
        }
        let r_size = (n as f64 * ALPHA).round() as usize;
        self.c.push(BiGraph::<F>::generate(n, r_size, C_SIZE));
        let l_size = self.generate(r_size, dep + 1);
        let r_size = ((n as f64 * (R - 1.0)).round() as usize - l_size);
        self.d.push(BiGraph::<F>::generate(l_size, r_size, D_SIZE));
        if dep == 0 {
            self.d.reverse();
        }
        self.code_len = n + l_size + r_size;
        return self.code_len;
    }

    pub fn encode<MsgF, CodeF>(&self, src: &[MsgF], dst: &mut [CodeF], n: usize) -> usize 
    where
        MsgF: Field + Sized,
        F: Mul<MsgF, Output = CodeF> + Mul<CodeF, Output = CodeF>,
        CodeF: Field + From<MsgF> + Sized,
    {
        if MsgF::NAME == CodeF::NAME {
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr() as *const CodeF, dst.as_mut_ptr(), n); }
            // dst[..n].copy_from_slice(&src[..n]);
            self.encode_inplace(dst, n, 0)
        }
        else {
            let dep = 0;
            for (s, d) in src.iter().zip(dst.iter_mut()).take(n) {
                *d = CodeF::from(*s)
            }

            let r = self.c[dep].r_size;
            let nxt_dst = &mut dst[n..];
            nxt_dst[..r].fill(CodeF::ZERO);
            for (i, u) in src.iter().enumerate() {
                for (v, w) in &self.c[dep].edge[i] {
                    nxt_dst[*v] += *w * *u;
                }
            }
            
            let l = self.encode_inplace(nxt_dst, r, dep + 1);

            let r = self.d[dep].r_size;
            let (nxt_src, nxt_dst) = dst.split_at_mut(n + l);

            nxt_dst[..r].fill(CodeF::ZERO);
            for (i, u) in nxt_src[n..].iter().enumerate() {
                for (v, w) in &self.d[dep].edge[i] {
                    nxt_dst[*v] += *w * *u;
                }
            }
            
            n + l + r
        }
// println!("start encode {} {} {}", src.len(), n, dst.len());
    }

    pub fn encode_inplace<CodeF: Field>(&self, dst: &mut [CodeF], n: usize, dep: usize) -> usize 
    where
        F: Mul<CodeF, Output = CodeF>,
    {
        if n <= DISTANCE_THRESHOLD {
            return n
        }
        // let r = (n as f64 * ALPHA).round() as usize;
        let r = self.c[dep].r_size;
        let (src, nxt_dst) = dst.split_at_mut(n);

        // TODO: unsafe?
        nxt_dst[..r].fill(CodeF::ZERO);
// println!("c {} {}", dep, self.c[dep].edge.len());
        for (i, u) in src.iter().enumerate() {
            for (v, w) in &self.c[dep].edge[i] {
                nxt_dst[*v] += *w * *u;
            }
        }
        
        let l = self.encode_inplace(nxt_dst, r, dep + 1);

        let r = self.d[dep].r_size;
        let (nxt_src, nxt_dst) = dst.split_at_mut(n + l);

        nxt_dst[..r].fill(CodeF::ZERO);
        for (i, u) in nxt_src[n..].iter().enumerate() {
            for (v, w) in &self.d[dep].edge[i] {
                nxt_dst[*v] += *w * *u;
            }
        }
        
        n + l + r
    }
}

fn trans_bigraph_vec<S: Field, T: P3Field>(src: &[BiGraph<S>]) -> Vec<BiGraph<T>> {
    let mut g: Vec<BiGraph<T>> = Vec::with_capacity(src.len());
    for gi in src.iter() {
        let mut edges: Vec<Vec<(usize, T)>> = Vec::with_capacity(gi.edge.len());
        for e in gi.edge.iter() {
            let mut edge: Vec<(usize, T)> = vec![(0, T::ZERO); e.len()];
            unsafe { std::ptr::copy_nonoverlapping(e.as_ptr() as *const (usize, T), edge.as_mut_ptr(), e.len()); }
            edges.push(edge);
        }
        g.push(BiGraph::<T> {
            l_degree: gi.l_degree,
            l_size: gi.l_size,
            r_size: gi.r_size,
            edge: edges,
        });
    }
    g
}

pub trait NewFrom<S: Field> {
    fn new_from(encoder: &Encoder<S>) -> Self;
}

impl<S: Field, T: P3Field> NewFrom<S> for Encoder<T> {
    fn new_from(encoder: &Encoder<S>) -> Encoder<T> {
        Encoder::<T> {
           code_len: encoder.code_len,
           c: trans_bigraph_vec(&encoder.c),
           d: trans_bigraph_vec(&encoder.d),
        }
    }
}