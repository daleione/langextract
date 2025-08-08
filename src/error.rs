#[derive(Debug)]
pub struct InvalidDatasetError(pub String);

impl std::fmt::Display for InvalidDatasetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InvalidDatasetError: {}", self.0)
    }
}

impl std::error::Error for InvalidDatasetError {}
