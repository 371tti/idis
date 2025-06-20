use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub struct RUID {
    val: [u8; 16],
}

pub struct RUIDGenerator {
    rng: ChaCha20Rng,
    device_id: [u8; 2],
}

impl RUID {
    pub fn new() -> Self {
        RUID { val: [0; 16] }
    }

    pub fn set_device_id(&mut self, device_id: u64) {
        self.val[0..8].copy_from_slice(&device_id.to_le_bytes());
    }

    pub fn set_prefix(&mut self, prefix: &[u8; 2]) {
        self.val[0..2].copy_from_slice(prefix);
    }

    pub fn set_version(&mut self, version: u8) {
        self.val[8] = version;
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.val[9..17].copy_from_slice(&timestamp.to_le_bytes());
    }

    pub fn set_rndom(&mut self, random: &[u8]) {
        let len = random.len().min(8);
        self.val[8..8 + len].copy_from_slice(&random[0..len]);
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
        }
    }

    pub fn generate(&mut self) -> RUID {
        let mut ruid = RUID::new();
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