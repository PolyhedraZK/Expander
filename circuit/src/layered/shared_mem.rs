use super::circuit::{Circuit, CircuitLayer, StructureInfo};
use super::gates::{GateAdd, GateConst, GateMul, GateUni};
use gkr_field_config::GKRFieldConfig;
use mpi_config::shared_mem::SharedMemory;

impl<C: GKRFieldConfig> SharedMemory for CircuitLayer<C> {
    fn bytes_size(&self) -> usize {
        8 + 8
            + self.mul.bytes_size()
            + self.add.bytes_size()
            + self.const_.bytes_size()
            + self.uni.bytes_size()
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

    fn self_destroy(self) {
        self.mul.self_destroy();
        self.add.self_destroy();
        self.const_.self_destroy();
        self.uni.self_destroy();
    }
}

impl<C: GKRFieldConfig> SharedMemory for Circuit<C> {
    fn bytes_size(&self) -> usize {
        8 + self
            .layers
            .iter()
            .map(|layer| layer.bytes_size())
            .sum::<usize>()
    }

    fn to_memory(&self, ptr: &mut *mut u8) {
        let len = self.layers.len();
        len.to_memory(ptr);
        self.layers.iter().for_each(|layer| layer.to_memory(ptr));
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        let len = usize::from_memory(ptr);
        let layers = (0..len)
            .map(|_| CircuitLayer::<C>::from_memory(ptr))
            .collect();

        Circuit {
            layers,

            public_input: vec![],
            expected_num_output_zeros: 0,

            rnd_coefs_identified: false,
            rnd_coefs: vec![],
        }
    }

    fn self_destroy(self) {
        self.layers
            .into_iter()
            .for_each(|layer| layer.self_destroy());
    }
}
