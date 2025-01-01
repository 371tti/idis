use ruid_set::ruid::Ruid;



/// Indexの種類
#[derive(Debug, Clone)]
pub enum Index {
    Number(i32),
    String(String),
}


/// データの場所を指定するクエリ
#[derive(Debug)]
pub enum LocationQuery {
    All,                             // 条件なし（全データ対象） (object, list)
    This,                            // 現在の場所を対象
    // ネストの進行
    Slice(i32, i32, Box<LocationQuery>),    // 範囲内のリスト要素を対象 (-1, 0 は先頭、0, -1 は末尾) (list)
    Index(i32, Box<LocationQuery>),         // 指定されたインデックスの要素を対象 (list)
    IndexBack(i32, Box<LocationQuery>),     // 末尾から指定されたインデックスの要素を対象 (list)
    Nested(Index, Box<LocationQuery>),      // ネストされたフィールド内の指定場所を対象 (object, list)
    Skip(Box<FeatureQuery>, Box<LocationQuery>), // 現在のスコープをスキップして対象を絞る (object, list)
}


/// データの挿入場所を指定するクエリ
#[derive(Debug)]
pub enum InsertQuery {
    AtHead(i32),                     // リストの先頭に挿入 0 は先頭 [<0>X, <1>X, ...] (list)
    AtBack(i32),                     // リストの末尾に挿入 0 は末尾 [...X<1>, X<0>] (list)
    Push,                            // リストの末尾に挿入もしくは単に挿入 (list, object)
    // ネストの進行
    Slice(i32, i32, Box<InsertQuery>),                 // リストの値が範囲内にある (-1, 0 は先頭, 0, -1 は末尾) (list)
    Index(i32, Box<InsertQuery>),                      // インデックスで指定された場所 (list)
    IndexBack(i32, Box<InsertQuery>),                  // 末尾からのインデックスで指定された場所 (list)
    Nested(Index, Box<InsertQuery>), // ネストされたフィールドに対するクエリ (object, list)
    Skip(Box<FeatureQuery>, Box<InsertQuery>),  // 現在のスコープをスキップして対象を絞る (object, list)
}

/// データの特徴を指定するクエリ
#[derive(Debug, Clone)]
pub enum FeatureQuery {
    Any,                            // なにかデータがあるとき
    None,                           // データがないとき
    Less(i32),                        // 数値が指定された値以下 (number)
    Greater(i32),                     // 数値が指定された値以上 (number)
    MatchNum(i32),                    // 値が一致 (number)
    MatchStr(String),           // 値が一致 (String)
    MatchBool(bool),             // 値が一致 (bool)
    // ネストの進行
    Index(i32, Box<FeatureQuery>),                      // インデックスで指定された場所 (list)
    Nested(String, Box<FeatureQuery>), // ネストされたフィールドの特徴 (object, list)
    // 論理操作
    And(Vec<FeatureQuery>),          // AND条件 (object, list)
    Or(Vec<FeatureQuery>),           // OR条件 (object, list)
    Not(Box<FeatureQuery>),          // NOT条件 (object, list)
}

/// 操作の種類
#[derive(Debug)]
pub enum QueryType {
    None,                        // 操作なし (object, list)
    Set(Box<Ruid>, Box<Ruid>, LocationQuery),  // 指定された場所に値を設定 (object, list)
    Add(Box<Ruid>, Box<Ruid>, InsertQuery),    // 指定された場所に値を挿入 (object, list)
    Del(Box<Ruid>, Box<Ruid>, LocationQuery),          // 指定された場所のデータを削除 (object, list)
    Get(Box<Ruid>, Box<Ruid>, LocationQuery),          // 指定された場所のデータを取得 (object, list)
    DelMany(Box<Ruid>, FeatureQuery),          // 指定された特徴を持つデータを削除 (object, list)
    Find(Box<Ruid>, FeatureQuery),          // 指定された特徴を持つデータを検索 (object, list)
    List(Box<Ruid>),                        // 全データをリスト取得 (object, list)
}

pub mod generator {
    use mongodb::bson::{bson, Bson, Document};

    use super::FeatureQuery;
    
            
    /// MongoDBクエリ変換時の"演算子"を表す
    #[derive(Debug)]
    enum StackInstruction {
        Extend,
        MargeKey,
        MargeObject,
    }

    /// スタックに積む1フレーム（1段）の情報をまとめた構造体
    #[derive(Debug)]
    pub struct StackFrame<T> {
        instruction: StackInstruction,
        result: Bson,
        query: Vec<T>
    }

    impl<T> StackFrame<T> {
        pub fn new(query: Vec<T>) -> Self {
            StackFrame {
                instruction: StackInstruction::Extend,
                result: Bson::Null,
                query: query,
            }
        }
    }


