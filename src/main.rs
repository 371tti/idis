use std::{result, time::Instant};

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
}

/// `usize` に `log64_ceil()` を実装
trait Log64Ext {
    fn log64_ceil(self) -> usize;
}

impl Log64Ext for usize {
    fn log64_ceil(self) -> usize {
        if self <= 1 {
            return 1;
        }
        let log = self.ilog(64);
        log as usize + ((self > 64usize.pow(log)) as usize)
    }
}

fn main() {
    println!("=== PowerMap テスト開始 ===");

    let block_num = 62_500_000;
    let mut power_map = PowerMap::new(block_num);
    println!("PowerMap を作成しました (ブロック数: {})", block_num);

    let start = Instant::now();
    match power_map.search_free_block() {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    let start = Instant::now();
    match power_map.search_free_block() {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    let start = Instant::now();
    match power_map.search_free_block() {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    let start = Instant::now();
    match power_map.search_free_block() {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    let start = Instant::now();
    match power_map.search_free_block() {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    let start = Instant::now();
    let result = power_map.search_free_block();
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    match result {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let start = Instant::now();
    let result = power_map.search_free_block();
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    match result {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    let start = Instant::now();
    let result = power_map.search_free_block();
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    match result {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
    loop {
    let start = Instant::now();
    let result = power_map.search_free_block();
    let duration = start.elapsed();
    println!("処理時間: {:?}", duration);
    match result {
        Some(index) => println!("空きブロックの位置: {}", index),
        None => println!("空きブロックが見つかりませんでした"),
    }
}
    

    println!("=== PowerMap テスト終了 ===");
}
