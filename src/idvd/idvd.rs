use std::fs::Permissions;
use std::path::PathBuf;

use rand::rngs::OsRng;
use rand::seq::index;
use rand::TryRngCore;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;
use std::io::Read;

use super::error::IDVDError;
use tokio::fs::OpenOptions;
use tokio::fs::File;
use tokio::io::AsyncReadExt;


/// IDIS Virtual Disk(IDVD) format
/// structure
/// =====================
/// padding: 64 - 8 bit
/// ---------------------
/// vd_version: 8bit
/// ---------------------
/// vd_gen: 64bit
/// ---------------------
/// hash_seed: 64bit
/// ---------------------
/// block_size: 64bit
/// ---------------------
/// size: 64bit
/// ---------------------
/// block_index_pos: 64bit
/// ---------------------
/// fs_pos: 64bit
/// ---------------------
/// id_index_pos: 64bit
/// =====================
/// 
/// 
pub struct IDVD {
    pub path: PathBuf,
    pub size: u64, // in bytes
    pub block_size: u64, // in bytes
    pub block_index_pos: u64, // in blocks
    pub fs_index_addr: u64, // in blocks
    pub id_index_addr: u64, // in blocks
    pub vd_gen: u64, // snapshot number
    pub hash_seed: u64, // hash seed
    pub vd_version: u8, // version number

    pub file: File,
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
    pub fn new(size: u64) -> Self {
        let layer_num = size.log64_ceil();
        let mut raw_size: u64 = 0;
        for i in 0..layer_num {
            raw_size += size >> (i * 6);
        }
        let page_num = (raw_size + 0xFFFF_FFFE) >> 32;
        let raw_size_mod = raw_size & 0xFFFF_FFFF;
        let mut pow_map: Vec<Vec<u64>> = Vec::with_capacity(page_num as usize);
        for i in 0..page_num {
            let capacity = if i == page_num - 1 {
            raw_size_mod as usize
            } else {
            0xFFFF_FFFF as usize
            };
            pow_map.push(vec![0; capacity]);
        }
        FreeMap { pow_map, size, layer_num }
    }

    /// ある深さのindexの要素を取得する
    /// indexはu64
    /// 内部でu32のrangeで分割
    /// 
    /// # Arguments
    /// * `deep` - ページの深さ
    /// * `index` - ブロックのインデックス
    #[inline(always)]
    pub fn c(&self, deep: usize, index: u64) -> u64 {
        const U32MASK: usize = 0xFFFF_FFFF;
        // bitmap のオフセットを計算
        // ブロック数がu64::MAXに近い場合、mapのほうがドライブ容量より
        // 大きくなってしまうのでオーバーフローは発生しえない
        let offset = self.precomputed_offset(deep);
        let page: usize = ((offset + index) >> 32) as usize;
        let index: usize = (offset + index) as usize & U32MASK;
        self.pow_map[page][index]
    }

    #[inline(always)]
    fn precomputed_offset(&self, deep: usize) -> u64 {
        let mut offset: u64 = 0;
        for i in 0..deep {
            offset += self.size >> (i * 6);
        }
        offset
    }

