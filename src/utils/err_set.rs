use std::os::windows::process;

use serde::{Deserialize, Serialize};
use chrono::Utc;
use serde_with::serde_as;
use super::custom_serializers_adapters::TimeStamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrMsg {
    DEBUG(String),
    INFO(String),
    WARN(String),
    ERROR(String),
    CRITICAL(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProsessType {
    LocalApi,
    DB,
    StorageEngine,
    DiskDriver,
    OS,
    Network,
}
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrState {
    pub prosess_type: ProsessType,
    pub message: Vec<ErrMsg>,
    #[serde_as(as = "TimeStamp")]
    pub timestamp: i64,
    pub from: Option<Box<ErrState>>, // 親エラーを保持するフィールドを追加
}

impl ErrState {
    pub fn new(prosess_type: ProsessType, parent: Option<ErrState>) -> Self {
        let utc_timestamp = Utc::now().timestamp_millis();
        let from = if let Some(parent_val) = parent {
            Some(Box::new(parent_val))
        } else { None };
        let message = Vec::new();

        Self {
            prosess_type,
            message,
            timestamp: utc_timestamp,
            from: from,
        }
    }

    pub fn add_message(mut self, msg: ErrMsg) -> Self {
        self.message.push(msg);
        self
    }
}