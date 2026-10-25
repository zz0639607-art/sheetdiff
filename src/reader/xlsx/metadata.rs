//! Sparse coordinate, inheritance, and merge records from worksheet XML.
use super::{package::parse_xml, required_attr, styles};
use anyhow::{anyhow, ensure, Context, Result};
use roxmltree::Node;
use std::collections::BTreeMap;

pub(super) const MAX_CELLS: usize = 1_000_000;
const MAX_ROWS: usize = 1_048_576;
const MAX_COLUMNS: usize = 16_384;

#[derive(Default)]
pub(super) struct Metadata {
    pub(super) cells: BTreeMap<(usize, usize), Option<usize>>,
    pub(super) rows: BTreeMap<usize, Option<usize>>,
    columns: Vec<(usize, usize, usize)>,
    pub(super) merged: Vec<String>,
}

impl Metadata {
    /// Explicit cell style wins over row style, then column style. Inherited
    /// formatting applies only to present cells; missing coordinates stay absent.
    pub(super) fn style_at(&self, position: (usize, usize)) -> usize {
        self.cells
            .get(&position)
            .copied()
            .flatten()
            .or_else(|| self.rows.get(&position.0).copied().flatten())
            .or_else(|| {
                self.columns
                    .iter()
                    .rev()
                    .find(|(a, b, _)| (*a..=*b).contains(&position.1))
                    .map(|(_, _, s)| *s)
            })
            .unwrap_or(0)
    }

    pub(super) fn parse(xml: &str) -> Result<Self> {
        let doc = parse_xml(xml)?;
        let mut result = Self::default();
        let mut next_row = 0;
        for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
            let row_index = match row.attribute("r") {
                Some(r) => parse_index(r, "row")?,
                None => next_row,
            };
            validate_position((row_index, 0))?;
            next_row = row_index + 1;
            let row_style = if row
                .attribute("customFormat")
                .is_some_and(|v| !styles::boolean(v))
            {
                None
            } else {
                optional_usize(row, "s")?
            };
            result.rows.insert(row_index, row_style);
            let mut next_column = 0;
            for cell in row.children().filter(|n| n.has_tag_name("c")) {
                let position = match cell.attribute("r") {
                    Some(r) => coordinate(r)?,
                    None => (row_index, next_column),
                };
                ensure!(
                    position.0 == row_index,
                    "cell coordinate does not match its enclosing row"
                );
                validate_position(position)?;
                next_column = position.1 + 1;
                ensure!(
                    !result.cells.contains_key(&position),
                    "duplicate cell coordinate in worksheet"
                );
                result.cells.insert(position, optional_usize(cell, "s")?);
                ensure!(
                    result.cells.len() <= MAX_CELLS,
                    "worksheet exceeds the {MAX_CELLS}-cell resource limit"
                );
                // The calamine convenience API stores shared formulas in a Vec
                // indexed by si. Bound sparse IDs before that API can allocate.
                for formula in cell.children().filter(|n| n.has_tag_name("f")) {
                    if let Some(index) = optional_usize(formula, "si")? {
                        ensure!(
                            index <= MAX_CELLS,
                            "shared formula index exceeds the resource limit"
                        );
                    }
                }
            }
        }
        for column in doc.descendants().filter(|n| n.has_tag_name("col")) {
            if let Some(style) = optional_usize(column, "style")? {
                let first = parse_index(required_attr(column, "min")?, "column")?;
                let last = parse_index(required_attr(column, "max")?, "column")?;
                ensure!(
                    first <= last && last < MAX_COLUMNS,
                    "invalid column style range"
                );
                result.columns.push((first, last, style));
            }
        }
        for merge in doc.descendants().filter(|n| n.has_tag_name("mergeCell")) {
            let reference = required_attr(merge, "ref")?;
            let (first, last) = reference
                .split_once(':')
                .ok_or_else(|| anyhow!("invalid merged cell range {reference}"))?;
            let a = coordinate(first)?;
            let b = coordinate(last)?;
            ensure!(
                a.0 <= b.0 && a.1 <= b.1,
                "invalid merged cell range {reference}"
            );
            result.merged.push(format!(
                "{}:{}",
                crate::model::a1(a.0, a.1),
                crate::model::a1(b.0, b.1)
            ));
        }
        result.merged.sort();
        result.merged.dedup();
        Ok(result)
    }
}

fn optional_usize(node: Node<'_, '_>, attribute: &str) -> Result<Option<usize>> {
    node.attribute(attribute)
        .map(|v| {
            v.parse()
                .with_context(|| format!("invalid {attribute} attribute"))
        })
        .transpose()
}
fn parse_index(value: &str, kind: &str) -> Result<usize> {
    value
        .parse::<usize>()
        .ok()
        .and_then(|v| v.checked_sub(1))
        .ok_or_else(|| anyhow!("invalid {kind} index {value}"))
}
pub(super) fn validate_position((row, col): (usize, usize)) -> Result<()> {
    ensure!(
        row < MAX_ROWS && col < MAX_COLUMNS,
        "cell coordinate exceeds XLSX row or column limits"
    );
    Ok(())
}
fn coordinate(reference: &str) -> Result<(usize, usize)> {
    let split = reference
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| anyhow!("invalid cell coordinate {reference}"))?;
    let (letters, digits) = reference.split_at(split);
    ensure!(
        !letters.is_empty() && letters.bytes().all(|c| c.is_ascii_uppercase()),
        "invalid cell coordinate {reference}"
    );
    let mut col = 0usize;
    for c in letters.bytes() {
        col = col
            .checked_mul(26)
            .and_then(|v| v.checked_add((c - b'A' + 1) as usize))
            .ok_or_else(|| anyhow!("cell coordinate is too large"))?;
    }
    let position = (parse_index(digits, "row")?, col - 1);
    validate_position(position)?;
    Ok(position)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_maximum_coordinate_and_explicit_blank() {
        let meta = Metadata::parse(r#"<worksheet><sheetData><row r="1048576"><c r="XFD1048576"/></row></sheetData></worksheet>"#).unwrap();
        assert!(meta.cells.contains_key(&(1_048_575, 16_383)));
        assert_eq!(meta.cells.len(), 1);
    }
    #[test]
    fn inherited_style_precedence() {
        let meta = Metadata::parse(r#"<worksheet><cols><col min="1" max="2" style="3"/></cols><sheetData><row r="1" s="2"><c r="A1" s="1"/><c r="B1"/></row><row r="2"><c r="A2"/></row></sheetData></worksheet>"#).unwrap();
        assert_eq!(meta.style_at((0, 0)), 1);
        assert_eq!(meta.style_at((0, 1)), 2);
        assert_eq!(meta.style_at((1, 0)), 3);
    }
    #[test]
    fn invalid_sparse_formula_id_is_rejected() {
        assert!(Metadata::parse(r#"<worksheet><sheetData><row r="1"><c r="A1"><f t="shared" si="999999999"/></c></row></sheetData></worksheet>"#).is_err());
    }
}
