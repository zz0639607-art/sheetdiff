//! Stable sheetdiff/v1 serialization without terminal presentation limits.
use crate::model::Report;
use anyhow::Result;

/// Serialize a complete report as indented UTF-8 JSON with a final newline.
pub fn render(report: &Report) -> Result<String> {
    let mut text = serde_json::to_string_pretty(report)?;
    text.push('\n');
    Ok(text)
}
