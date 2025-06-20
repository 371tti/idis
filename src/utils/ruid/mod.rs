pub mod prefix;

use std::fmt::{Debug, Display, Formatter};

use chrono::Utc;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Constants for RUID Fields Format
pub const PREFIX_SHIFT: u8 = 112;
pub const VERSION_CODE_SHIFT: u8 = 108; // 4 bits
pub const VERSION_CODE_MASK: u8 = 0x0F; // 4 bits
pub const DEVICE_ID_SHIFT: u8 = 92;
pub const TIMESTAMP_SHIFT: u8 = 44;
pub const TIMESTAMP_MASK: u64 = 0x0000_FFFF_FFFF_FFFF; // 48 bits
pub const RANDOM_MASK: u64 = 0x0000_0FFF_FFFF_FFFF; // 44 bits
pub const VERSION_CODE: u8 = 0x1; // バージョンコードv1

/// RUID value structure
pub struct RUID {
    val: u128,
}

/// RUID Generator structure
pub struct RUIDGenerator {
    rng: ChaCha20Rng,
    device_id: u16,
}

/// Initialize methods for RUID
impl RUID {
    pub fn new() -> Self {
        RUID { val: 0 }
    }

    /// Set RUID Device ID
    /// This method overrides the device ID in the RUID.
    pub fn set_device_id(&mut self, device_id: u16) {
        self.val |= (device_id as u128) << DEVICE_ID_SHIFT;
    }

    /// Set RUID Prefix
    /// This method overrides the prefix in the RUID.
    pub fn set_prefix(&mut self, prefix: u16) {
        self.val |= (prefix as u128) << PREFIX_SHIFT;
    }

    /// Set RUID Version Code
    /// This method overrides the version code in the RUID.
    
    pub fn set_version(&mut self, version: u8) {
        let masked_version = version & VERSION_CODE_MASK;
        self.val |= (masked_version as u128) << VERSION_CODE_SHIFT;
    }

    /// Set RUID Timestamp
    /// This method overrides the timestamp in the RUID.
    pub fn set_timestamp(&mut self, timestamp: u64) {
        let masked_timestamp = timestamp & TIMESTAMP_MASK;
        self.val |= (masked_timestamp as u128) << TIMESTAMP_SHIFT;
    }

    /// Set RUID Random Value
    /// This method overrides the random value in the RUID.
    pub fn set_random(&mut self, random: u64) {
        let masked_random  = random & RANDOM_MASK;
        self.val |= masked_random as u128
    }
}

/// Get methods for RUID
impl RUID {
    pub fn get_device_id(&self) -> u16 {
        ((self.val >> DEVICE_ID_SHIFT) & 0xFFFF) as u16
    }

    pub fn get_prefix(&self) -> u16 {
        ((self.val >> PREFIX_SHIFT) & 0xFFFF) as u16
    }

    pub fn get_version(&self) -> u8 {
        ((self.val >> VERSION_CODE_SHIFT) & VERSION_CODE_MASK as u128) as u8
    }

    pub fn get_timestamp(&self) -> u64 {
        ((self.val >> TIMESTAMP_SHIFT) & TIMESTAMP_MASK as u128) as u64
    }

    pub fn get_random(&self) -> u64 {
        (self.val & RANDOM_MASK as u128) as u64
    }
    
}

impl From<u128> for RUID {
    fn from(val: u128) -> Self {
        RUID { val }
    }
}

impl From<[u8; 16]> for RUID {
    fn from(bytes: [u8; 16]) -> Self {
        let val = u128::from_le_bytes(bytes);
        RUID { val }
    }
}

impl Into<[u8; 16]> for RUID {
    fn into(self) -> [u8; 16] {
        self.val.to_le_bytes()
    }
}

impl Into<u128> for RUID {
    fn into(self) -> u128 {
        self.val
    }
}

impl Display for RUID {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:04X}_{:01X}_{:04X}_{:012X}_{:011X}",
            self.get_prefix(),
            self.get_version(),
            self.get_device_id(),
            self.get_timestamp(),
            self.get_random()
        )
    }
}

impl Debug for RUID {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "RUID({})", self)
    }
    
}

/// RUIDGenerator methods
impl RUIDGenerator {
    pub fn new() -> Self {
        RUIDGenerator {
            rng: ChaCha20Rng::from_os_rng(),
            device_id: 0,
        }
    }

    pub fn new_with_entropy(entropy: u64) -> Self {
        RUIDGenerator {
            rng: ChaCha20Rng::seed_from_u64(entropy),
            device_id: 0,
        }
    }

    pub fn new_with_device_id(device_id: u16) -> Self {
        RUIDGenerator {
            rng: ChaCha20Rng::from_os_rng(),
            device_id,
        }
    }

    pub fn new_with(entropy: u64, device_id: u16) -> Self {
        RUIDGenerator {
            rng: ChaCha20Rng::seed_from_u64(entropy),
            device_id,
        }
    }

    pub fn set_device_id(&mut self, device_id: u16) {
        self.device_id = device_id;
    }

    pub fn generate(&mut self, prefix: u16) -> RUID {
        let mut ruid = RUID::new();
        ruid.set_prefix(prefix);
        ruid.set_device_id(self.device_id);
        ruid.set_version(VERSION_CODE);
        let timestamp_sec = Utc::now().timestamp();
        ruid.set_timestamp(timestamp_sec as u64);
        let random = self.rng.next_u64();
        ruid.set_random(random);
        ruid
    }
}