    #[inline(always)]
    pub fn search_free_block(&self) -> Option<u64> {
        let mut index: u64 = 0;
        for i in (0..self.layer_num).rev() {
            println!("map_binary: {:064b}", self.c(i, index));
            let c = (self.c(i, index) + 1).trailing_zeros() as u64;
            if c == 64 {
                return None;
            }
            index = (index << 6) | c;
        }
        Some(index)
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

/// block index の代わりにもする
pub struct RuidIndex {
    // ソートしておくべきかも
    pub ruid: Vec<u128>,
    pub value: Vec<BlockIndex>,
    // lazy load するかも
}

pub struct BlockIndex {
    pub value: Vec<BlockIndexData>
}

pub struct BlockIndexData {
    pub pos: u64,
    pub len: u64,
}

pub struct FSIndex {
    /// ファイルのタイプフラグ
    pub type_flag: u8,
    /// 親ディレクトリの ruid
    pub referrer: u128,
    /// ファイルの ruid
    pub ruid: u128,
    /// latest access timestamp
    pub timestamp_la: u64,
    /// latest create timestamp
    pub timestamp_ct: u64,
    /// latest modify timestamp
    pub timestamp_lm: u64,
    /// latest change timestamp(meta or perm)
    pub timestamp_lc: u64,
    /// directory or file name
    pub name: String,
    /// データのクラスタアドレス
    pub data_addr: u64,
    pub links: FSLink,
    pub perm: FSPermission,
}

/// Struct of Array で実装
pub struct FSLink {
    /// ファイル名のハッシュ値
    /// ローカルハッシュから作成
    pub hash: Vec<u128>,
    /// ファイルのruid
    pub ruid: Vec<u128>,
}

/// Struct of Array で実装
pub struct FSPermission {
    pub ruid: Vec<u128>,
    pub flag_map: Vec<u8>,
}

impl FSPermission {
    pub fn new(ruid: Vec<u128>, flag_map: Vec<u8>) -> Self {
        Self { ruid, flag_map }
    }

    pub fn get_flag(&self, ruid: u128) -> Option<u8> {
        let pos = self.ruid.binary_search(&ruid).ok()?;
        Some(self.flag_map[pos])
    }

    pub fn add(&mut self, ruid: u128, flag: u8) {
        let pos = self.ruid.binary_search(&ruid).unwrap_or_else(|i| i);
        self.ruid.insert(pos, ruid);
        self.flag_map.insert(pos, flag);
    }

    pub fn get_list(&self) -> Vec<(u128, u8)> {
        self.ruid.iter().zip(self.flag_map.iter()).map(|(ruid, flag)| (*ruid, *flag)).collect()
    }

    pub fn remove(&mut self, ruid: u128) -> Option<u8> {
        let pos = self.ruid.binary_search(&ruid).ok()?;
        self.ruid.remove(pos);
        Some(self.flag_map.remove(pos))
    }

    pub fn contains(&self, ruid: u128) -> bool {
        self.ruid.binary_search(&ruid).is_ok()
    }
}

pub enum FSPermissions {
    Visible,
    Read,
    Write,
    Modify,
    Edit,
    Delete,
    Copy,
    Moveable,
}

impl FSPermissions {
    pub fn generate_flag(list: &[FSPermissions]) -> u8 {
        let mut flag: u8 = 0;
        for permission in list {
            match permission {
                FSPermissions::Visible => flag |= 0b1000_0000,
                FSPermissions::Read => flag |= 0b0100_0000,
                FSPermissions::Write => flag |= 0b0010_0000,
                FSPermissions::Modify => flag |= 0b0001_0000,
                FSPermissions::Edit => flag |= 0b0000_1000,
                FSPermissions::Delete => flag |= 0b0000_0100,
                FSPermissions::Copy => flag |= 0b0000_0010,
                FSPermissions::Moveable => flag |= 0b0000_0001,
            }
        }
        flag
    }

    pub fn from_flag(flag: u8) -> Vec<FSPermissions> {
        let mut list = Vec::new();
        if flag & 0b1000_0000 != 0 {
            list.push(FSPermissions::Visible);
        }
        if flag & 0b0100_0000 != 0 {
            list.push(FSPermissions::Read);
        }
        if flag & 0b0010_0000 != 0 {
            list.push(FSPermissions::Write);
        }
        if flag & 0b0001_0000 != 0 {
            list.push(FSPermissions::Modify);
        }
        if flag & 0b0000_1000 != 0 {
            list.push(FSPermissions::Edit);
        }
        if flag & 0b0000_0100 != 0 {
            list.push(FSPermissions::Delete);
        }
        if flag & 0b0000_0010 != 0 {
            list.push(FSPermissions::Copy);
        }
        if flag & 0b0000_0001 != 0 {
            list.push(FSPermissions::Moveable);
        }
        list
    }

    pub fn is_visible(flag: u8) -> bool {
        flag & 0b1000_0000 != 0
    }

    pub fn is_readable(flag: u8) -> bool {
        flag & 0b0100_0000 != 0
    }

    pub fn is_editable(flag: u8) -> bool {
        flag & 0b0000_1000 != 0
    }

    pub fn is_writable(flag: u8) -> bool {
        flag & 0b0010_0000 != 0
    }

    pub fn is_modifiable(flag: u8) -> bool {
        flag & 0b0001_0000 != 0
    }

    pub fn is_copyable(flag: u8) -> bool {
        flag & 0b0000_0010 != 0
    }

    pub fn is_deletable(flag: u8) -> bool {
        flag & 0b0000_0100 != 0
    }

    pub fn is_moveable(flag: u8) -> bool {
        flag & 0b0000_0001 != 0
    }
}

impl IDVD {
    pub async fn load(path: &PathBuf) -> Result<Self, IDVDError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(path)
            .await
            .map_err(|_| IDVDError::VDNotFound)?;

        let mut buf = [0u8; 64];
        file.read_exact(&mut buf)
            .await
            .map_err(|_| IDVDError::InvalidFormat)?;

        let vd_version = buf[7] as u8;
        if vd_version != 1 {
            return Err(IDVDError::NotSupportedVersion);
        }

        let vd_gen = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let hash_seed = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let block_size = u64::from_le_bytes(buf[24..32].try_into().unwrap());
        let size = u64::from_le_bytes(buf[32..40].try_into().unwrap());
        let block_index_pos = u64::from_le_bytes(buf[40..48].try_into().unwrap());
        let fs_index_addr = u64::from_le_bytes(buf[48..56].try_into().unwrap());
        let id_index_addr = u64::from_le_bytes(buf[56..64].try_into().unwrap());

        Ok(Self {
            path: path.clone(),
            size,
            block_size,
            block_index_pos,
            fs_index_addr,
            id_index_addr,
            vd_gen,
            vd_version,
            hash_seed,
            file,
        })
    }

    pub async fn new(path: &PathBuf, size: u64, block_size: u64) -> Result<Self, IDVDError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .await
            .map_err(|_| IDVDError::OSPermissionDenied)?;

        let block_index_pos: u64 = 1;
        let fs_index_addr: u64 = 1;
        let id_index_addr: u64 = 0;
        let mut rng = OsRng;
        let hash_seed = rng
            .try_next_u64()
            .map_err(|_| IDVDError::FiledGetOsRng)?;
        let vd_version: u8 = 0;
        let vd_gen: u64 = 0;

        Ok(Self {
            path: path.clone(),
            size,
            block_size,
            block_index_pos,
            fs_index_addr,
            id_index_addr,
            vd_gen,
            vd_version,
            hash_seed,
            file,
        })
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn block_size(&self) -> u64 {
        self.block_size
    }

    pub fn block_index_pos(&self) -> u64 {
        self.block_index_pos
    }

    pub async fn seek(&mut self, pos: u64) -> Result<(), IDVDError> {
        self.file
            .seek(std::io::SeekFrom::Start(pos))
            .await
            .map(|_| ())
            .map_err(|_| IDVDError::OSPermissionDenied)
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, IDVDError> {
        self.file
            .read(buf)
            .await
            .map_err(|_| IDVDError::OSPermissionDenied)
    }

    pub fn file(&mut self) -> &mut File {
        &mut self.file
    }

    pub fn buf_write(&mut self) -> BufWriter<&mut File> {
        BufWriter::new(self.file())
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, IDVDError> {
        self.file
            .write(buf)
            .await
            .map_err(|_| IDVDError::OSPermissionDenied)
    }

    pub fn resize(&mut self, add_size: u64) {
        self.size += add_size;
    }
}