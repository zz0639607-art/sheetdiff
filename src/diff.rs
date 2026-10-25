//! Bounded LCS row alignment followed by coordinate-based cell comparison.
use crate::model::*;
use std::collections::BTreeSet;

/// Bound the alignment matrix to 16 MiB; report data is never truncated.
const MAX_LCS_ENTRIES: usize = 4_000_000;
const MAX_LCS_CELL_CHECKS: usize = 8_000_000;
type Pair = (Option<usize>, Option<usize>);

/// Compare selected sheets and retain every event for machine-readable output.
pub fn compare(old: &Workbook, new: &Workbook, options: &Options) -> Report {
    let mut report = Report {
        schema: "sheetdiff/v1",
        old: old.file.clone(),
        new: new.file.clone(),
        summary: Summary::default(),
        sheets: vec![],
        warnings: vec![],
    };
    let names: BTreeSet<_> = old
        .sheets
        .iter()
        .chain(&new.sheets)
        .map(|s| s.name.clone())
        .filter(|name| options.sheets.is_empty() || options.sheets.contains(name))
        .collect();
    for name in names {
        let a = old.sheets.iter().find(|s| s.name == name);
        let b = new.sheets.iter().find(|s| s.name == name);
        let sheet = compare_sheet(&name, a, b, options, &mut report.warnings);
        let s = &mut report.summary;
        match sheet.status {
            SheetStatus::Added => s.sheets_added += 1,
            SheetStatus::Removed => s.sheets_removed += 1,
            _ => {}
        }
        if sheet.status != SheetStatus::Unchanged {
            s.sheets_changed += 1;
        }
        for change in &sheet.changes {
            match change.kind {
                ChangeKind::Modified => s.cells_modified += 1,
                ChangeKind::Added => s.cells_added += 1,
                ChangeKind::Removed => s.cells_removed += 1,
                ChangeKind::FormulaChanged => s.formulas_changed += 1,
                ChangeKind::FormatChanged => s.formats_changed += 1,
            }
        }
        for row in &sheet.row_changes {
            match row.kind {
                ChangeKind::Added => s.rows_added += 1,
                ChangeKind::Removed => s.rows_removed += 1,
                _ => unreachable!("row events are additions or removals"),
            }
        }
        report.sheets.push(sheet);
    }
    report
}