    pub fn feature_query_to_mongo(query: &FeatureQuery) -> Document {
        let mut stack: Vec<StackFrame<FeatureQuery>> = Vec::new();

        stack.push(
            StackFrame::<FeatureQuery>::new(vec![query.clone()])
        );

        while let Some(mut frame) = stack.pop() {
            match frame.instruction {
                StackInstruction::Extend => {
                    // 元フレームからクエリを取り出す
                    match frame.query.pop() { 
                        // queryが存在 展開処理
                        Some(coquery) => {
                            match coquery {
                                FeatureQuery::Any => {
                                    frame.result = bson!({ "$exists": true });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::None => {
                                    frame.result = bson!({ "$exists": false });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::Less(val) => {
                                    frame.result = bson!({ "$lte": val });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::Greater(val) => {
                                    frame.result = bson!({ "$gte": val });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::MatchNum(val) => {
                                    frame.result = bson!({ "$eq": val });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::MatchStr(val) => {
                                    frame.result = bson!({ "$eq": val });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::MatchBool(val) => {
                                    frame.result = bson!({ "$eq": val });
                                    frame.instruction = StackInstruction::MargeObject;
                                    stack.push(frame);// これ以上展開しない
                                },
                                FeatureQuery::Index(index, feature_query) => {
                                    frame.result = bson!(index);
                                    frame.instruction = StackInstruction::MargeKey;
                                    stack.push(frame);// 元フレームをスタックに積む
                                    let second_frame = StackFrame::<FeatureQuery>::new(vec![*feature_query]);
                                    stack.push(second_frame);// 展開フレームをスタックに積む
                                },
                                FeatureQuery::Nested(index, feature_query) => {
                                    frame.result = bson!(index);
                                    frame.instruction = StackInstruction::MargeKey;
                                    stack.push(frame);// 元フレームをスタックに積む
                                    let second_frame = StackFrame::<FeatureQuery>::new(vec![*feature_query]);
                                    stack.push(second_frame);// 展開フレームをスタックに積む
                                },
                                FeatureQuery::And(mut vec) => {
                                    frame.result = bson!({ "$and": [] });
                                    let covec = vec.pop().unwrap();
                                    frame.query = vec;
                                    frame.instruction = StackInstruction::Extend;
                                    stack.push(frame); // 元フレームをスタックに積む
                                    let second_frame = StackFrame::<FeatureQuery>::new(vec![covec]);
                                    stack.push(second_frame);// 展開フレームをスタックに積む
                                },
                                FeatureQuery::Or(mut vec) => {
                                    frame.result = bson!({ "$or": [] });
                                    let covec = vec.pop().unwrap();
                                    frame.query = vec;
                                    frame.instruction = StackInstruction::Extend;
                                    stack.push(frame); // 元フレームをスタックに積む
                                    let second_frame = StackFrame::<FeatureQuery>::new(vec![covec]);
                                    stack.push(second_frame);// 展開フレームをスタックに積む
                                },
                                FeatureQuery::Not(feature_query) => {
                                    frame.result = bson!({ "$not": Bson::Null });
                                    frame.instruction = StackInstruction::Extend;
                                    stack.push(frame); // 元フレームをスタックに積む
                                    let second_frame = StackFrame::<FeatureQuery>::new(vec![*feature_query]);
                                    stack.push(second_frame);// 展開フレームをスタックに積む
                                },
                            }
                        },
                        None =>todo!(), // ここには来ない
                    }
                    
                },
                StackInstruction::MargeKey => todo!(), // ここには来ない,
                StackInstruction::MargeObject => {
                    match stack.last_mut() {
                        Some(front_frame) => {
                            match front_frame.instruction {
                                StackInstruction::Extend => {
                                    let marge_object = frame.result;
                                    let marged_object = front_frame.result.clone();
                                    if let Some((key, value)) = marged_object.as_document().unwrap().iter().next() {
                                        let new_bson = if let Some(array) = value.clone().as_array_mut() {
                                            // 配列の場合: 要素を追加
                                            array.push(marge_object);
                                            bson!({ key: array })
                                        } else {
                                            // 配列でない場合: ドキュメントとしてそのまま突っ込む
                                            bson!({ key: marge_object })
                                        };
                                    
                                        // 結果を更新
                                        front_frame.result = new_bson;
                                    
                                        // 次のクエリが存在する場合はスタックに追加
                                        match front_frame.query.pop() {
                                            Some(coquery) => {
                                                let second_frame = StackFrame::<FeatureQuery>::new(vec![coquery]);
                                                stack.push(second_frame);
                                            },
                                            None => {
                                                // 次のクエリがない場合はMargeObjectに設定
                                                front_frame.instruction = StackInstruction::MargeObject;
                                            }
                                        }
                                    }
                                    
                                },
                                StackInstruction::MargeKey => {
                                    let marge_object = frame.result;
                                    let marged_key = front_frame.result.clone();
                                    let marged_key_str = match marged_key {
                                        Bson::Int32(num) => num.to_string(),
                                        Bson::Int64(num) => num.to_string(),
                                        Bson::String(s) => s,
                                        _ => todo!(), // ここには来ない,
                                    };
                                    let new_bson = bson!({ marged_key_str: marge_object });
                                    front_frame.instruction = StackInstruction::MargeObject;
                                    front_frame.result = new_bson;
                                },

                                StackInstruction::MargeObject => todo!(), // ここには来ない,
                            }
                        },
                        None => {
                            stack.push(frame);
                            break;
                        },
                    }
                },
            }
            println!("stack: {:?}", stack);
        }


        stack.last().unwrap().result.as_document().unwrap().clone()
    }
}