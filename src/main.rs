use idis::{cash, utils::target::get_bytes_per_sector};
use linked_hash_map::LinkedHashMap;
use lru::LruCache;
use tokio::{fs::File, io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt}};

#[cfg(target_os = "windows")]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(target_os = "windows")]
use winapi::um::winbase::FILE_FLAG_NO_BUFFERING;

pub struct IDVD {
    pub size: u64,
    pub block_size: u64,
    pub bit_map_pos: u64,
    pub cluster_index_pos: u64,
    pub vd_gen: u64,
    pub hash_seed: u64,
    pub vd_version: u64,

    pub cash_idvd: Cash,
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

use std::{alloc::{alloc, dealloc, Layout}, collections::{BTreeMap, HashMap, VecDeque}, f32::consts::E, num::NonZero, ops::{Deref, DerefMut}, path::Path, ptr::{self, NonNull}, time::Instant};

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
    #[inline]
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
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }
}

// スライスとして扱えるようにする
impl Deref for CashEntry {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
}

impl DerefMut for CashEntry {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}


impl Clone for CashEntry {
    fn clone(&self) -> Self {
        // 新しい領域を確保
        let mut new_entry = CashEntry::with_size_aligned(self.size, self.align);
        // 中身をディープコピー
        new_entry.as_mut_slice().copy_from_slice(&self[..]);
        new_entry
    }
}


impl Drop for CashEntry {
    #[inline]
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
    #[inline]
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
    #[inline]
    pub async fn read_block(&mut self, block_pos: u64) -> io::Result<CashEntry> {
        // キャッシュに存在する場合
        if self.map.contains(&block_pos) {
            return Ok(self.map.get(&block_pos).unwrap().clone());
        }

        let mut entry = CashEntry::with_size_aligned(self.block_size as usize, self.block_size as usize);
        let mut buf = entry.as_mut_slice();
        self.file
            .seek(std::io::SeekFrom::Start(block_pos * self.block_size))
            .await?;
        self.file.read_exact(&mut buf).await?;

        Ok(entry)
    }