fn compare_sheet(
    name: &str,
    a: Option<&Sheet>,
    b: Option<&Sheet>,
    options: &Options,
    warnings: &mut Vec<String>,
) -> SheetDiff {
    let mut result = SheetDiff {
        name: name.into(),
        status: SheetStatus::Unchanged,
        changes: vec![],
        row_changes: vec![],
        context: vec![],
        merged_old: a.map(|s| s.merged.clone()).unwrap_or_default(),
        merged_new: b.map(|s| s.merged.clone()).unwrap_or_default(),
    };
    let old_rows: Vec<_> = a.into_iter().flat_map(|s| s.rows.iter()).collect();
    let new_rows: Vec<_> = b.into_iter().flat_map(|s| s.rows.iter()).collect();
    let pairs = align(&old_rows, &new_rows, options, name, warnings);
    // LCS uses populated rows, so a sparse physical move could otherwise look
    // identical. Only previously observed row insertions/deletions can justify
    // moving an equal anchor. Zero displacement is always valid (an insertion
    // can fill a sparse gap). Synthetic moves do not justify further movement.
    let mut physical_pairs = Vec::with_capacity(pairs.len());
    let (mut added, mut removed) = (0i128, 0i128);
    for (ai, bi) in pairs {
        match (ai, bi) {
            (Some(i), Some(j)) => {
                let delta = *new_rows[j].0 as i128 - *old_rows[i].0 as i128;
                if delta > added || delta < -removed {
                    physical_pairs.push((Some(i), None));
                    physical_pairs.push((None, Some(j)));
                } else {
                    physical_pairs.push((ai, bi));
                }
            }
            (None, Some(_)) => {
                added += 1;
                physical_pairs.push((ai, bi));
            }
            (Some(_), None) => {
                removed += 1;
                physical_pairs.push((ai, bi));
            }
            _ => unreachable!("alignment always contains a row"),
        }
    }
    let mut unchanged = Vec::new();
    let mut changed_old = BTreeSet::new();
    let mut changed_new = BTreeSet::new();
    for (ai, bi) in physical_pairs {
        let ar = ai.map(|i| old_rows[i]);
        let br = bi.map(|i| new_rows[i]);
        let before = result.changes.len();
        let row_change = ar.is_none() || br.is_none();
        let cols: BTreeSet<_> = ar
            .into_iter()
            .chain(br)
            .flat_map(|(_, r)| r.keys().copied())
            .collect();
        for col in cols {
            let ac = ar.and_then(|(_, r)| r.get(&col));
            let bc = br.and_then(|(_, r)| r.get(&col));
            for kind in cell_changes(ac, bc, options) {
                let old_cell = ar.map(|(row, _)| a1(*row, col));
                let new_cell = br.map(|(row, _)| a1(*row, col));
                result.changes.push(Change {
                    cell: new_cell
                        .as_ref()
                        .or(old_cell.as_ref())
                        .expect("one row exists")
                        .clone(),
                    old_cell,
                    new_cell,
                    column_name: b.or(a).and_then(|s| s.columns.get(col)).cloned(),
                    kind,
                    old: ac.cloned(),
                    new: bc.cloned(),
                    row_change,
                });
            }
        }
        if row_change {
            let (row, cells) = br.or(ar).expect("one row exists");
            result.row_changes.push(RowChange {
                row: row + 1,
                kind: if br.is_some() {
                    ChangeKind::Added
                } else {
                    ChangeKind::Removed
                },
                cells: cells.clone(),
            });
        }
        if row_change || result.changes.len() != before {
            if let Some((row, _)) = ar {
                changed_old.insert(*row);
            }
            if let Some((row, _)) = br {
                changed_new.insert(*row);
            }
        } else if let (Some((old_row, _)), Some((new_row, cells))) = (ar, br) {
            unchanged.push((*old_row, *new_row, cells));
        }
    }
    if !options.skip_format && a.is_some() && b.is_some() && result.merged_old != result.merged_new
    {
        // Layout is a format event at an affected anchor; both full merge sets
        // are retained so reports describe the actual old/new range changes.
        let anchor = result
            .merged_new
            .iter()
            .find(|r| !result.merged_old.contains(r))
            .or_else(|| {
                result
                    .merged_old
                    .iter()
                    .find(|r| !result.merged_new.contains(r))
            })
            .and_then(|r| r.split(':').next())
            .unwrap_or("A1")
            .to_owned();
        let (row, col) = parse_a1(&anchor).unwrap_or((0, 0));
        changed_old.insert(row);
        changed_new.insert(row);
        result.changes.push(Change {
            cell: anchor.clone(),
            old_cell: Some(anchor.clone()),
            new_cell: Some(anchor),
            column_name: None,
            kind: ChangeKind::FormatChanged,
            old: a
                .and_then(|s| s.rows.get(&row))
                .and_then(|r| r.get(&col))
                .cloned(),
            new: b
                .and_then(|s| s.rows.get(&row))
                .and_then(|r| r.get(&col))
                .cloned(),
            row_change: false,
        });
    }
    if options.context > 0 {
        for (old_row, new_row, cells) in unchanged {
            if near(&changed_old, old_row, options.context)
                || near(&changed_new, new_row, options.context)
            {
                result.context.push(ContextRow {
                    old_row: old_row + 1,
                    new_row: new_row + 1,
                    cells: cells.clone(),
                });
            }
        }
    }
    result.status = match (a, b) {
        (None, Some(_)) => SheetStatus::Added,
        (Some(_), None) => SheetStatus::Removed,
        _ if !result.changes.is_empty() || !result.row_changes.is_empty() => SheetStatus::Modified,
        _ => SheetStatus::Unchanged,
    };
    result
}
fn near(changed: &BTreeSet<usize>, row: usize, context: usize) -> bool {
    changed
        .range(row.saturating_sub(context)..=row.saturating_add(context))
        .next()
        .is_some()
}

