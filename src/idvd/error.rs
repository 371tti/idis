use std::fmt::{self, Debug};

pub enum IDVDError {
    VDNotFound,
    OSPermissionDenied,
    FiledGetOsRng,
    InvalidFormat,
    NotSupportedVersion,
    Other(String),
}

impl fmt::Display for IDVDError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IDVDError::VDNotFound => write!(f, "VD not found"),
            IDVDError::OSPermissionDenied => write!(f, "OS permission denied"),
            IDVDError::FiledGetOsRng => write!(f, "Failed to get OS RNG"),
            IDVDError::InvalidFormat => write!(f, "Invalid format"),
            IDVDError::NotSupportedVersion => write!(f, "Not supported version"),
            IDVDError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl Debug for IDVDError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}


impl std::error::Error for IDVDError {
}