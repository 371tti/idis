
// ------------------------------------
// DriveHeader
// unit: byte
// ------------------------------------
#[repr(C)] // Ensure memory layout is consistent
struct Header {
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
    id: u128,                    // ノードID
    size_in_block: u64,         // ブロック数
    date_created: u64,          // 作成日時
    date_modified: u64,         // 更新日時
    date_accessed: u64,         // 最終アクセス日時
    parent_id: u128,             // 親ノードID
    data_offset: u128,           // データの開始オフセット
    child_count: u32,           // 子ノードの数
    perm: Vec<u128>,               // ノード名
    children_offsets: Vec<u128>, // 子ノードのセット
    name: String,            // ノード名
}

struct BlockEntry {
    block_id: u64,          // ブロックID
    data_size: u64,         // このブロック内のデータサイズ
    data_offset: u128,       // データの開始オフセット
    next_block_id: Option<u64>, // 次のブロックのID（複数ブロックにまたがる場合）
}
