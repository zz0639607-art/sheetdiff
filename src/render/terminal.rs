//! Unicode-aware terminal tables with optional ANSI color and bounded display.
use crate::model::*;
use comfy_table::{presets::NOTHING, ContentArrangement, Table};
use owo_colors::OwoColorize;
use std::collections::BTreeSet;

/// Truncate presentation only; the report and JSON always retain all changes.
pub const MAX_DISPLAY_ROWS: usize = 1000;

/// Lowercase sheet status shared by human-readable renderers.
pub fn status_name(status: SheetStatus) -> &'static str {
    match status {
        SheetStatus::Unchanged => "unchanged",
        SheetStatus::Modified => "modified",
        SheetStatus::Added => "added",
        SheetStatus::Removed => "removed",
    }
}

/// Render a report with predictable plain text or explicit ANSI colors.
pub fn render(report: &Report, color: bool, summary_only: bool) -> String {
    let mut out = String::new();
    if !summary_only {
        out.push_str(&paint(
            &format!(
                "--- a: {}\n+++ b: {}\n",
                safe(&report.old.path),
                safe(&report.new.path)
            ),
            "header",
            color,
        ));
        for sheet in &report.sheets {
            if sheet.status == SheetStatus::Unchanged {
                continue;
            }
            out.push_str(&paint(
                &format!(
                    "\nSheet: {} ({})\n",
                    safe(&sheet.name),
                    status_name(sheet.status)
                ),
                "header",
                color,
            ));
            for (side, ranges) in [("Old", &sheet.merged_old), ("New", &sheet.merged_new)] {
                for range in ranges {
                    out.push_str(&format!("  {side}: (merged {})\n", safe(range)));
                }
            }
            if matches!(sheet.status, SheetStatus::Added | SheetStatus::Removed) {
                let action = status_name(sheet.status);
                let kind = if sheet.status == SheetStatus::Added {
                    "[+]"
                } else {
                    "[-]"
                };
                out.push_str(&paint(
                    &format!(
                        "  (all {} rows {action}; {} cells)\n",
                        sheet.row_changes.len(),
                        sheet.changes.len()
                    ),
                    kind,
                    color,
                ));
                continue;
            }
            let mut events = Vec::<(usize, u8, Vec<String>)>::new();
            for change in sheet.changes.iter().filter(|c| !c.row_change) {
                let tag = tag(change.kind);
                let coordinate = match (&change.old_cell, &change.new_cell) {
                    (Some(a), Some(b)) if a != b => format!("{a} → {b}"),
                    _ => change.cell.clone(),
                };
                let coordinate = match &change.column_name {
                    Some(label) => format!("{coordinate} ({})", safe(label)),
                    None => coordinate,
                };
                let row = parse_a1(&change.cell).map(|(r, _)| r).unwrap_or(0);
                events.push((
                    row,
                    1,
                    vec![
                        paint(tag, tag, color),
                        coordinate,
                        paint(&safe(&change_details(change)), tag, color),
                    ],
                ));
            }
            for row in &sheet.row_changes {
                let tag = tag(row.kind);
                let label = if row.kind == ChangeKind::Added {
                    "added"
                } else {
                    "removed"
                };
                events.push((
                    row.row - 1,
                    0,
                    vec![
                        paint(tag, tag, color),
                        format!("Row {} ({label}):", row.row),
                        paint(&row_text(row.row - 1, &row.cells), tag, color),
                    ],
                ));
            }
            for context in &sheet.context {
                events.push((
                    context.new_row - 1,
                    2,
                    vec![
                        "[ ]".into(),
                        format!("Context {}/{}", context.old_row, context.new_row),
                        row_text(context.new_row - 1, &context.cells),
                    ],
                ));
            }
            events.sort_by_key(|(row, kind, _)| (*row, *kind));
            let mut table = Table::new();
            table
                .load_preset(NOTHING)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_width(120);
            for (_, _, cells) in events.iter().take(MAX_DISPLAY_ROWS) {
                table.add_row(cells.clone());
            }
            out.push_str(&format!("{table}\n"));
            if events.len() > MAX_DISPLAY_ROWS {
                out.push_str(&format!("  Display truncated after {MAX_DISPLAY_ROWS} rows/events ({} omitted); JSON retains all changes.\n",events.len()-MAX_DISPLAY_ROWS));
            }
        }
        for warning in &report.warnings {
            out.push_str(&format!("Warning: {}\n", safe(warning)));
        }
    }
    let s = &report.summary;
    out.push_str(&format!("Summary: {} sheets changed | {} modified / {} formulas / {} formats / {} cells added / {} cells removed / {} rows added / {} rows removed / {} sheets added / {} sheets removed\n",
        s.sheets_changed,s.cells_modified,s.formulas_changed,s.formats_changed,s.cells_added,s.cells_removed,s.rows_added,s.rows_removed,s.sheets_added,s.sheets_removed));
    out
}

