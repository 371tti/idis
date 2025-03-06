use std::alloc::{self, Layout, alloc};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;

pub mod map;

struct Object {
    ptr: ,
}

struct Cash {
    map: HashMap<u128, Object>,
    
    buf:RawCash,
}

struct RawCash {
    ptr: NonNull<u8>,
    size: usize,
    _marker: std::marker::PhantomData<u8>,
}

impl RawCash {
    fn new(size: usize) -> Self {
        unsafe {
            let u8_cap= std::mem::size_of::<u8>();
            let u8_align = std::mem::align_of::<u8>();
            let new_mem_layout = Layout::from_size_align(u8_cap, u8_align).expect("Failed to create layout");
            alloc(new_mem_layout);
        }
        Self {
            // set none pointer
            ptr: NonNull::dangling(),
            size: size,
            _marker: std::marker::PhantomData,
        }
    }

    fn alloc(&mut self) {

    }
}