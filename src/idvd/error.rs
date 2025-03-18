pub enum IDVDError {
    VDNotFound,
    OSPermissionDenied,
    InvalidFormat,
    NotSupportedVersion,
    Other(String),
}