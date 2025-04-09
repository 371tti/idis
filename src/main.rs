use idis::cash;
use linked_hash_map::LinkedHashMap;
use lru::LruCache;
use tokio::{fs::File, io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt}};

pub struct IDVD {
    pub size: u64,
    pub block_size: u64,
    pub bit_map_pos: u64,
    pub cluster_index_pos: u64,
    pub vd_gen: u64,
    pub hash_seed: u64,
    pub vd_version: u64,

    pub cash_idvd: IDVDCash,
}





pub struct RawIDVD {
    pub free_map: FreeMap,
    pub id_map: IDMap,
    pub cluster_map: ClusterMap,
    pub object_map: ObjectMap,
}

pub struct IDMap {
    pub ruid: Vec<u128>,
    pub map_pos: Vec<u64>,
}

pub struct ClusterMap {
    pub ruid: u128,
    pub len: u64,
}

pub struct ClusterEntry {
    pub generation: u64,
    pub cluster: Vec<Cluster>,
}

pub struct Cluster {
    pub pos: u64,
    pub len: u64,
}

pub struct ObjectMap {
    pub map: BTreeMap<u128, ObjectEntry>,
}

pub struct ObjectEntry {
    pub len: u64,
    pub pos: u64,
    pub object: Vec<u8>,
}

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
                    pow_map.push(Vec::with_capacity(Self::PAGE_CAPACITY));
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

    /// 線形走査で連続する空ブロックを探索するシンプル版
    /// req は要求する連続空ブロック数（lowest layer のビット単位）
    #[inline(always)]
    pub fn search_free_blocks(&mut self, r_size: u64) -> Option<u64> {
        let mut current_index: u64 = 0;
        let mut count: u64 = 0;
        let limit_index = self.size - r_size - 1;
        loop {
            // 上位レイヤーが埋まっているかどうかを確認
            // 埋まってたらスキップ
            'outer: loop {
                if current_index > limit_index {
                    return None;
                }
                for i in (0..self.layer_num).rev() {
                    if i == 0 {
                        break 'outer;
                    }
                    let mode = current_index & (u64::MAX >> (64 - (6 * i)));
                    let index = current_index >> (6 * i);
                    if mode == 0 {
                        let c = ((*self.c(i, index >> 6) >> (index & 0x3F)) & 1) != 0;
                        if c {
                            current_index += 1 << (6 * i);
                            count = 0;
                            break;
                        }
                    }
                }
            }
            // 連続する空ブロックを探索
            // うまっていた時点でbreak
            loop {
                if current_index > limit_index {
                    return None;
                }
                let c = ((*self.c(0, current_index >> 6) >> (current_index & 0x3F)) & 1) != 0;
                if c {
                    // 途中で埋まっているブロックが見つかった場合はカウントをリセットして次に進む
                    count = 0;
                    current_index += 1;
                    break;
                } else {
                    count += 1;
                    if count == r_size {
                        return Some(current_index - (r_size - 1));
                    }
                    current_index += 1;
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

    #[inline(always)]
    pub fn fill_blocks(&mut self, block_index: u64, r_size: u64) {
        for i in 0..r_size {
            self.fill_free_block(block_index + i);
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

use std::{alloc::{alloc, dealloc, Layout}, collections::{BTreeMap, HashMap, VecDeque}, f32::consts::E, num::NonZero, ops::{Deref, DerefMut}, ptr::{self, NonNull}, time::Instant};

// fn main() {
//     // FreeMap のテスト開始
//     println!("=== FreeMap テスト開始 ===");

//     // FreeMap の初期化
//     let size: u64 = 16_000_000_02; // 64GB のブロックサイズを仮定
//     let mut free_map = FreeMap::new(size);
//     println!("FreeMap を作成しました (サイズ: {} bytes)", size);

//     let mut allocated_count: u64 = 0;
//     loop {
//         let num = 1; // 探索したい連続する空きブロックの数
//         let start = Instant::now();
//         match free_map.search_free_blocks(num) {
//             Some(start_index) => {
//                 let duration = start.elapsed();
//                 println!("連続した {} 個の空きブロックが見つかりました: {}", num, start_index);
//                 println!("処理時間: {:?}", duration);
//                 // 見つかった連続ブロックを埋める
//                 println!("埋めます");
//                 free_map.fill_blocks(start_index, num);
//                 allocated_count += 1;
//                 println!("埋め終わりました。これまでに確保したブロック数: {}", allocated_count);
//             },
//             None => {
//                 let duration = start.elapsed();
//                 println!("連続した {} 個の空きブロックが見つかりませんでした", num);
//                 println!("処理時間: {:?}", duration);
//                 break;
//             }
//         }
//     }
//     println!("=== FreeMap テスト終了 ===");
// }


/// キャッシュエントリ
/// 
/// キャッシュエントリは、メモリを確保し、スライスとして扱えるようにする構造体です。
pub struct CashEntry {
    ptr: NonNull<u8>,
    size: usize,
    align: usize,
}

impl CashEntry {
    /// サイズとアライメントを指定してメモリを確保する
    /// # Safety
    /// メモリの使用に関しては、適切なアライメントとサイズを保証する必要があります。
    /// 
    /// # Arguments
    /// * `size` - 確保するメモリのサイズ
    /// * `align` - メモリのアライメント
    /// 
    /// # Returns
    /// * `CashEntry` - 確保されたメモリを持つ構造体
    pub fn with_size_aligned(size: usize, align: usize) -> Self {
        // リリースビルドで消える
        assert!(align.is_power_of_two(), "Alignment must be a power of 2");

        let layout = Layout::from_size_align(size, align).expect("Invalid layout");
        let ptr = unsafe { alloc(layout) };

        let ptr = NonNull::new(ptr).expect("Failed to allocate memory");

        Self { ptr, size, align }
    }

    /// 可変スライスとしてメモリを取得する
    /// # Safety
    /// メモリの使用に関しては、適切なアライメントとサイズを保証する必要があります。
    /// 
    /// # Returns
    /// * `&mut [u8]` - 可変スライス
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }
}

// スライスとして扱えるようにする
impl Deref for CashEntry {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
}

impl DerefMut for CashEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for CashEntry {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align(self.size, self.align).unwrap();
            dealloc(self.ptr.as_ptr(), layout);
        }
    }
}


/// cashドライバ
/// ブロック単位でキャッシュを管理する
pub struct DriverCash {
    pub cashed_max_blocks: usize,
    pub block_size: u64,
    pub map: LruCache<u64, CashEntry>,
    pub write_buf: Vec<(u64, CashEntry)>,
    pub file: File,
}

impl DriverCash {
    pub fn new(file: File, cashed_max_blocks: usize, block_size: u64) -> Self {
        Self {
            cashed_max_blocks,
            block_size,
            map: LruCache::new(NonZero::new(cashed_max_blocks).unwrap()),
            write_buf: Vec::new(),
            file,
        }
    }

    /// ブロックを読み込む（キャッシュに無ければファイルから）
    pub async fn read_block(&mut self, block_pos: u64) -> io::Result<&CashEntry> {
        // キャッシュに存在する場合
        if self.map.contains(&block_pos) {
            return Ok(self.map.get(&block_pos).unwrap());
        }

        let mut entry = CashEntry::with_size_aligned(self.block_size as usize, self.block_size as usize);
        let mut buf = entry.as_mut_slice();
        self.file
            .seek(std::io::SeekFrom::Start(block_pos * self.block_size))
            .await?;
        self.file.read_exact(&mut buf).await?;

        self.map.put(block_pos, entry);

        Ok(self.map.get(&block_pos).unwrap())
    }

    pub fn contain(&self, block_pos: u64) -> bool {
        self.map.contains(&block_pos)
    }

    /// ブロックを書き込む（キャッシュがあれば更新、ファイルにも即書き込み）
    pub async fn write_block(&mut self, block_pos: u64, data: CashEntry) -> io::Result<()> {
        // キャッシュに存在する場合
        if let Some(entry) = self.map.get_mut(&block_pos) {
            if entry.size != data.size {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid block size"));
            }
            entry.copy_from_slice(&data);
        }

        self.write_buf.push((block_pos, data));
        Ok(())
    }

    /// 強制的にファイルをフラッシュ（整合性のため）
    pub async fn sync(&mut self) -> io::Result<()> {
        self.write_buf.sort_by_key(|(block_pos, _)| *block_pos);
        for (block_pos, entry) in &self.write_buf {
            self.file
                .seek(std::io::SeekFrom::Start(block_pos * self.block_size))
                .await?;
            self.file.write_all(entry).await?;
        }
        self.file.flush().await?;
        self.file.sync_all().await?;
        Ok(())
    }
}

pub struct Cash {
    pub driver: DriverCash,
}

impl Cash {
    pub fn new() -> Self {
        Self {}
    }
}


#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("lol.bin")
        .await
        .unwrap();
    let mut driver_cash = DriverCash::new(file, 10, 4096);
    driver_cash.resize(20);
    let block_pos = 0;
    let data = b"Hello, world!";
    driver_cash.write(block_pos, data).await.unwrap();
    driver_cash.write(block_pos + 1, data).await.unwrap();
    driver_cash.flush(block_pos);
    driver_cash.flush(block_pos + 1);
    driver_cash.sync().await.unwrap();
    let data = driver_cash.read(block_pos).await.unwrap();
    println!("Read data as UTF-8: {}", String::from_utf8_lossy(&data.data));
}