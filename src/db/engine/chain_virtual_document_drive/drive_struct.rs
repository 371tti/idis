
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
    total_size: u64,       // Total size of the drive in bytes
    meta_offset: u64,      // Offset to the meta (index) section in bytes
    snapshot_offset: u64,  // Offset to the snapshot section in bytes
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

#[repr(C)]
struct Snapshot {
    id : u128,               // Snapshot ID
    versions: Vec<u128>,      // List of back drive ids
}
// 各バージョンのidのリスト

#[repr(C)]
struct Metadata {
    id: u128,                    // ノードID 16byte
    size: u64,         // サイズ 8byte
    date_created: u64,          // 作成日時 8byte
    date_modified: u64,         // 更新日時 8byte
    date_accessed: u64,         // 最終アクセス日時 8byte
    parent_offset: u64,             // 親ノードのオフセット 8byte
    data_offset: u64,           // データの開始オフセット 8byte
    data_size: u64,             // データのサイズ 8byte
    child_count: u64,           // 子ノードの数 8byte
    name: [u8; 256],            // ノード名 256byte
    perm: Vec<u128>,               // ノードの権限 16n-byte
    children_offsets: Vec<u64>, // 子ノードのオフセットセット 8m-byte
    child_snapshot_version: Vec<u32>, // 子ノードのスナップショットバージョン 4m-byte
}
// メタデータはブロックに1つ配置していく 容量が足りなくなってきたら重なる 細断化しにくいように余裕を持たせる 必ず連続データになるようにする

#[repr(C)]
struct DataMeta {
    id: u128,                    // ノードID 16byte
    size: u64,         // サイズ 8byte
    jump_list: Vec<(u64, u64)>,           // ジャンプリスト 16n-byte
}

