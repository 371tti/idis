
// ------------------------------------
// DriveHeader
// unit: byte
// ------------------------------------
#[repr(C)] // Ensure memory layout is consistent
struct DriveHeader {
    magic: [u8; 4],         // Magic number to identify the drive
    version: u32,           // Version of the drive format
    compression: u16,       // Compression algorithm used
    crypt_method: u16,      // Encryption method used
    nb_snapshots: u32,      // Number of snapshots in the drive
    total_size: u128,       // Total size of the drive in bytes
    meta_offset: u128,      // Offset to the meta (index) section in bytes
    data_offset: u128,      // Offset to the data section in bytes
    snapshot_offset: u128,  // Offset to the snapshot section in bytes
    block_size: u64,        // Block size in bytes
    entry_count: u64,       // Number of entries in the drive
}

enum CompressionMethod {
    None = 0,
    Zstd = 1,
}

enum CryptMethod {
    None = 0,
    Aes256 = 1,
}

const BLOCK_SIZE: usize = 4096; // 4KBブロック

#[repr(C)]
struct MetadataBlock {
    id: u64,                    // ノードID
    size_in_block: u64,         // ブロック数
    parent_id: u64,             // 親ノードID
    child_count: u32,           // 子ノードの数
    children_offsets: [u64; 128], // 子ノードのオフセット（最大128子ノード）
    attributes: [u8; 2048],     // 属性情報（例: JSON形式など）
}


#[repr(C)] // Ensure memory layout is consistent
struct DriveMeta {
    key: [u8; 32],          // Key of the entry
    offset: u128,           // Offset to the data in bytes
    size: u64,              // Size of the data in bytes
}

struct BlockEntry {
    block_id: u64,          // ブロックID
    data_size: u64,         // このブロック内のデータサイズ
    data_offset: u128,       // データの開始オフセット
    next_block_id: Option<u64>, // 次のブロックのID（複数ブロックにまたがる場合）
}
