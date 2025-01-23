use std::io::SeekFrom;

use futures::TryFutureExt;
use tokio::{fs::{File, OpenOptions}, io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt}, sync::mpsc};

use crate::utils::err_set::{self, ErrState};


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

    pub fn get_queue_size(&self) -> usize {
        self.queue_size
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
            log::error!("DiskDriver Error - Failed to clone file: {:?}", e);
            ErrState::new(err_set::ProsessType::DiskDriver, None)
                .add_message(err_set::ErrMsg::ERROR("Failed to clone file.".to_string()))
        }).await?;
        let chunk_size = chunk_size as usize;

        tokio::spawn(async move {
            if let Err(e) = file.seek(SeekFrom::Start(offset)).await {
                log::error!("Failed to seek: {:?}", e);
                return ;
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

    pub async fn write_from_queue(
        &mut self,
        mut rx: mpsc::Receiver<Vec<u8>>,
        offset: u64,
    ) -> Result<(), ErrState> {
        // ファイルのクローンを作成
        let mut file = self.file.try_clone().map_err(|e| {
            log::error!("DiskDriver Error - Failed to clone file: {:?}", e);
            ErrState::new(err_set::ProsessType::DiskDriver, None)
                .add_message(err_set::ErrMsg::ERROR("Failed to clone file struct.".to_string()))
        }).await?;
    
        // 書き込み位置をシーク
        file.seek(SeekFrom::Start(offset)).await.map_err(|e| {
            log::error!("Failed to seek during write: {:?}", e);
            ErrState::new(err_set::ProsessType::DiskDriver, None)
                .add_message(err_set::ErrMsg::ERROR("Failed to seek during write.".to_string()))
        })?;
    
        // コンシューマータスクを生成
        tokio::spawn(async move {
            while let Some(chunk) = rx.recv().await {
                // チャンクをファイルに書き込む
                if let Err(e) = file.write_all(&chunk).await {
                    log::error!("Failed to write chunk to file: {:?}", e);
                    break;
                }
            }
    
            // ファイルをフラッシュ
            if let Err(e) = file.flush().await {
                log::error!("Failed to flush file: {:?}", e);
            }
        });
    
        Ok(())
    }
    

}