use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unsupported file type for: {0}")]
    UnsupportedFileType(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP status error: {0}")]
    Status(StatusCode),
}

/// Read a file from local path or download if it's a URL.
pub fn open_or_download(path_or_url: &str) -> Result<Box<dyn Read>, IoError> {
    if is_url(path_or_url) {
        let file = download(path_or_url)?;
        Ok(Box::new(BufReader::new(file)))
    } else {
        let path = Path::new(path_or_url);
        let file = File::open(path)?;
        Ok(Box::new(BufReader::new(file)))
    }
}

/// Download a file from the URL and return a reader to its content.
/// Handles .gz decompression.
pub fn download(url: &str) -> Result<Box<dyn Read>, IoError> {
    if !is_url(url) {
        return Err(IoError::InvalidUrl(url.to_string()));
    }

    let client = Client::new();
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(IoError::Status(response.status()));
    }

    let mut temp_file = NamedTempFile::new()?;
    let content = response.bytes()?;
    temp_file.write_all(&content)?;
    temp_file.flush()?;

    let path = temp_file.path().to_path_buf();
    let file = File::open(&path)?;

    if path.extension().is_some_and(|ext| ext == "gz") {
        Ok(Box::new(BufReader::new(GzDecoder::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

/// Copy data from a reader to a local path.
pub fn copy_from_reader<R: Read>(mut reader: R, path: &Path) -> Result<(), IoError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    io::copy(&mut reader, &mut file)?;
    Ok(())
}

/// Load content as a string from a file or URL.
pub fn load_str(path_or_url: &str) -> Result<String, IoError> {
    let mut reader = open_or_download(path_or_url)?;
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Save string to a file.
pub fn save_str(path: &Path, data: &str) -> Result<(), IoError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

/// Check if a string is a URL.
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use serde_json::{json, from_str};

    #[test]
    fn test_save_and_load_data() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("data.json");

        let mut data = HashMap::new();
        data.insert("name".to_string(), json!("dalei"));
        data.insert("age".to_string(), json!(18));

        let data_str = serde_json::to_string(&data).expect("Failed to serialize");
        save_str(&file_path, &data_str).expect("Failed to save data");

        let loaded_str = load_str(file_path.to_str().unwrap()).expect("Failed to load string");
        let loaded_data: HashMap<String, serde_json::Value> =
            from_str(&loaded_str).expect("Failed to parse JSON");

        assert_eq!(loaded_data.get("name"), Some(&json!("dalei")));
        assert_eq!(loaded_data.get("age"), Some(&json!(18)));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let path = Path::new("nonexistent_file_123456789.json");
        let result = load_str(path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_copy_from_reader() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("copy.txt");

        let input_data = b"hello world";
        copy_from_reader(&input_data[..], &file_path).expect("Failed to copy");

        let content = fs::read_to_string(&file_path).expect("Failed to read file");
        assert_eq!(content, "hello world");
    }
}