    #[inline]
    pub async fn read_block_cashing(&mut self, block_pos: u64) -> io::Result<&CashEntry> {
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

    #[inline]
    pub fn contain(&self, block_pos: u64) -> bool {
        self.map.contains(&block_pos)
    }

    /// ブロックを書き込む（キャッシュがあれば更新、ファイルにも即書き込み）
    #[inline]
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
    /// 最適化すべき
    #[inline]
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

    /// キャッシュをクリアする
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

/// キャッシュ構造体
pub struct Cash {
    pub driver: DriverCash,
}

impl Cash {
    /// Create new Cash instance
    /// cashed size is floor(size divided by os_fs_sector_size)
    #[inline]
    pub async fn new(path: &Path, size: u64) -> Self {
        {
            let dir = path.parent()
                .unwrap_or_else(|| Path::new(r"I:\"));
            let sector_size = get_bytes_per_sector(dir).unwrap() as u64;
            // サイズをセクタサイズで割る. 切り上げ.
            let cashed_block_size = size / sector_size as u64;
            let file = tokio::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                // Windows-specific flag: disable buffering
                .custom_flags(FILE_FLAG_NO_BUFFERING)
                .open(path)
                .await
                .unwrap();
            let driver = DriverCash::new(file, cashed_block_size as usize, sector_size);
            Self { driver }
        }
    }

    /// read data into buffer beginning at position `pos`
    #[inline]
    pub async fn read(&mut self, buffer: &mut [u8], pos: u64) -> io::Result<()> {
        let mut remaining = buffer.len();
        let mut buf_offset = 0;
        let mut current_pos = pos;

        while remaining > 0 {
            let block_pos = current_pos / self.driver.block_size;
            let block_offset = (current_pos % self.driver.block_size) as usize;
            let entry = self.driver.read_block_cashing(block_pos).await?;
            let data = entry.as_ref(); // &[u8] のビュー
            // このブロックからコピーできる長さ
            let to_copy = std::cmp::min(remaining, data.len() - block_offset);

            buffer[buf_offset..buf_offset + to_copy]
                .copy_from_slice(&data[block_offset..block_offset + to_copy]);

            buf_offset += to_copy;
            current_pos += to_copy as u64;
            remaining -= to_copy;
        }

        Ok(())
    }
    
    pub async fn write(&mut self, buffer: &mut [u8], pos: u64) -> io::Result<()> {
        // 事前計算
        let len = buffer.len();
        let first_block_pos = pos / self.driver.block_size;
        let first_block_offset = (pos % self.driver.block_size) as usize;
        let mut buffer_seek = 0;

        // 重なり分部をオーバーライド
        let mut first_block = self.driver.read_block(first_block_pos).await?;
        let to_copy = std::cmp::min(len, self.driver.block_size as usize - first_block_offset);
        first_block[first_block_offset..first_block_offset + to_copy].copy_from_slice(&buffer[..to_copy]);
        buffer_seek += to_copy;
        self.driver.write_block(first_block_pos, first_block).await?;

        // バッファが1ブロックの書き換えのみなら終了
        if buffer_seek == len {
            return Ok(());
        }

        // ブロックまるまるかきかえるやつを一気に書き込む
        let mut now_block_pos = (pos + buffer_seek as u64) / self.driver.block_size;
        while self.driver.block_size <= (len - buffer_seek) as u64 {
            let mut new_block = CashEntry::with_size_aligned(self.driver.block_size as usize, self.driver.block_size as usize);
            new_block.copy_from_slice(&buffer[buffer_seek..(buffer_seek + self.driver.block_size as usize)]);
            self.driver.write_block(now_block_pos as u64, new_block).await?;
            buffer_seek += self.driver.block_size as usize;
            now_block_pos += 1;
        }

        // 余りがなかったら終了
        if buffer_seek == len {
            return Ok(());
        }

        // 後ろの重なり部分をオーバーライド
        let mut last_block = self.driver.read_block(now_block_pos).await?;
        last_block[..(len - buffer_seek)].copy_from_slice(&buffer[buffer_seek..]);
        self.driver.write_block(now_block_pos, last_block).await?;

        Ok(())
    }

    pub async fn sync(&mut self) -> io::Result<()> {
        self.driver.sync().await
    }
}

#[tokio::main]
async fn main() {

    // キャッシュ用ファイルとブロックサイズ
    let path = Path::new("I:\\RustBuilds\\IDIS\\main\\idis\\lol.bin");
    // ① まずファイルを開いて……
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .await.unwrap();

    // ② 16KiB (16384 バイト) に伸ばす
    file.set_len(16 * 1024).await.unwrap();
    let block_size = 1024u64;
    let mut cache = Cash::new(path, block_size).await;

    // 読み込み用バッファ
    let mut buffer = vec![0u8; block_size as usize];

    // 1 回目の読み込み（ファイルから）
    let t1 = Instant::now();
    cache.read(&mut buffer, 0).await.unwrap();
    let d1 = t1.elapsed();
    println!("First read took: {:?}", d1);

    // 2 回目の読み込み（キャッシュから）
    let t2 = Instant::now();
    cache.read(&mut buffer, 0).await.unwrap();
    let d2 = t2.elapsed();
    println!("Second read took: {:?}", d2);

    // 文字列を1回だけ書き込むテスト
    let mut data = tokio::fs::read("buf.txt").await.unwrap();
    cache.write(&mut data, 0).await.unwrap();
    cache.sync().await.unwrap();

    // 書き込んだ内容を読み込んで確認
    let mut buf = vec![0u8; data.len()];
    cache.read(&mut buf, 0).await.unwrap();
    println!("Read data: {:?}", String::from_utf8_lossy(&buf));
}