use std::alloc::{self, Layout, alloc};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;

pub mod map;

struct Object {
}

struct Cash {
    map: HashMap<u128, Object>,
    buf:RawCash,
}

struct RawCash {
    ptr: NonNull<u8>,
    block_size: usize,
    block_num: usize,
    _marker: std::marker::PhantomData<u8>,
}

impl RawCash {
    /// 新しくキャッシュ領域を作成する
    /// 
    /// `capacity` - 1ブロックあたりのサイズ（キャッシュライン 64B の倍数を推奨）
    /// `block_num` - 確保するブロックの数
    /// 
    /// return: RawCash
    fn new(capacity: usize, block_num: usize) -> Self {
        unsafe {
            // `capacity` は 64 の倍数であるべき（調整）
            let aligned_capacity = (capacity + 63) & !63;
            
            // メモリ確保する総サイズ
            let total_size = aligned_capacity * block_num;

            // アライメントを `capacity` に設定
            let layout = Layout::from_size_align(total_size, aligned_capacity)
                .expect("Failed to create layout");

            // メモリ確保
            let ptr = alloc(layout);
            let ptr = NonNull::new(ptr).unwrap_or_else(|| std::alloc::handle_alloc_error(layout));

            Self {
                ptr,
                block_size: aligned_capacity,
                block_num,
                _marker: std::marker::PhantomData,
            }
        }
    }
}

struct PowerShiftMapAlloc {
    free_tree: Vec<Vec<u64>>,
    map_tree: Vec<Vec<u64>>,
}

struct PowerMap {
    pub free_map: Vec<Vec<u64>>,
}

impl PowerMap {
    fn new(block_num: usize) -> Self {
        let layer_num = block_num.log64_ceil();
        let mut map = Vec::with_capacity(layer_num);
        let mut this_layer_num = block_num;

        for _ in 0..layer_num {
            let rem = this_layer_num % 64;
            this_layer_num = this_layer_num / 64 + (rem > 0) as usize;
            let mut layer = Vec::with_capacity(this_layer_num);
            for _ in 0..this_layer_num {
                layer.push(0);
            }
            if rem > 0 {
                *layer.last_mut().unwrap() = !0u64 << rem;
            }
            map.push(layer);
        }
        map.reverse();

        Self { free_map: map }
    }

    /// 空きブロックを検索する（見つからなければ `None`）
    fn search_free_block(&self) -> Option<usize> {
        let mut index: usize = 0;
        for map in &self.free_map {
            let c = (map[index] + 1).trailing_zeros() as usize;
            if c == 64 {
                return None;
            }
            index = (index << 6) | c;
        }
        Some(index)
    }

    fn get_map(&self) -> &Vec<u64> {
        &self.free_map.last().unwrap()
    }

    fn alloc(&mut self) {

    }
}

impl PowerShiftMapAlloc {
    fn new(block_num: usize) -> Self {
        let layer_num = block_num.log64_ceil();
        let mut free_tree: Vec<Vec<u64>> = Vec::with_capacity(layer_num);
        let mut this_layer_num = block_num;
        for _ in 0..layer_num {
            let rem = this_layer_num % 64;
            this_layer_num = this_layer_num / 64 + (rem > 0) as usize;
            let mut layer = Vec::with_capacity(this_layer_num);
            for _ in 0..this_layer_num {
                layer.push(0);
            }
            if rem > 0 {
                *layer.last_mut().unwrap() = !0u64 >> rem
            }
            free_tree.push(layer);
        }

        Self {
            free_tree,
            map_tree: Vec::with_capacity(layer_num),
        }
    }
}

trait Log64Ext {
    fn log64_ceil(self) -> usize;
}

impl Log64Ext for usize {
    fn log64_ceil(self) -> usize {
        if self <= 1 {
            return 1; // 1のときも切り上げて1を返す
        }
        let log = self.ilog(64);
        log as usize + ((self > 64usize.pow(log)) as usize)
    }
}