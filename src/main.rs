/// 領域アロケーター
/// ページ単位で管理
/// ページあたりのブロック数は274,877,906,944 = 2^32 * 64
pub struct FreeMap {
    /// bit map
    /// page > (hi_layer - lo_layer)
    /// 連続領域でtree map を構築
    pub pow_map: Vec<Vec<u64>>,
    /// ブロック数
    pub size: u64,
    /// レイヤー数
    pub layer_num: usize,
}

impl FreeMap {
    pub const PAGE_CAPACITY: usize = 0x03FF_FFFF as usize;
    pub const PAGE_SHIFT: usize = 26;
    /// FreeMap を初期化する
    pub fn new(r_size: u64) -> Self {
        let mut pow_map: Vec<Vec<u64>> = Vec::new();
        // レイヤー数を計算
        let layer_num = r_size.log64_ceil();
        let mut size = r_size;
    
        let mut now_page = 0;
        pow_map.push(Vec::new());
        // レイヤーごとに処理
        for _ in 0..layer_num {
            // レイヤーのサイズとあまりを計算
            let layer_mode = size & 0x3F;
            let layer_size = (size + 0x3E) >> 6;
            for i in 0..layer_size {
                if pow_map[now_page].len() == Self::PAGE_CAPACITY {
                    now_page += 1;
                    pow_map.push(Vec::new());
                } else {
                    if i == layer_size - 1 {
                        pow_map[now_page].push(!0u64 << layer_mode);
                    } else {
                        pow_map[now_page].push(0);
                    }
                }
            }
            
            size = layer_size;
        }
        println!("layer_num: {}", layer_num);
        FreeMap { pow_map, size: r_size, layer_num }
    }

    /// ある深さのindexの要素を取得する
    /// indexはu64
    /// 内部でu32のrangeで分割
    /// 
    /// # Arguments
    /// * `deep` - ページの深さ
    /// * `index` - ブロックのインデックス
    #[inline(always)]
    pub fn c(&mut self, deep: usize, index: u64) -> &mut u64 {
        let offset = self.precomputed_offset(deep);
        let raw_index = offset + index;
        let page: usize = (raw_index >> Self::PAGE_SHIFT) as usize;
        let index: usize = raw_index as usize & Self::PAGE_CAPACITY;
        &mut self.pow_map[page][index]
    }

    /// ある深さのlayerが始まるindexを取得する
    #[inline(always)]
    fn precomputed_offset(&self, deep: usize) -> u64 {
        let mut size = self.size;
        let mut offset: u64 = 0;
        for _ in 0..deep {
            size = (size + 0x3E) >> 6;
            offset += size
        }
        offset
    }

    #[inline(always)]
    pub fn search_free_block(&mut self) -> Option<u64> {
        let mut block_index: u64 = 0;
        for i in (0..self.layer_num).rev() {
            let c = (*self.c(i, block_index)).trailing_ones() as u64;
            if c == 64 {
                return None;
            }
            block_index = (block_index << 6) | c;
        }
        Some(block_index)
    }

    #[inline(always)]
    pub fn search_free_blocks(&mut self, r_block_num: u64) -> Option<u64> {
        let mut need_block_num_as_layer = [0u64; 16];
        let mut block_num = r_block_num;
        for i in 0..self.layer_num {
            need_block_num_as_layer[i] = block_num;
            block_num = (block_num + 0x3E) >> 6;
        }
        let mut deep = self.layer_num - 1;
        let mut index = 0;
        let mut count = 0;
        loop {
            let c = self.c(deep, index);
            let need_count = need_block_num_as_layer[deep];
            for i in 0..64 {
                if *c >> i & 1 == 0 {
                    count += 1;
                    if count == need_count {
                        return Some(index);
                    }
                } else {
                    count = 0;
                }
            }
        }
    }

    #[inline(always)]
    pub fn fill_free_block(&mut self, block_index: u64) {
        let mut index = block_index >> 6;
        let mut mode = block_index & 0x3F;
        for i in 0..self.layer_num {
            let c = self.c(i, index);
            *c |= 1 << mode;
            if *c != u64::MAX {
                break;
            }
            mode = index & 0x3F;
            index >>= 6;
        }
    }
}

/// `u64` に `log64_ceil()` を実装
trait Log64Ext {
    fn log64_ceil(self) -> usize;
}

impl Log64Ext for u64 {
    fn log64_ceil(self) -> usize {
        if self <= 1 {
            return 1;
        }
        let log = self.ilog(64);
        log as usize + ((self > 64u64.pow(log)) as usize)
    }
}

use std::time::Instant;

fn main() {
    // FreeMap のテスト開始
    println!("=== FreeMap テスト開始 ===");

    // FreeMap の初期化
    let size: u64 = 16_000_000_000; // 64GB のブロックサイズを仮定
    let mut free_map = FreeMap::new(size);
    println!("FreeMap を作成しました (サイズ: {} bytes)", size);

    loop {
    // 空きブロックを検索
    let start = Instant::now();
    match free_map.search_free_block() {
        Some(index) => {
            let duration = start.elapsed();
            println!("空きブロックが見つかりました: {}", index);
            println!("処理時間: {:?}", duration);
            free_map.fill_free_block(index);
        }
        None => {
            let duration = start.elapsed();
            println!("空きブロックが見つかりませんでした");
            println!("処理時間: {:?}", duration);
        }
    }
}
    println!("=== FreeMap テスト終了 ===");
}