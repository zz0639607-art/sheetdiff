//! Local file readers. No reader performs network access.
mod csv;
mod xlsx;
use crate::model::{FileInfo, Options, Workbook};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};
use thiserror::Error;

/// Errors callers can identify without parsing human-readable diagnostics.
#[derive(Debug, Error)]
pub enum ReadError {
    #[error("unsupported file type: {0}; expected .xlsx or .csv")]
    Unsupported(String),
    #[error("sheet not found in either input: {0}")]
    UnknownSheet(String),
}

/// Read one input and attach its SHA256 identity.
pub fn read(path: &Path, options: &Options) -> Result<Workbook> {
    let mut file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let sheets = match extension.as_str() {
        "csv" => csv::read(path, options),
        "xlsx" => xlsx::read(path),
        _ => return Err(ReadError::Unsupported(extension).into()),
    }
    .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(Workbook {
        file: FileInfo {
            path: path.to_string_lossy().into_owned(),
            sha256: format!("{:x}", hasher.finalize()),
        },
        sheets,
    })
}
