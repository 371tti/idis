use std::{backtrace, result, time::Instant, u64};

use idis::ton::serde::value::index;

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
    fn search_free_block(&self) -> Option<u64> {
        let mut index: u64 = 0;
        for map in &self.free_map {
            println!("map_binary: {:064b}", map[index as usize]);
            let c = (map[index as usize] + 1).trailing_zeros() as u64;
            if c == 64 {
                return None;
            }
            index = (index << 6) | c;
        }
        Some(index)
    }

    fn search_free_blocks(&self) -> Option<usize> {

    }

    fn fill_free_block(&mut self, r_index: usize) {
        let mut index = r_index;
        for map in &mut self.free_map.iter_mut().rev() {
            let c = index & 0x3f;
            index = index >> 6;
            map[index] |= 1 << c;
            if map[index] != u64::MAX {
                break;
            } 
        }
    }

 
 
    fn fill_free_blocks(&mut self, r_index: u64, r_len: u64) {
        let mut index = r_index;
        let mut len = r_len;
        // 各レイヤーを下位から順に処理
        for map in &mut self.free_map.iter_mut().rev() {
            // １レイヤーあたりのブロック数（あらかじめ計算）
            let block_num = (len + 63) >> 6;
            let mut seek = index & 0x3f;
            // 必要に応じた範囲を埋める
            let mut i = 0;
            while len != 0 {
                // 最初のブロックだけはオフセット c を適用
                let available = 64 - seek;
                let fill_count = available.min(len);
                let mask = u64::MAX >> (64 - fill_count) << seek;
                map[(index >> 6) + i] |= mask;
                // 利用可能ビット分を引く（不足なら 0 になる）
                len = len.saturating_sub(available);
                i += 1;
                seek = 0;
            }
            // 次のレイヤーの初期位置に移行
            index >>= 6;
            // 次レイヤーで埋まっているブロックを検索（最初に見つかった free = u64::MAX の位置へ）
            if let Some(offset) = (0..block_num).find(|&i| map[index + i] == u64::MAX) {
                index += offset;
                // free のブロック数を再計算（次レイヤーで連続して free なら len を伸ばす）
                len += (0..block_num).filter(|&i| map[index + i] == u64::MAX).count();
            } else {
                return;
            }
        }
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


    loop {
        let start = Instant::now();
        let result = power_map.search_free_block();
        let duration = start.elapsed();
        println!("処理時間: {:?}", duration);
        match result {
            Some(index) => {
                //power_map.fill_free_block(index);
                power_map.fill_free_blocks(index, 1000000);
                println!("空きブロックの位置: {}", index)
            },
            None => println!("空きブロックが見つかりませんでした"),
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    

    println!("=== PowerMap テスト終了 ===");
}
