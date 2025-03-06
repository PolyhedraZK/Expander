use std::ptr::copy_nonoverlapping;

use super::circuit::{Circuit, CircuitLayer, StructureInfo};
use super::gates::{GateAdd, GateConst, GateMul, GateUni};
use gkr_field_config::GKRFieldConfig;

pub trait SharedMemory {
    fn bytes_size(&self) -> usize;

    fn to_memory(&self, ptr: &mut *mut u8);

    fn from_memory(ptr: &mut *mut u8) -> Self;
}

impl SharedMemory for usize {

    fn bytes_size(&self) -> usize {
        8
    }

    fn to_memory(&self, ptr: &mut *mut u8) {
        unsafe {
            (*ptr as *mut usize).write(*self);
            *ptr = ptr.add(8);
        }
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        unsafe {
            let ret = (*ptr as *mut usize).read();
            *ptr = ptr.add(8);
            ret
        }
    }
}

impl SharedMemory for u8 {

    fn bytes_size(&self) -> usize {
        1
    }

    fn to_memory(&self, ptr: &mut *mut u8) {
        unsafe {
            ptr.write(*self);
            *ptr = ptr.add(1);
        }
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        unsafe {
            let ret = ptr.read();
            *ptr = ptr.add(1);
            ret
        }
    }
}

impl<T: Sized> SharedMemory for Vec<T> {

    fn bytes_size(&self) -> usize {
        8 + self.len() * std::mem::size_of::<T>()
    }

    fn to_memory(&self, ptr: &mut *mut u8) {
        unsafe {
            let len = self.len();
            len.to_memory(ptr);

            copy_nonoverlapping(self.as_ptr(), *ptr as *mut T, len);
            *ptr = ptr.add(len * std::mem::size_of::<T>());
        }
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        unsafe {
            let len = usize::from_memory(ptr);
            let ret = Vec::<T>::from_raw_parts(*ptr as *mut T, len, len);
            *ptr = ptr.add(len * std::mem::size_of::<T>());
            ret
        }
    }
}

impl<C: GKRFieldConfig> SharedMemory for CircuitLayer<C> {
    fn bytes_size(&self) -> usize {
        8 + 8 + self.mul.bytes_size() + self.add.bytes_size() + self.const_.bytes_size() + self.uni.bytes_size()
    }
    
    fn to_memory(&self, ptr: &mut *mut u8) {
        self.input_var_num.to_memory(ptr);
        self.output_var_num.to_memory(ptr);
        self.mul.to_memory(ptr);
        self.add.to_memory(ptr);
        self.const_.to_memory(ptr);
        self.uni.to_memory(ptr);
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        let input_var_num = usize::from_memory(ptr);
        let output_var_num = usize::from_memory(ptr);
        let mul = Vec::<GateMul<C>>::from_memory(ptr);
        let add = Vec::<GateAdd<C>>::from_memory(ptr);
        let const_ = Vec::<GateConst<C>>::from_memory(ptr);
        let uni = Vec::<GateUni<C>>::from_memory(ptr);

        CircuitLayer {
            input_var_num,
            output_var_num,

            input_vals: vec![],
            output_vals: vec![],

            mul,
            add,
            const_,
            uni,

            structure_info: StructureInfo::default(),
        }
    }
}

impl<C: GKRFieldConfig> SharedMemory for Circuit<C> {

    fn bytes_size(&self) -> usize {
        8 + self.layers.iter().map(|layer| layer.bytes_size()).sum::<usize>()
    }

    fn to_memory(&self, ptr: &mut *mut u8) {
        let len = self.layers.len();
        len.to_memory(ptr);
        self.layers.iter().for_each(|layer| layer.to_memory(ptr));
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        let len = usize::from_memory(ptr);
        let layers = (0..len).map(|_| CircuitLayer::<C>::from_memory(ptr)).collect();

        Circuit {
            layers,

            public_input: vec![],
            expected_num_output_zeros: 0,

            rnd_coefs_identified: false,
            rnd_coefs: vec![],
        }
    }
}