pub mod prefix;

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub const PREFIX_SHIFT: u8 = 112;
pub const VERSION_CODE_SHIFT: u8 = 108; // 4 bits
pub const VERSION_CODE_MASK: u8 = 0x0F; // 4 bits
pub const DEVICE_ID_SHIFT: u8 = 92;
pub const TIMESTAMP_SHIFT: u8 = 44;
pub const TIMESTAMP_MASK: u64 = 0x0000_FFFF_FFFF_FFFF; // 48 bits
pub const RANDOM_MASK: u64 = 0x0000_0FFF_FFFF_FFFF; // 44 bits
pub const VERSION_CODE: u8 = 0x1; // バージョンコードv1

pub struct RUID {
    val: u128,
}

pub struct RUIDGenerator {
    rng: ChaCha20Rng,
    device_id: u16,
}

impl RUID {
    pub fn new() -> Self {
        RUID { val: 0 }
    }

    pub fn set_device_id(&mut self, device_id: u16) {
        self.val |= (device_id as u128) << DEVICE_ID_SHIFT;
    }

    pub fn set_prefix(&mut self, prefix: u16) {
        self.val |= (prefix as u128) << PREFIX_SHIFT;
    }

    pub fn set_version(&mut self, version: u8) {
        let masked_version = version & VERSION_CODE_MASK;
        self.val |= (masked_version as u128) << VERSION_CODE_SHIFT;
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        let masked_timestamp = timestamp & TIMESTAMP_MASK;
        self.val |= (masked_timestamp as u128) << TIMESTAMP_SHIFT;
    }

    pub fn set_rndom(&mut self, random: u64) {
        let masked_random  = random & RANDOM_MASK;
        self.val |= masked_random as u128
    }
}

impl From<[u8; 16]> for RUID {
    fn from(val: [u8; 16]) -> Self {
        RUID { val }
    }
}

impl RUIDGenerator {
    pub fn new() -> Self {
        RUIDGenerator {
            rng: ChaCha20Rng::from_os_rng(),
            device_id: 0,
        }
    }

    pub fn new_with_device_id(device_id: u16) -> Self {
        RUIDGenerator {
            rng: ChaCha20Rng::from_os_rng(),
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
        ruid.set_timestamp(self.rng.gen());
        ruid.set_rndom(self.rng.gen());
        ruid
    }

    fn id_builder(
        device_id: &[u8; 2], 
        prefix: &[u8;2], 
        version: u8, 
        timestamp: u64, 
        random: &[u8]
    ) -> RUID {
        let mut ruid = RUID::new();
    }
}