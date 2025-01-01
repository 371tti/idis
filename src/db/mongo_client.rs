use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value};
use ruid_set::ruid::Ruid;
use mongodb::{options::ClientOptions, Client};
use mongodb::bson::{self, bson, doc, Bson, Document};

use std::str::FromStr;

// use super::db_trait::Database;
use super::query::{self, FeatureQuery, Index, InsertQuery, LocationQuery, QueryType};

pub struct MongoDB {
    instance: mongodb::Database,
}

impl MongoDB {
    // pub fn to_mongo_query(query: &QueryType) -> bson::Document {
    //     match query {
    //         QueryType::None => doc! {},
    //         QueryType::Set(r, d, loc_query) => Self::set_query_builder(loc_query),
    //         QueryType::Add(r, d, insert_query) => Self::add_query_builder(insert_query),
    //         QueryType::Del(r, d, loc_query) => Self::del_query_builder(loc_query),
    //         QueryType::Get(r, d, loc_query) => Self::get_query_builder(loc_query),
    //         QueryType::DelMany(r, feat_query) => Self::del_many_query_builder(feat_query),
    //         QueryType::Find(r, feat_query) => Self::find_query_builder(feat_query),
    //         QueryType::List(r) => Self::list_query_builder(r),
    //     }
    // }

    // fn set_query_builder(collection_id: &Ruid, docment_id: &Ruid, q: &LocationQuery, set_docment: bson::Document) -> bson::Document {
    //     let mut query = q.clone();

    //     while  {
            
    //     }
    // }

    // fn add_query_builder(q: &InsertQuery) -> bson::Document {

    // }

    // fn del_query_builder(q: &LocationQuery) -> bson::Document {

    // }

    // fn get_query_builder(q: &LocationQuery) -> bson::Document {

    // }

    // fn del_many_query_builder(q: &FeatureQuery) -> bson::Document {

    // }


    
    
}

