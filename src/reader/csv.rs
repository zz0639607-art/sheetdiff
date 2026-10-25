use crate::model::{a1, Cell, Options, Sheet};
use anyhow::Result;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// CSV is a single logical sheet named CSV, regardless of input filename.
pub(super) fn read(path: &Path, options: &Options) -> Result<Vec<Sheet>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(options.delimiter)
        .from_path(path)?;
    let mut sheet = Sheet {
        name: "CSV".into(),
        ..Sheet::default()
    };
    let mut used = BTreeSet::new();
    let mut counts = BTreeMap::<String, usize>::new();
    for (row, record) in reader.records().enumerate() {
        let record = record?;
        if row == 0 && !options.no_header {
            for (col, label) in record.iter().enumerate() {
                let base = if label.is_empty() {
                    a1(0, col).trim_end_matches('1').to_owned()
                } else {
                    label.into()
                };
                let count = counts.entry(base.clone()).or_default();
                *count += 1;
                let mut label = if *count == 1 {
                    base.clone()
                } else {
                    format!("{base}#{count}")
                };
                while used.contains(&label) {
                    *count += 1;
                    label = format!("{base}#{count}");
                }
                used.insert(label.clone());
                sheet.columns.push(label);
            }
        }
        let cells = record
            .iter()
            .enumerate()
            .map(|(col, text)| {
                // Infer finite JSON numbers in data records only; identifiers
                // with leading zeros remain strings. Keep the original display.
                let value = if text.is_empty() {
                    Value::Null
                } else if row > 0 || options.no_header {
                    match serde_json::from_str::<Value>(text) {
                        Ok(value) if value.is_number() => value,
                        _ => Value::String(text.into()),
                    }
                } else {
                    Value::String(text.into())
                };
                let mut cell = Cell::new(value);
                if !text.is_empty() {
                    cell.formatted = text.into();
                }
                (col, cell)
            })
            .collect();
        sheet.rows.insert(row, cells);
    }
    Ok(vec![sheet])
}
