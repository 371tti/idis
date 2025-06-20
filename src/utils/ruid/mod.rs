use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub struct RUID {
    val: [u8; 16],
}

pub struct RUIDGenerator {
    rng: ChaCha20Rng,
}

impl RUID {
    pub fn new() -> Self {
        RUID { val: [0; 16] }
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
        self.rng.fill(&mut ruid.val);
        ruid
    }
}