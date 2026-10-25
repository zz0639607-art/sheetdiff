//! Command-line syntax and policy validation.
use crate::model::Options;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Compare Excel (.xlsx) or CSV files, entirely offline.
#[derive(Debug, Parser)]
#[command(
    version,
    about,
    after_help = "Exit codes: 0 identical, 1 different, 2 error."
)]
pub struct Cli {
    pub old: PathBuf,
    pub new: PathBuf,
    /// Compare only this sheet (repeatable).
    #[arg(long = "sheet")]
    pub sheets: Vec<String>,
    /// Ignore formatting and merge-layout differences.
    #[arg(long)]
    pub skip_format: bool,
    /// Ignore formula text changes (cached values are still compared).
    #[arg(long)]
    pub skip_formula: bool,
    /// Absolute tolerance for numeric comparisons.
    #[arg(long, default_value = "1e-9", value_parser = parse_epsilon)]
    pub epsilon: f64,
    /// Treat the first CSV record as data.
    #[arg(long)]
    pub no_header: bool,
    /// One ASCII delimiter character.
    #[arg(long, default_value = ",", value_parser = parse_delimiter)]
    pub delimiter: u8,
    #[arg(long, value_enum, default_value = "auto")]
    pub color: Color,
    /// Number of unchanged rows shown around changes.
    #[arg(long, default_value_t = 2)]
    pub context: usize,
    /// Save a self-contained, offline HTML report.
    #[arg(long)]
    pub html: Option<PathBuf>,
    /// Save a sheetdiff/v1 JSON report.
    #[arg(long)]
    pub json: Option<PathBuf>,
    /// Print only the summary in the terminal.
    #[arg(long)]
    pub summary_only: bool,
}

/// ANSI color policy.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Color {
    Always,
    Never,
    Auto,
}
impl Cli {
    /// Translate CLI syntax to the reusable library's options.
    pub fn options(&self) -> Options {
        Options {
            sheets: self.sheets.clone(),
            skip_format: self.skip_format,
            skip_formula: self.skip_formula,
            epsilon: self.epsilon,
            no_header: self.no_header,
            delimiter: self.delimiter,
            context: self.context,
        }
    }
}
fn parse_epsilon(s: &str) -> Result<f64, String> {
    let value: f64 = s
        .parse()
        .map_err(|_| "epsilon must be a finite nonnegative number")?;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err("epsilon must be a finite nonnegative number".into())
    }
}
fn parse_delimiter(s: &str) -> Result<u8, String> {
    if s.len() == 1 && !matches!(s.as_bytes()[0], 0 | b'\n' | b'\r' | b'"') {
        Ok(s.as_bytes()[0])
    } else {
        Err("delimiter must be one non-NUL ASCII character other than quote or newline".into())
    }
}
