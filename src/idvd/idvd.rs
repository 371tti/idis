use std::path::PathBuf;

use libc::rand;
use vec_plus::vec::default_sparse_vec::DefaultSparseVec;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};

use super::error::IDVDError;


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
}

pub struct BlockIndex {
    pub index: Vec<MapBlockIndex>,
    pub len: u64,
    // lazy load するかも
}

pub struct MapBlockIndex {
    pub key: u64,
    pub value: Vec<BlockIndexData>
}

pub struct BlockIndexData {
    pub pos: u64,
    pub len: u64,
}

pub struct IDIndex {
    // ソートしておくべきかも
    pub index: Vec<MapIDIndex>,
    pub len: u64,
    // lazy load するかも
}

#[derive(Default, Clone, PartialEq)]
pub struct MapIDIndex {
    // ruid の addr
    pub key: u128,
    // FSIndex の addr
    pub value: u64,
}

pub struct FSIndex {
    pub r#type: u8,
    pub referrer: u64,
    pub ruid: u128,
    // latest access timestamp
    pub timestamp_la: u64,
    // latest create timestamp
    pub timestamp_ct: u64,
    // latest modify timestamp
    pub timestamp_lm: u64,
    // latest change timestamp(meta or perm)
    pub timestamp_lc: u64,
    // directory or file name
    pub name: String,
    pub data_addr: u64,
    pub links: Vec<FSIndex/*addr*/>,
    pub perm: FSPermission,
}

pub struct FSPermission {
    pub list: Vec<PermissionType>,

}

pub struct PermissionType {
    /// role, user and group id
    pub ruid: u128,
    /// perm list
    /// - 0: READ,
    /// - 1: WRITE,
    /// - 2: EDIT,
    /// - 3: DELETE,
    /// - 4: COPY,
    /// - 5: MOVE,
    /// - 6: VISIBLE,
    /// - 7: MODIFY,
    pub r#type: u8,/*bitmap flag*/
}

pub struct MapFSIndex {
    pub key: u128/*file name hash*/,
    // スナップショットの番号
    pub back_num: u64,
    pub value: u64
}

impl IDVD {
    pub fn load(path: &PathBuf) -> Result<Self, IDVDError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(path)
            .map_err(|_| IDVDError::VDNotFound)?;

        let mut buf = [0u8; 64];
        file.read_exact(&mut buf).map_err(|_| IDVDError::InvalidFormat)?;

        let vd_version = buf[7] as u8;

        if vd_version != 1 {
            return Err(IDVDError::NotSupportedVersion);
        }

        let vd_gen = u64::from_le_bytes(buf[8..].try_into().unwrap());
        let hash_seed = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let block_size = u64::from_le_bytes(buf[24..32].try_into().unwrap());
        let size = u64::from_le_bytes(buf[32..40].try_into().unwrap());
        let block_index_pos = u64::from_le_bytes(buf[40..48].try_into().unwrap());
        let fs_index_pos = u64::from_le_bytes(buf[48..56].try_into().unwrap());
        let id_index_pos = u64::from_le_bytes(buf[56..64].try_into().unwrap());

        Ok(Self {
            path: path.clone(),
            size,
            block_size,
            block_index_pos,
            fs_index_addr: fs_index_pos,
            id_index_addr: id_index_pos,
            vd_gen,
            vd_version,
            hash_seed,
        })
    }


    pub fn new(path: &PathBuf, size: u64, block_size: u64) -> Self {
        let block_index_pos = 1;
        let fs_index_pos = 1;
        let id_index_pos = 0;
        let vd_gen = 0;
        let hash_seed = rand::random::<u64>();
        let vd_version = 1;
        
        Self {
            path: path.clone(),
            size,
            block_size,
            block_index_pos,
            fs_index_pos,
            id_index_pos,
            vd_gen,
            vd_version,
            hash_seed,
        }
    }
}