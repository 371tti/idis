use idis::{cache, idvd::{allocator::FreeMap, cache::Cash}};
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


#[tokio::main]
async fn main() {

    // キャッシュ用ファイルとブロックサイズ
    let path = Path::new("./lol.bin");
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
    let cache = Cash::new(path, block_size).await;
    let mut cach = match cache {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating Cash: {}", e);
            return;
        }
        
    };

    // 読み込み用バッファ
    let mut buffer = vec![0u8; block_size as usize];

    // 1 回目の読み込み（ファイルから）
    let t1 = Instant::now();
    cach.read(&mut buffer, 0).await.unwrap();
    let d1 = t1.elapsed();
    println!("First read took: {:?}", d1);

    // 2 回目の読み込み（キャッシュから）
    let t2 = Instant::now();
    cach.read(&mut buffer, 0).await.unwrap();
    let d2 = t2.elapsed();
    println!("Second read took: {:?}", d2);

    // 文字列を1回だけ書き込むテスト
    let mut data = tokio::fs::read("buf.txt").await.unwrap();
    cach.write(&mut data, 10000).await.unwrap();
    cach.sync().await.unwrap();

    // 書き込んだ内容を読み込んで確認
    let mut buf = vec![0u8; data.len()];
    cach.read(&mut buf, 10002).await.unwrap();
    println!("Read data: {:?}", String::from_utf8_lossy(&buf));
}