# idis KV-DBの実装

## 必要な機能
- Key(RUID)に対するValue(Bin)の読み書き
- トランザクション単位で一斉処理
- CoWの実装
- スナップショット
- key と val で vhdの分割
## プロジェクトフォルダTree
```
kv_store/
├── mod.rs
├── cache/
│   ├── mod.rs
│   ├── key.rs
│   └── entry.rs
├── kv/
│   ├── mod.rs
│   ├── vhd/
│   │   ├── mod.rs
│   │   ├── key_vhd.rs
│   │   ├── val.vhd.rs
│   │   ├── key_alloc.rs
│   │   ├── val_alloc.rs
│   └── block.rs
```
## 試案
key と val を分割
key はhashmap で データ本体はval側に

keyの整合性管理 書き込み中の電源遮断管理
keyに対する変更にたいしwalを作る

val はブロック単位 サイズは 1MB 4MB 16MB