fn row_text(row: usize, cells: &Row) -> String {
    let mut text = cells
        .iter()
        .take(100)
        .map(|(col, cell)| format!("{}: {}", a1(row, *col), safe(cell.display())))
        .collect::<Vec<_>>()
        .join(" | ");
    if cells.len() > 100 {
        text.push_str(&format!(
            " | … {} more cells (display truncated)",
            cells.len() - 100
        ));
    }
    if cells.is_empty() {
        text.push_str("(empty row)");
    }
    text
}
fn tag(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "[+]",
        ChangeKind::Removed => "[-]",
        ChangeKind::FormulaChanged => "[F]",
        ChangeKind::FormatChanged => "[FMT]",
        ChangeKind::Modified => "[M]",
    }
}

/// Describe the specific property represented by an event, including cached values.
pub(crate) fn change_details(change: &Change) -> String {
    match change.kind {
        ChangeKind::FormulaChanged => format!(
            "Formula: {} → {}",
            change
                .old
                .as_ref()
                .and_then(|c| c.formula.as_deref())
                .unwrap_or("(none)"),
            change
                .new
                .as_ref()
                .and_then(|c| c.formula.as_deref())
                .unwrap_or("(none)")
        ),
        ChangeKind::FormatChanged => {
            let a = change.old.as_ref().map(|c| &c.format);
            let b = change.new.as_ref().map(|c| &c.format);
            let keys: BTreeSet<_> = a.into_iter().chain(b).flat_map(|f| f.keys()).collect();
            let details = keys
                .into_iter()
                .filter_map(|key| {
                    let old = a.and_then(|f| f.get(key));
                    let new = b.and_then(|f| f.get(key));
                    (old != new).then(|| {
                        format!(
                            "{key}: {} → {}",
                            old.map(String::as_str).unwrap_or("(default)"),
                            new.map(String::as_str).unwrap_or("(default)")
                        )
                    })
                })
                .collect::<Vec<_>>();
            format!(
                "Format: {}",
                if details.is_empty() {
                    "merge layout changed".into()
                } else {
                    details.join("; ")
                }
            )
        }
        _ => format!(
            "Value: {} → {}",
            change
                .old
                .as_ref()
                .map(|c| c.formatted.as_str())
                .unwrap_or("(missing)"),
            change
                .new
                .as_ref()
                .map(|c| c.formatted.as_str())
                .unwrap_or("(missing)")
        ),
    }
}

/// Escape control characters so workbook text cannot inject terminal sequences.
fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn paint(text: &str, kind: &str, color: bool) -> String {
    if !color {
        return text.into();
    }
    match kind {
        "[+]" => text.green().to_string(),
        "[-]" => text.red().to_string(),
        "[FMT]" => text.magenta().to_string(),
        "header" => text.cyan().to_string(),
        _ => text.yellow().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_escapes_are_visible() {
        assert_eq!(safe("a\u{1b}[2J\n"), "a\\u{1b}[2J\\n");
    }
}
