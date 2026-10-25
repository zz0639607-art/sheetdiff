//! Offline cell-level spreadsheet comparison, reusable independently of the CLI.
pub mod cli;
pub mod diff;
pub mod model;
pub mod reader;
pub mod render;
use anyhow::Result;
use model::{Options, Report};
use std::path::Path;

/// Read two local files and compare them under a shared policy.
pub fn compare_files(old: &Path, new: &Path, options: &Options) -> Result<Report> {
    anyhow::ensure!(
        options.delimiter.is_ascii() && !matches!(options.delimiter, 0 | b'\n' | b'\r' | b'"'),
        "delimiter must be an ASCII character other than NUL, quote or newline"
    );
    anyhow::ensure!(
        options.epsilon.is_finite() && options.epsilon >= 0.0,
        "epsilon must be finite and nonnegative"
    );
    let old = reader::read(old, options)?;
    let new = reader::read(new, options)?;
    for name in &options.sheets {
        if !old
            .sheets
            .iter()
            .chain(&new.sheets)
            .any(|s| &s.name == name)
        {
            return Err(reader::ReadError::UnknownSheet(name.clone()).into());
        }
    }
    Ok(diff::compare(&old, &new, options))
}
