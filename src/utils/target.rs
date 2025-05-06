/// 取得したパスのファイルシステムでの1セクタあたりのバイト数を取得する
/// targetによって変わる
/// !! windowsは検証済み
/// !! linuxは未検証

pub mod fs {
    use std::path::Path;
    use std::io;

    /// 指定したパスのファイルシステムでの1セクタあたりのバイト数を取得する
    #[cfg(windows)]
    pub fn get_bytes_per_sector(path: &Path) -> io::Result<u64> {
        use std::io;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::fileapi::GetDiskFreeSpaceW;
        // Path → &str へ変換
        let path_str = path.to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid UTF-8 in path"))?;
        let wide: Vec<u16> = OsStr::new(path_str)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut sectors_per_cluster = 0u32;
        let mut bytes_per_sector = 0u32;
        let mut number_of_free_clusters = 0u32;
        let mut total_number_of_clusters = 0u32;

        let res = unsafe {
            GetDiskFreeSpaceW(
                wide.as_ptr(),
                &mut sectors_per_cluster,
                &mut bytes_per_sector,
                &mut number_of_free_clusters,
                &mut total_number_of_clusters,
            )
        };
        if res == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(bytes_per_sector as u64)
        }
    }

    #[cfg(target_os = "windows")]
    pub async fn open_file_direct(path: &Path) -> io::Result<tokio::fs::File> {
        use winapi::um::winbase::FILE_FLAG_NO_BUFFERING;
        tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            // Windows-specific flag: disable buffering
            .custom_flags(FILE_FLAG_NO_BUFFERING)
            .open(path)
            .await
    }
}


