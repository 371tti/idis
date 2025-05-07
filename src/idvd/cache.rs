use std::{alloc::{alloc, dealloc, Layout}, num::NonZero, ops::{Deref, DerefMut}, path::Path, ptr::NonNull};

use lru::LruCache;
use tokio::{fs::File, io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt}};

use crate::utils::target::fs::{get_bytes_per_sector, open_file_direct};

pub struct CacheEntry {
    ptr: NonNull<u8>,
    size: usize,
    align: usize,
}

impl CacheEntry {
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
        debug_assert!(align.is_power_of_two(), "Alignment must be a power of 2");

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
impl Deref for CacheEntry {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
}

impl DerefMut for CacheEntry {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}


impl Clone for CacheEntry {
    fn clone(&self) -> Self {
        // 新しい領域を確保
        let mut new_entry = CacheEntry::with_size_aligned(self.size, self.align);
        // 中身をディープコピー
        new_entry.as_mut_slice().copy_from_slice(&self[..]);
        new_entry
    }
}


impl Drop for CacheEntry {
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
    pub map: LruCache<u64, CacheEntry>,
    pub write_buf: Vec<(u64, CacheEntry)>,
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
    pub async fn read_block(&mut self, block_pos: u64) -> io::Result<CacheEntry> {
        // キャッシュに存在する場合
        if self.map.contains(&block_pos) {
            return Ok(self.map.get(&block_pos).unwrap().clone());
        }

        let mut entry = CacheEntry::with_size_aligned(self.block_size as usize, self.block_size as usize);
        let mut buf = entry.as_mut_slice();
        self.file
            .seek(std::io::SeekFrom::Start(block_pos * self.block_size))
            .await?;
        self.file.read_exact(&mut buf).await?;

        Ok(entry)
    }

    #[inline]
    pub async fn read_block_cashing(&mut self, block_pos: u64) -> io::Result<&CacheEntry> {
        // キャッシュに存在する場合
        if self.map.contains(&block_pos) {
            return Ok(self.map.get(&block_pos).unwrap());
        }

        let mut entry = CacheEntry::with_size_aligned(self.block_size as usize, self.block_size as usize);
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
    pub async fn write_block(&mut self, block_pos: u64, data: CacheEntry) -> io::Result<()> {
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

    #[inline]
    pub fn drop_block(&mut self, block_pos: u64) {
        self.map.pop(&block_pos);
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
    pub async fn new(path: &Path, size: u64) -> io::Result<Self> {
        let dir = path.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;
        let sector_size = get_bytes_per_sector(dir)? as u64;
        // サイズをセクタサイズで割る. 切り下げ.
        let cashed_block_size = size / sector_size as u64;
        let file = open_file_direct(path).await?;
        let driver = DriverCash::new(file, cashed_block_size as usize, sector_size);
        Ok(Self { driver })
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
            let mut new_block = CacheEntry::with_size_aligned(self.driver.block_size as usize, self.driver.block_size as usize);
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

    pub async fn clear(&mut self) -> io::Result<()> {
        self.driver.clear();
        Ok(())
    }
}