/// LCS selects equal-row anchors; replacement gaps pair rows by position.
///
/// First trim equal prefixes/suffixes, keeping common large sheets linear.
/// In the unmatched middle, LCS anchors preserve row identity across insertions.
/// Between anchors, min(old_len,new_len) rows are replacements; excess rows
/// are additions/deletions. Columns always align by absolute column index.
/// Ties remove an old row first, making duplicate-row output deterministic.
/// Entirely rewritten rows lack identity keys, so position is intentional.
/// A bounded matrix avoids quadratic memory on unrelated sheets. Its fallback
/// warns and compares every cell positionally; it never truncates report data.
fn align(
    a: &[(&usize, &Row)],
    b: &[(&usize, &Row)],
    options: &Options,
    name: &str,
    warnings: &mut Vec<String>,
) -> Vec<Pair> {
    let mut prefix = 0;
    while prefix < a.len().min(b.len()) && rows_equal(a[prefix].1, b[prefix].1, options) {
        prefix += 1;
    }
    let (mut ae, mut be) = (a.len(), b.len());
    while ae > prefix && be > prefix && rows_equal(a[ae - 1].1, b[be - 1].1, options) {
        ae -= 1;
        be -= 1;
    }
    let mut pairs: Vec<_> = (0..prefix).map(|i| (Some(i), Some(i))).collect();
    let (n, m) = (ae - prefix, be - prefix);
    let entries = n
        .checked_add(1)
        .and_then(|n| m.checked_add(1).and_then(|m| n.checked_mul(m)));
    let max_width = a[prefix..ae]
        .iter()
        .chain(&b[prefix..be])
        .map(|(_, r)| r.len())
        .max()
        .unwrap_or(1)
        .max(1);
    let work = entries.and_then(|entries| entries.checked_mul(max_width));
    if n == 0 || m == 0 {
        gap(&mut pairs, prefix, ae, prefix, be);
    } else if entries.is_none_or(|size| size > MAX_LCS_ENTRIES)
        || work.is_none_or(|size| size > MAX_LCS_CELL_CHECKS)
    {
        warnings.push(format!("Row alignment in sheet {name:?} exceeded the LCS budget ({MAX_LCS_ENTRIES} entries or {MAX_LCS_CELL_CHECKS} cell checks); using positional alignment. All cells are still compared."));
        gap(&mut pairs, prefix, ae, prefix, be);
    } else {
        let width = m + 1;
        let mut scores = vec![0u32; entries.expect("checked matrix")];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                scores[i * width + j] = if rows_equal(a[prefix + i].1, b[prefix + j].1, options) {
                    1 + scores[(i + 1) * width + j + 1]
                } else {
                    scores[(i + 1) * width + j].max(scores[i * width + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        let (mut sa, mut sb) = (prefix, prefix);
        while i < n && j < m {
            if rows_equal(a[prefix + i].1, b[prefix + j].1, options) {
                gap(&mut pairs, sa, prefix + i, sb, prefix + j);
                pairs.push((Some(prefix + i), Some(prefix + j)));
                i += 1;
                j += 1;
                sa = prefix + i;
                sb = prefix + j;
            } else if scores[(i + 1) * width + j] >= scores[i * width + j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
        gap(&mut pairs, sa, ae, sb, be);
    }
    for offset in 0..a.len() - ae {
        pairs.push((Some(ae + offset), Some(be + offset)));
    }
    pairs
}
fn gap(out: &mut Vec<Pair>, a: usize, ae: usize, b: usize, be: usize) {
    let common = (ae - a).min(be - b);
    out.extend((0..common).map(|i| (Some(a + i), Some(b + i))));
    out.extend((a + common..ae).map(|i| (Some(i), None)));
    out.extend((b + common..be).map(|i| (None, Some(i))));
}
fn rows_equal(a: &Row, b: &Row, options: &Options) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|((ai, ac), (bi, bc))| {
            ai == bi
                && cell_values_equal(ac, bc, options.epsilon)
                && (options.skip_formula || ac.formula == bc.formula)
                && (options.skip_format || ac.format == bc.format)
        })
}
fn cell_changes(a: Option<&Cell>, b: Option<&Cell>, options: &Options) -> Vec<ChangeKind> {
    match (a, b) {
        (None, None) => vec![],
        (None, Some(_)) => vec![ChangeKind::Added],
        (Some(_), None) => vec![ChangeKind::Removed],
        (Some(a), Some(b)) => {
            let mut kinds = Vec::new();
            if !cell_values_equal(a, b, options.epsilon) {
                kinds.push(if a.value.is_null() && a.formula.is_none() {
                    ChangeKind::Added
                } else if b.value.is_null() && b.formula.is_none() {
                    ChangeKind::Removed
                } else {
                    ChangeKind::Modified
                });
            }
            if !options.skip_formula && a.formula != b.formula {
                kinds.push(ChangeKind::FormulaChanged);
            }
            if !options.skip_format && a.format != b.format {
                kinds.push(ChangeKind::FormatChanged);
            }
            kinds
        }
    }
}
fn cell_values_equal(a: &Cell, b: &Cell, epsilon: f64) -> bool {
    // Two dates compare by normalized calendar value, including across Excel
    // epochs. A date vs a plain number compares by serial: changing only the
    // number format is a format event, which --skip-format can ignore.
    if a.date_serial.is_some() && b.date_serial.is_some() {
        values_equal(&a.value, &b.value, epsilon)
    } else {
        values_equal(
            a.date_serial.as_ref().unwrap_or(&a.value),
            b.date_serial.as_ref().unwrap_or(&b.value),
            epsilon,
        )
    }
}
fn values_equal(a: &serde_json::Value, b: &serde_json::Value, epsilon: f64) -> bool {
    if a == b {
        return true;
    }
    if let (Some(a), Some(b)) = (a.as_number(), b.as_number()) {
        let int = |n: &serde_json::Number| {
            n.as_i64()
                .map(i128::from)
                .or_else(|| n.as_u64().map(i128::from))
        };
        match (int(a), int(b)) {
            (Some(a), Some(b)) => return ((a - b).abs() as f64) <= epsilon,
            (Some(a), None) => {
                return b
                    .as_f64()
                    .is_some_and(|b| integer_float_distance(a, b) <= epsilon)
            }
            (None, Some(b)) => {
                return a
                    .as_f64()
                    .is_some_and(|a| integer_float_distance(b, a) <= epsilon)
            }
            _ => {}
        }
        if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
            return (a - b).abs() <= epsilon;
        }
    }
    false
}

// Split the float before converting the integer to f64, so values beyond 2^53
// do not collapse onto the same floating-point number during mixed comparisons.
fn integer_float_distance(integer: i128, float: f64) -> f64 {
    if float.abs() >= i128::MAX as f64 {
        return (integer as f64 - float).abs();
    }
    ((integer - float.trunc() as i128) as f64 - float.fract()).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn book(values: &[i64]) -> Workbook {
        Workbook {
            file: FileInfo {
                path: "test".into(),
                sha256: "test".into(),
            },
            sheets: vec![Sheet {
                name: "Data".into(),
                rows: values
                    .iter()
                    .enumerate()
                    .map(|(r, v)| (r, [(0, Cell::new(json!(v)))].into()))
                    .collect(),
                ..Sheet::default()
            }],
        }
    }
    #[test]
    fn empty_and_single_cell() {
        assert!(!compare(&book(&[]), &book(&[]), &Options::default()).has_changes());
        let d = compare(&book(&[1]), &book(&[2]), &Options::default());
        assert_eq!(d.sheets[0].changes[0].cell, "A1");
        assert_eq!(d.summary.cells_modified, 1);
    }
    #[test]
    fn insertion_preserves_following_coordinates() {
        let d = compare(&book(&[1, 2]), &book(&[1, 3, 2]), &Options::default());
        assert_eq!(d.summary.rows_added, 1);
        assert_eq!(d.summary.cells_modified, 0);
        assert_eq!(d.sheets[0].changes[0].new_cell.as_deref(), Some("A2"));
    }
    #[test]
    fn duplicate_rows_are_deterministic() {
        let d = compare(&book(&[1, 1, 2]), &book(&[1, 2]), &Options::default());
        assert_eq!(d.summary.rows_removed, 1);
        assert_eq!(d.summary.cells_modified, 0);
    }
    #[test]
    fn large_unrelated_sheet_warns_without_dropping_events() {
        let a = book(&(0..2100).collect::<Vec<_>>());
        let b = book(&(3000..5100).collect::<Vec<_>>());
        let d = compare(&a, &b, &Options::default());
        assert_eq!(d.summary.cells_modified, 2100);
        assert_eq!(d.warnings.len(), 1);
        assert!(d.warnings[0].contains("positional"));
        assert!(crate::render::terminal::render(&d, false, false).contains("Display truncated"));
    }
    #[test]
    fn integer_precision_is_preserved() {
        assert!(!values_equal(
            &json!(9007199254740992u64),
            &json!(9007199254740993u64),
            1e-9
        ));
    }
}
