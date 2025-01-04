
// ------------------------------------
// DriveHeader
// unit: byte
// ------------------------------------
#[repr(C)] // Ensure memory layout is consistent
struct DriveHeader {
    version: u32,           // Version of the drive format
    total_size: u128,       // Total size of the drive in bytes
    meta_offset: u128,      // Offset to the meta (index) section in bytes
    data_offset: u128,      // Offset to the data section in bytes
    block_size: u64,        // Block size in bytes
    entry_count: u64,       // Number of entries in the drive
}
