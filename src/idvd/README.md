# idvd構想
idvd(idis virtual drive)
DBのストレージエンジンでコレクション単位に構築する仮想ドライブ
ドライブごとキャッシュにおいてon ram 扱いのデータとする
Cowするか? no
walする
wal領域をドライブに作成する？
walの制御アルゴリズム
巨大バイナリを扱う際walをパススルーすべき
どうやって？

# idvd構造
idvd [ meta | data | bitmap ]  
::data { cluster_map, fs, binary }  

メタデータは先頭に配置initialize最適化
bitmapは後方配置ドライブサイズの拡張時最適化
bitmapはpow_mapというサイズ変更時全体構成が変わるものをつかうためサイズ拡張時は完全に作り直される
data部分にはcluster_mapとfsと実際のデータ-binaryを含む

## cluster_mapの構造
block IDはruidであらわす  
ruid to cluster_map_pos  
のindexと実cluster_mapを保持  

ruid to cluster_mapはソート済みであるべき  

このレイヤーでCOWを実装すべきかも  

und_pos [ruid -> cluster_map_pos]  
cluster_pos [start_pos, block_offset]  

## walについて
idisではtonドキュメントを操作するので比較的柔軟かつ単純にできる
del -> padding型に入れ替え(boolは既存使用では無理)(定期的に断片化を解決)
new -> 単に領域をアロケートしてtonを配置
add -> シーク位置による 再アロケートするか断片化させて拡張(アルゴリズムを試案すべき)

tonでは内部でpadding型をつかいアロケーションを最適化できる
巨大サイズのtonもハイパフォーマンスであつかえるよう配慮すべき

## ただのバイナリについて
トランザクションが不明なので基本全体の複製(old と new の2つ) を持つべき
破損フラグを使い整合性維持 破損時はnewを削除
