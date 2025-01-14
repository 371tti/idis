// ------------------------------------
// DriveHeader
// unit: byte
// ------------------------------------

pub struct Header {
    magic: [u8; 4],         // Magic number to identify the drive
    version: u32,           // Version of the drive format
    compression: u16,       // Compression algorithm used
    crypt_method: u16,      // Encryption method used
    nb_snapshots: u32,      // Number of snapshots in the drive
    total_size: u64,       // Total size of the drive in bytes
    meta_offset: u64,      // Offset to the meta (index) section in bytes
    snapshot_offset: u64,  // Offset to the snapshot section in bytes
    index_offset: u64,     // Offset to the index section in bytes
    free_block_manager_offset: u64, // Offset to the free block manager section in bytes
    block_size: u64,        // Block size in bytes
    entry_count: u64,       // Number of entries in the drive
}

impl Header {
    pub const SIZE: usize = 72; // 72 bytes
}

pub enum CompressionMethod {
    None = 0,
    Zstd = 1,
}

pub enum CryptMethod {
    None = 0,
    Aes256 = 1,
}

pub struct Snapshot {
    id : u128,               // Snapshot ID
    size: u64,               // Size of the snapshot in bytes
    versions: Vec<u128>,      // List of back drive ids
}
// 各バージョンのidのリスト

pub struct Metadata {
    id: u128,                    // ノードID 16byte
    size: u64,         // サイズ 8byte
    date_created: u64,          // 作成日時 8byte
    date_modified: u64,         // 更新日時 8byte
    date_accessed: u64,         // 最終アクセス日時 8byte
    parent_offset: u64,             // 親ノードのオフセット 8byte
    data_offset: u64,           // データの開始オフセット 8byte
    data_size: u64,             // データのサイズ 8byte
    child_count: u64,           // 子ノードの数 8byte
    perm_num: u32,              // 権限の数 2byte
    children_num: u32,          // 子ノードの数 2byte
    name: [u8; 256],            // ノード名 256byte
    perm: Vec<u128>,               // ノードの権限 16n-byte
    children_offsets: Vec<u64>, // 子ノードのオフセットセット 8m-byte
    child_snapshot_version: Vec<u32>, // 子ノードのスナップショットバージョン 4m-byte
}
// メタデータはブロックに1つ配置していく 容量が足りなくなってきたら重なる 細断化しにくいように余裕を持たせる 必ず連続データになるようにする

pub struct DataBlockList {
    id: u128,
    size: u64,
    blocks: Vec<Block>,
}

pub struct Index {
    id: u128,
    size: u64,
    index_offset: Vec<u64>,
    indexes: Vec<IndexField>,
    types: Vec<IndexType>,
}

pub struct IndexData<T> {
    id: u128,
    size: u64,
    key: Vec<T>,
    addr: Vec<u64>,
}


pub enum IndexField {
    Id,
    Size,
    DataCreated,
    DateModified,
    DateAccessed,
    ParentOffset,
    DataOffset,
    DataSize,
    ChildCount,
    PermNum,
    ChildrenNum,
    Name,
}

pub enum IndexType {
    HashMap,
    BTree,
}

// 空き領域管理
struct FreeBlockManager {
    id: u128,
    size: u64,
    free_blocks: Vec<Block>,
}

struct Block {
    start: u64, // 開始位置
    length: u64, // 長さ（ブロック数）
}

/*
memo
メタデータは細断化させないようにする
メタデータはrootからのtree構造を持つ
また探索高速化のため メタデータ構造体には探索に必要なすべての情報を持たせつつサイズを小さくする
データは細断化を許可
これはデフラグ時も適応される仕様
ドライブ使用量が75%を超えない場合
連続すべきデータはその長さの倍の容量をもつ連続した領域に配置する
ドライブ使用量が75%を超える場合
メタデータ以外の細断化を許容し容量を節約する
空きブロックの管理は階層的管理で行う(Btree)
スナップショットはホットデータを連続すべき単位で全コピーする
back driveは圧縮したり

+-----------------------------------+
|           Drive Header            |
+-----------------------------------+
|        Snapshot Section           |
|  - Snapshot Metadata              |
|  - Snapshot Data                  |
+-----------------------------------+
|         Metadata Section          |
|  - Metadata Blocks (Tree)         |
+-----------------------------------+
|         Index Section             |
|  - BTree/HashMap for Keys         |
+-----------------------------------+
|    Free Block Manager Section     |
|  - BTree for Free Block Ranges    |
+-----------------------------------+
|           Data Section            |
|  - Actual File/Data Content       |
+-----------------------------------+

*/