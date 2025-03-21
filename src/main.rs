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
    pub fn new(r_size: u64) -> Self {
        let mut pow_map = Vec::new();
        // レイヤー数を計算
        let layer_num = r_size.log64_ceil();
        let mut size = r_size;
        // レイヤーごとに処理
        for i in 0..layer_num {
            // レイヤーのサイズとあまりを計算
            let layer_mode = size & 0x3F;
            let layer_size = (size + 0x3F) >> 6;
            // pageのサイズを計算
            let page_mod = layer_size & 0xFFFF_FFFF;
            let page_size = (layer_size + 0xFFFF_FFFE) >> 32;
            println!("layer_size: {}, page_size: {}", layer_size, page_size);
            // ページごとに処理
            for j in 0..page_size {
                if j == page_size - 1 {
                    let mut page = Vec::new();
                    // 最後のページの場合 あまり数で埋める
                    for k in 0..page_mod {
                        if k == page_mod - 1 {
                            // 最後のブロックの場合 あまり数で埋める
                            println!("layer_mode: {}", layer_mode);
                            let mask = u64::MAX << layer_mode;
                            page.push(mask);
                        } else {
                            page.push(0);
                        } 
                    } 
                    pow_map.push(page);
                } else {
                    pow_map.push(vec![0; 0xFFFF_FFFF]);
                }
            }
            size >>= 6;
        }
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
        println!("deep: {}, index: {}", deep, index);
        const U32MASK: usize = 0xFFFF_FFFF;
      
        let offset = self.precomputed_offset(deep);
        let page: usize = ((offset + index) >> 32) as usize;
        let index: usize = (offset + index) as usize & U32MASK;
        println!("page: {}, index: {}", page, index);
        &mut self.pow_map[page][index]
    }

    #[inline(always)]
    fn precomputed_offset(&self, deep: usize) -> u64 {
        let mut offset: u64 = 0;
        for i in 0..deep {
            offset += (self.size + ((1u64 << (i * 6)) - 1)) >> (i * 6);
        }
        offset
    }

    #[inline(always)]
    pub fn search_free_block(&mut self) -> Option<u64> {
        let mut block_index: u64 = 0;
        for i in (0..self.layer_num).rev() {
            println!("i: {}", i);
            println!("map_binary: {:064b}", self.c(i, block_index));
            let c = (*self.c(i, block_index) + 1).trailing_zeros() as u64;
            if c == 64 {
                return None;
            }
            block_index = (block_index << 6) | c;
        }
        Some(block_index)
    }

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
    let size: u64 = 90129301; // 64GB のブロックサイズを仮定
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