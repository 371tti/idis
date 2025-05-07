use std::path::PathBuf;
use tokio::fs::File;


/// IDIS Virtual Disk(IDVD) format
pub struct IDVD {
    pub path: PathBuf,
    pub size: u64, // in bytes
    pub block_size: u64, // in bytes
    pub bitmap_pos: u64, // in blocks
    pub cluster_index_pos: u64, // in blocks
    pub fs_index_addr: u64, // in blocks
    pub id_index_addr: u64, // in blocks
    pub vd_gen: u64, // snapshot number
    pub hash_seed: u64, // hash seed
    pub vd_version: u8, // version number

    pub file: File,
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

// impl IDVD {
//     pub async fn load(path: &PathBuf) -> Result<Self, IDVDError> {
//         let mut file = OpenOptions::new()
//             .read(true)
//             .write(true)
//             .create(false)
//             .open(path)
//             .await
//             .map_err(|_| IDVDError::VDNotFound)?;

//         let mut buf = [0u8; 64];
//         file.read_exact(&mut buf)
//             .await
//             .map_err(|_| IDVDError::InvalidFormat)?;

//         let vd_version = buf[7] as u8;
//         if vd_version != 1 {
//             return Err(IDVDError::NotSupportedVersion);
//         }

//         let vd_gen = u64::from_le_bytes(buf[8..16].try_into().unwrap());
//         let hash_seed = u64::from_le_bytes(buf[16..24].try_into().unwrap());
//         let block_size = u64::from_le_bytes(buf[24..32].try_into().unwrap());
//         let size = u64::from_le_bytes(buf[32..40].try_into().unwrap());
//         let block_index_pos = u64::from_le_bytes(buf[40..48].try_into().unwrap());
//         let fs_index_addr = u64::from_le_bytes(buf[48..56].try_into().unwrap());
//         let id_index_addr = u64::from_le_bytes(buf[56..64].try_into().unwrap());

//         Ok(Self {
//             path: path.clone(),
//             size,
//             block_size,
//             block_index_pos,
//             fs_index_addr,
//             id_index_addr,
//             vd_gen,
//             vd_version,
//             hash_seed,
//             file,
//         })
//     }

//     pub async fn new(path: &PathBuf, size: u64, block_size: u64) -> Result<Self, IDVDError> {
//         let file = OpenOptions::new()
//             .read(true)
//             .write(true)
//             .create(true)
//             .open(path)
//             .await
//             .map_err(|_| IDVDError::OSPermissionDenied)?;

//         let block_index_pos: u64 = 1;
//         let fs_index_addr: u64 = 1;
//         let id_index_addr: u64 = 0;
//         let mut rng = OsRng;
//         let hash_seed = rng
//             .try_next_u64()
//             .map_err(|_| IDVDError::FiledGetOsRng)?;
//         let vd_version: u8 = 0;
//         let vd_gen: u64 = 0;

//         Ok(Self {
//             path: path.clone(),
//             size,
//             block_size,
//             block_index_pos,
//             fs_index_addr,
//             id_index_addr,
//             vd_gen,
//             vd_version,
//             hash_seed,
//             file,
//         })
//     }

//     pub fn size(&self) -> u64 {
//         self.size
//     }

//     pub fn block_size(&self) -> u64 {
//         self.block_size
//     }

//     pub fn block_index_pos(&self) -> u64 {
//         self.block_index_pos
//     }

//     pub async fn seek(&mut self, pos: u64) -> Result<(), IDVDError> {
//         self.file
//             .seek(std::io::SeekFrom::Start(pos))
//             .await
//             .map(|_| ())
//             .map_err(|_| IDVDError::OSPermissionDenied)
//     }

//     pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, IDVDError> {
//         self.file
//             .read(buf)
//             .await
//             .map_err(|_| IDVDError::OSPermissionDenied)
//     }

//     pub fn file(&mut self) -> &mut File {
//         &mut self.file
//     }

//     pub fn buf_write(&mut self) -> BufWriter<&mut File> {
//         BufWriter::new(self.file())
//     }

//     pub async fn write(&mut self, buf: &[u8]) -> Result<usize, IDVDError> {
//         self.file
//             .write(buf)
//             .await
//             .map_err(|_| IDVDError::OSPermissionDenied)
//     }

//     pub fn resize(&mut self, add_size: u64) {
//         self.size += add_size;
//     }
// }