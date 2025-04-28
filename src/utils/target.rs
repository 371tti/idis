/// 取得したパスのファイルシステムでの1セクタあたりのバイト数を取得する
/// targetによって変わる
/// !! windowsは検証済み
/// !! linuxは未検証
use std::path::Path;
#[cfg(windows)]
use std::io;
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use winapi::um::fileapi::GetDiskFreeSpaceW;
#[cfg(windows)]
use winapi::shared::minwindef::DWORD;

/// 指定したパスのファイルシステムでの1セクタあたりのバイト数を取得する
#[cfg(windows)]
pub fn get_bytes_per_sector(path: &Path) -> io::Result<DWORD> {
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
        Ok(bytes_per_sector)
    }
}

#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use libc::{statvfs, c_char};

#[cfg(unix)]
pub fn get_bytes_per_sector(path: &str) -> io::Result<u64> {
    let c_path = CString::new(path).unwrap();
    let mut stat: statvfs = unsafe { std::mem::zeroed() };
    let res = unsafe { statvfs(c_path.as_ptr() as *const c_char, &mut stat) };
    if res != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(stat.f_frsize as u64)
    }
}