use std::ptr::copy_nonoverlapping;

pub trait SharedMemory {
    fn bytes_size(&self) -> usize;

    fn to_memory(&self, ptr: &mut *mut u8);

    fn from_memory(ptr: &mut *mut u8) -> Self;

    fn discard_control_of_shared_mem(self);
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

    fn discard_control_of_shared_mem(self) {}
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

    fn discard_control_of_shared_mem(self) {}
}

impl<T: Copy> SharedMemory for Vec<T> {
    fn bytes_size(&self) -> usize {
        self.len().bytes_size()
         + self.len() * std::mem::size_of::<T>()
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

    fn discard_control_of_shared_mem(self) {
        self.leak();
    }
}

impl<T1: SharedMemory, T2: SharedMemory> SharedMemory for (T1, T2) {
    fn bytes_size(&self) -> usize {
        self.0.bytes_size() + self.1.bytes_size()
    }

    fn to_memory(&self, ptr: &mut *mut u8) {
        self.0.to_memory(ptr);
        self.1.to_memory(ptr);
    }

    fn from_memory(ptr: &mut *mut u8) -> Self {
        let t1 = T1::from_memory(ptr);
        let t2 = T2::from_memory(ptr);
        (t1, t2)
    }

    fn discard_control_of_shared_mem(self) {
        self.0.discard_control_of_shared_mem();
        self.1.discard_control_of_shared_mem();
    }
}
