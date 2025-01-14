use std::io::SeekFrom;

use bytes::buf;
use chrono::offset;
use futures::TryFutureExt;
use tokio::{fs::{File, OpenOptions}, io::{AsyncReadExt, AsyncSeekExt}, sync::mpsc};

use crate::utils::err_set::{self, ErrState};

use super::drive_struct::Header;

struct DiskDriver {
    file: File,
    queue_size: usize,
}

impl DiskDriver {
    pub async fn new(file_path: &str, queue_size: usize) -> Result<Self, ErrState> {
        let mut file = 
        match OpenOptions::new()
            .read(true)
            .write(true)
            .open(file_path)
            .await
            {
                Ok(file) => file,
                Err(e) => {
                    log::error!("DiskDriver Error - Failed to load Disk: {:?}", e);
                    let info_message = match e.kind() {
                        std::io::ErrorKind::NotFound => "Disk not found".to_string(),
                        std::io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
                        std::io::ErrorKind::InvalidInput => "Invalid input".to_string(),
                        std::io::ErrorKind::Interrupted => "Interrupted".to_string(),
                        std::io::ErrorKind::Other => "System drive may be full".to_string(),
                        _ => "Unknown cause".to_string(),
                    };
                    return Err(ErrState::new(
                        err_set::ProsessType::DiskDriver,
                        None).add_message(err_set::ErrMsg::ERROR("Failed to load Disk.".to_string()))
                        .add_message(err_set::ErrMsg::INFO(info_message)))
                }
            };
        Ok(Self { file, queue_size })
    }

    /// ファイルをキューに読み込む
    pub async fn read_to_queue(
        &mut self,
        chunk_size: u64,
        read_size: u64,
        offset: u64,
    ) -> Result<mpsc::Receiver<Vec<u8>>, ErrState> {
        let (tx, rx) = mpsc::channel(self.queue_size);

        // プロデューサータスク
        let mut file = self.file.try_clone().map_err(|e| {
            ErrState::new(err_set::ProsessType::DiskDriver, None)
                .add_message(err_set::ErrMsg::ERROR(format!(
                    "Failed to clone file handle: {:?}",
                    e
                )))
        }).await?;
        let chunk_size = chunk_size as usize;

        tokio::spawn(async move {
            if let Err(e) = file.seek(SeekFrom::Start(offset)).await {
                log::error!("Failed to seek: {:?}", e);
                return;
            }

            let mut total_read = 0u64;
            let mut buffer = vec![0u8; chunk_size];

            while total_read < read_size {
                let to_read = (read_size - total_read).min(chunk_size as u64) as usize;

                match file.read(&mut buffer[..to_read]).await {
                    Ok(0) => break, // EOF
                    Ok(bytes_read) => {
                        total_read += bytes_read as u64;
                        if tx.send(buffer[..bytes_read].to_vec()).await.is_err() {
                            break; // キューが閉じられた場合
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}


struct Driver<'a> {
    /// デスクドライバー
    /// brock_size分
    /// data_bytes = 0 でblock_size分のデータを読み込む
    data_bytes: &'a u64,
    disk_addr: &'a u64,
    ram_addr: &'a u64,
    block_size: &'a u64,
    block_num: &'a u64,
    instruction: Instruction,
}

pub enum Instruction {
    Load,
    Store,
}