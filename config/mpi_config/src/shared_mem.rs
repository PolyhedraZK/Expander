use std::ptr::copy_nonoverlapping;

pub trait SharedMemory {
    fn bytes_size(&self) -> usize;

    fn to_memory(&self, ptr: &mut *mut u8);

    fn from_memory(ptr: &mut *mut u8) -> Self;

    fn self_destroy(self);
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

    fn self_destroy(self) {}
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

    fn self_destroy(self) {}
}

impl<T: Copy> SharedMemory for Vec<T> {
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

    fn self_destroy(self) {
        self.leak();
    }
}
