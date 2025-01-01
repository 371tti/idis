

use std::{any::Any, collections::VecDeque, result};

use mongodb::bson::{self, doc, Bson, Document, bson};
use serde::de::value;

use crate::db::query::{FeatureQuery, Index, QueryType};


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


pub fn feature_query_to_mongo_while(query: &FeatureQuery) -> Document {
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
                                    bson::Bson::Int32(num) => num.to_string(),
                                    bson::Bson::Int64(num) => num.to_string(),
                                    bson::Bson::String(s) => s,
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
fn main() {

    // テスト1: MatchNumクエリ
    let query = FeatureQuery::MatchNum(10);
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! { "$eq": 10 };
    println!("Test MatchNum: {:?}", result);
    assert_eq!(result, expected);

    // テスト2: Lessクエリ
    let query = FeatureQuery::Less(50);
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! { "$lte": 50 };
    println!("Test Less: {:?}", result);
    assert_eq!(result, expected);

    // テスト3: Greaterクエリ
    let query = FeatureQuery::Greater(20);
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! { "$gte": 20 };
    println!("Test Greater: {:?}", result);
    assert_eq!(result, expected);

    // テスト4: ANDクエリ
    let query = FeatureQuery::And(vec![
        FeatureQuery::MatchNum(10),
        FeatureQuery::Less(50),
        FeatureQuery::Greater(20),
    ]);
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! {
        "$and": [
            { "$gte": 20 },
            { "$lte": 50 },
            { "$eq": 10 },
        ]
    };
    println!("Test AND: {:?}", result);

    // テスト5: ORクエリ
    let query = FeatureQuery::Or(vec![
        FeatureQuery::MatchNum(5),
        FeatureQuery::MatchNum(10),
    ]);
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! {
        "$or": [
            { "$eq": 5 },
            { "$eq": 10 }
        ]
    };
    println!("Test OR: {:?}", result);

    // テスト6: NOTクエリ
    let query = FeatureQuery::Not(Box::new(FeatureQuery::MatchNum(10)));
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! {
        "$not": { "$eq": 10 }
    };
    println!("Test NOT: {:?}", result);

    // テスト8: Nestedクエリ
    let query = FeatureQuery::Nested("field".to_string(),
        Box::new(FeatureQuery::MatchNum(10)),
    );
    let result = feature_query_to_mongo_while(&query);
    let expected = doc! {
        "field": { "$eq": 10 }
    };
    println!("Test Nested: {:?}", result);

    println!("All tests passed successfully!");
}
