//! Sparse workbook data and the versioned, serializable diff contract.
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Semantic formatting properties, independent of workbook-local style IDs.
pub type Format = BTreeMap<String, String>;

/// A present cell. A null value denotes an explicit blank; absence is `None`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Cell {
    pub value: Value,
    pub formula: Option<String>,
    pub formatted: String,
    pub format: Format,
    /// Original date serial, used to avoid treating a number-format edit as a
    /// value edit. Human/JSON values remain normalized ISO 8601 strings.
    #[serde(skip)]
    pub(crate) date_serial: Option<Value>,
}
impl Cell {
    /// Construct a cell with a normalized display string.
    pub fn new(value: Value) -> Self {
        let formatted = match &value {
            Value::Null => "(empty)".into(),
            Value::String(s) if s.is_empty() => "(empty string)".into(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        Self {
            value,
            formula: None,
            formatted,
            format: Format::new(),
            date_serial: None,
        }
    }
    /// Formula text takes precedence over a cached result for human display.
    pub fn display(&self) -> &str {
        self.formula.as_deref().unwrap_or(&self.formatted)
    }
}

/// A sparse row keyed by zero-based column index.
pub type Row = BTreeMap<usize, Cell>;

/// A worksheet keeps original row indices, including gaps.
#[derive(Clone, Debug, Default)]
pub struct Sheet {
    pub name: String,
    pub rows: BTreeMap<usize, Row>,
    pub columns: Vec<String>,
    pub merged: Vec<String>,
}

/// Input identity uses the hash of the original bytes, independent of options.
#[derive(Clone, Debug, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub sha256: String,
}

/// Reader output shared by the XLSX and CSV backends.
#[derive(Clone, Debug)]
pub struct Workbook {
    pub file: FileInfo,
    pub sheets: Vec<Sheet>,
}

/// Comparison policy. Epsilon is absolute and must be finite and nonnegative.
#[derive(Clone, Debug)]
pub struct Options {
    pub sheets: Vec<String>,
    pub skip_format: bool,
    pub skip_formula: bool,
    pub epsilon: f64,
    pub no_header: bool,
    pub delimiter: u8,
    pub context: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            sheets: vec![],
            skip_format: false,
            skip_formula: false,
            epsilon: 1e-9,
            no_header: false,
            delimiter: b',',
            context: 2,
        }
    }
}

/// Multiple independent changes may be recorded at the same coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Modified,
    FormulaChanged,
    FormatChanged,
    Added,
    Removed,
}

/// Sheet status includes unchanged sheets so consumers can build a full index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SheetStatus {
    Unchanged,
    Modified,
    Added,
    Removed,
}

/// A cell event, retaining coordinates on both sides of a row insertion.
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub cell: String,
    pub old_cell: Option<String>,
    pub new_cell: Option<String>,
    pub column_name: Option<String>,
    pub kind: ChangeKind,
    pub old: Option<Cell>,
    pub new: Option<Cell>,
    pub row_change: bool,
}

/// An inserted/deleted row is rendered once as a block.
#[derive(Clone, Debug, Serialize)]
pub struct RowChange {
    pub row: usize,
    pub kind: ChangeKind,
    pub cells: Row,
}

/// An unchanged row adjacent to a change; row numbers are one-based.
#[derive(Clone, Debug, Serialize)]
pub struct ContextRow {
    pub old_row: usize,
    pub new_row: usize,
    pub cells: Row,
}

/// All events and display metadata for one named sheet.
#[derive(Clone, Debug, Serialize)]
pub struct SheetDiff {
    pub name: String,
    pub status: SheetStatus,
    pub changes: Vec<Change>,
    pub row_changes: Vec<RowChange>,
    pub context: Vec<ContextRow>,
    pub merged_old: Vec<String>,
    pub merged_new: Vec<String>,
}

/// Counts include cells in added/removed rows and sheets.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub sheets_changed: usize,
    pub cells_modified: usize,
    pub cells_added: usize,
    pub cells_removed: usize,
    pub formulas_changed: usize,
    pub formats_changed: usize,
    pub rows_added: usize,
    pub rows_removed: usize,
    pub sheets_added: usize,
    pub sheets_removed: usize,
}

/// Stable top-level report; new v1 fields are additive.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub old: FileInfo,
    pub new: FileInfo,
    pub summary: Summary,
    pub sheets: Vec<SheetDiff>,
    pub warnings: Vec<String>,
}
impl Report {
    /// True when the selected sheets differ under the requested policy.
    pub fn has_changes(&self) -> bool {
        self.summary.sheets_changed != 0
    }
}

/// Convert a zero-based coordinate to Excel's A1 notation.
pub fn a1(row: usize, mut col: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push((b'A' + (col % 26) as u8) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    format!(
        "{}{}",
        letters.into_iter().rev().collect::<String>(),
        row + 1
    )
}

/// Parse an A1 coordinate into zero-based indices, rejecting invalid addresses.
pub fn parse_a1(address: &str) -> Option<(usize, usize)> {
    let split = address.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = address.split_at(split);
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    let mut col = 0usize;
    for letter in letters.bytes() {
        if !letter.is_ascii_alphabetic() {
            return None;
        }
        col = col
            .checked_mul(26)?
            .checked_add((letter.to_ascii_uppercase() - b'A' + 1) as usize)?;
    }
    Some((
        digits.parse::<usize>().ok()?.checked_sub(1)?,
        col.checked_sub(1)?,
    ))
}
