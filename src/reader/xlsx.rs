//! Streaming XLSX values with sparse XML metadata for styles and explicit blanks.
mod metadata;
mod package;
mod styles;

use crate::model::{Cell, Sheet};
use anyhow::{anyhow, ensure, Context, Result};
use calamine::{open_workbook, Data, ExcelDateTime, Xlsx};
use metadata::Metadata;
use package::{parse_xml, resolve_target, Package};
use roxmltree::Node;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use styles::{Style, Styles};

/// Resolve package metadata before calamine eagerly loads strings and styles.
/// Stream actual cells: a lone XFD1048576 never allocates a dense rectangle.
pub(super) fn read(path: &Path) -> Result<Vec<Sheet>> {
    let mut package = Package::open(path)?;
    let root_xml = package.xml("_rels/.rels")?;
    let root = parse_xml(&root_xml)?;
    let workbook_target = root
        .descendants()
        .find(|n| n.has_tag_name("Relationship") && relation_type(*n).ends_with("/officeDocument"))
        .ok_or_else(|| anyhow!("XLSX package has no workbook relationship"))?;
    ensure_local(workbook_target)?;
    let workbook_path = resolve_target("", required_attr(workbook_target, "Target")?)?;
    let workbook_xml = package.xml(&workbook_path)?;
    let workbook_doc = parse_xml(&workbook_xml)?;
    let rels_xml = package.xml(&package::relationships_path(&workbook_path))?;
    let rels_doc = parse_xml(&rels_xml)?;
    let relationships = rels_doc
        .descendants()
        .filter(|n| n.has_tag_name("Relationship"))
        .map(|n| Ok((required_attr(n, "Id")?.to_owned(), n)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut styles_xml = None;
    let mut theme_xml = None;
    for relationship in relationships.values() {
        let kind = relation_type(*relationship);
        if kind.ends_with("/styles") || kind.ends_with("/theme") {
            ensure_local(*relationship)?;
            let target = resolve_target(&workbook_path, required_attr(*relationship, "Target")?)?;
            let xml = package.xml(&target)?;
            if kind.ends_with("/styles") {
                styles_xml = Some(xml);
            } else {
                theme_xml = Some(xml);
            }
        }
    }
    let styles = Styles::parse(styles_xml.as_deref(), theme_xml.as_deref())?;
    let mut inputs = Vec::new();
    for node in workbook_doc
        .descendants()
        .filter(|n| n.has_tag_name("sheet"))
    {
        let name = required_attr(node, "name")?.to_owned();
        let id = node
            .attributes()
            .find(|a| a.name() == "id")
            .map(|a| a.value())
            .ok_or_else(|| anyhow!("worksheet {name} has no relationship ID"))?;
        let relationship = relationships
            .get(id)
            .ok_or_else(|| anyhow!("worksheet {name} has an unknown relationship ID"))?;
        // Chart sheets are not cell grids. Resolve worksheet paths from rels,
        // never by assuming workbook order equals sheetN.xml numbering.
        if !relation_type(*relationship).ends_with("/worksheet") {
            continue;
        }
        ensure_local(*relationship)?;
        let target = resolve_target(&workbook_path, required_attr(*relationship, "Target")?)?;
        inputs.push((name, target));
    }
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let is_1904 = workbook.has_1904_epoch();
    let mut sheets = Vec::with_capacity(inputs.len());
    for (name, target) in inputs {
        let xml = package.xml(&target)?;
        let metadata =
            Metadata::parse(&xml).with_context(|| format!("invalid worksheet {name}"))?;
        let mut sheet = Sheet {
            name: name.clone(),
            merged: metadata.merged.clone(),
            ..Sheet::default()
        };
        for row in metadata.rows.keys() {
            sheet.rows.entry(*row).or_default();
        }
        let mut reader = workbook.worksheet_cells_reader(&name)?;
        let mut count = 0;
        while let Some(record) = reader.next_cell_with_formula()? {
            count += 1;
            ensure!(
                count <= metadata::MAX_CELLS,
                "worksheet {name} exceeds the cell resource limit"
            );
            let position = (record.pos.0 as usize, record.pos.1 as usize);
            metadata::validate_position(position)?;
            let style = styles.get(metadata.style_at(position))?;
            let data = Data::from(record.value);
            let serial = match &data {
                Data::DateTime(date) => Some(json!(date.as_f64())),
                Data::Float(n) if style.date_type.is_some() => Some(json!(n)),
                Data::Int(n) if style.date_type.is_some() => Some(json!(n)),
                _ => None,
            };
            let mut cell = Cell::new(normalize(data, style, is_1904)?);
            cell.date_serial = serial;
            cell.formula = record
                .formula
                .map(|s| format!("={}", s.trim_start_matches('=')));
            cell.format = style.format.clone();
            sheet
                .rows
                .entry(position.0)
                .or_default()
                .insert(position.1, cell);
        }
        // Preserve self-closing <c/> records skipped by some value readers.
        for position in metadata.cells.keys() {
            if !sheet
                .rows
                .get(&position.0)
                .is_some_and(|r| r.contains_key(&position.1))
            {
                let mut cell = Cell::new(Value::Null);
                cell.format = styles.get(metadata.style_at(*position))?.format.clone();
                sheet
                    .rows
                    .entry(position.0)
                    .or_default()
                    .insert(position.1, cell);
            }
        }
        sheets.push(sheet);
    }
    Ok(sheets)
}

fn relation_type<'a>(node: Node<'a, '_>) -> &'a str {
    node.attribute("Type").unwrap_or("")
}
fn ensure_local(node: Node<'_, '_>) -> Result<()> {
    ensure!(
        node.attribute("TargetMode") != Some("External"),
        "external XLSX relationships are not supported"
    );
    Ok(())
}
fn required_attr<'a>(node: Node<'a, '_>, attribute: &str) -> Result<&'a str> {
    node.attribute(attribute)
        .ok_or_else(|| anyhow!("missing XML attribute {attribute}"))
}

fn normalize(data: Data, style: &Style, is_1904: bool) -> Result<Value> {
    let number = match &data {
        Data::Float(n) => Some(*n),
        Data::Int(n) => Some(*n as f64),
        _ => None,
    };
    if let Some(number) = number {
        ensure!(number.is_finite(), "non-finite number in XLSX cell");
        if let Some(kind) = style.date_type {
            return normalize(
                Data::DateTime(ExcelDateTime::new(number, kind, is_1904)),
                &Style::default(),
                is_1904,
            );
        }
    }
    Ok(match data {
        Data::Int(n) => json!(n),
        Data::Float(n) => json!(n),
        Data::Bool(v) => json!(v),
        Data::String(s) => json!(s),
        Data::DateTime(date) if date.is_duration() => json!(duration(date.as_f64())?),
        Data::DateTime(date) => {
            ensure!(date.as_f64().is_finite(), "non-finite Excel date");
            let (y, m, d, h, min, s, ms) = date.to_ymd_hms_milli();
            json!(format!(
                "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}.{ms:03}"
            ))
        }
        Data::DateTimeIso(s) | Data::DurationIso(s) => json!(s),
        Data::Error(e) => json!(e.to_string()),
        Data::Empty => Value::Null,
    })
}

fn duration(days: f64) -> Result<String> {
    let millis = (days * 86_400_000.0).round();
    ensure!(
        millis.is_finite() && millis.abs() < i64::MAX as f64,
        "Excel duration exceeds supported range"
    );
    let negative = millis < 0.0;
    let millis = millis.abs() as u64;
    let seconds = millis / 1000;
    let fraction = millis % 1000;
    let value = if fraction == 0 {
        seconds.to_string()
    } else {
        format!("{seconds}.{fraction:03}")
    };
    Ok(format!("{}PT{value}S", if negative { "-" } else { "" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations_are_iso_intervals() {
        assert_eq!(duration(1.5).unwrap(), "PT129600S");
        assert_eq!(duration(-0.5).unwrap(), "-PT43200S");
        let date = ExcelDateTime::new(1.5, calamine::ExcelDateTimeType::TimeDelta, false);
        assert_eq!(
            normalize(Data::DateTime(date), &Style::default(), false).unwrap(),
            json!("PT129600S")
        );
    }
}
