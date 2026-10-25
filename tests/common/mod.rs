use rust_xlsxwriter::Workbook;
use std::{fs, path::Path};

/// Generate deterministic synthetic source data; no external documents required.
pub fn basic_xlsx(path: &Path, value: f64) {
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.set_name("预算表").unwrap();
    sheet.write_string(0, 0, "项目").unwrap();
    sheet.write_string(0, 1, "预算").unwrap();
    sheet.write_string(1, 0, "研发费用").unwrap();
    sheet.write_number(1, 1, value).unwrap();
    book.save(path).unwrap();
}
pub fn csv(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}
