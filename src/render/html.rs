//! Self-contained, script-free review reports built with escaped strings.
use crate::model::*;
use crate::render::terminal::{change_details, status_name, MAX_DISPLAY_ROWS};
use std::collections::{BTreeMap, BTreeSet};

const MAX_DISPLAY_COLUMNS: usize = 100;
const CSS: &str = r#"
:root{color-scheme:light;--ink:#172b3b;--muted:#617281;--line:#dce4e9;--paper:#fff;--bg:#f3f6f8;--red:#a32231;--green:#116b48;--cyan:#087e91}*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.55 system-ui,-apple-system,"Segoe UI",sans-serif}main{max-width:1500px;margin:auto;padding:42px 32px 24px}header{margin-bottom:24px}.eyebrow{margin:0;color:var(--cyan);font-size:12px;letter-spacing:.14em;text-transform:uppercase;font-weight:750}h1{font-size:32px;letter-spacing:-.04em;line-height:1.2;margin:8px 0}h2{font-size:19px;margin:0 0 8px}p{margin:8px 0}.muted{color:var(--muted)}.files{display:grid;grid-template-columns:1fr 1fr;gap:14px;margin:20px 0}.file,.metric,.sheet-panel,.unchanged{background:var(--paper);border:1px solid var(--line);border-radius:10px}.file{padding:16px 18px;min-width:0}.file .label{font-size:11px;letter-spacing:.08em;font-weight:750;text-transform:uppercase}.file.old-file{border-top:3px solid #d16b75}.file.new-file{border-top:3px solid #4ba27e}.path{font-size:15px;font-weight:650;overflow-wrap:anywhere}.hash{display:block;color:var(--muted);font-size:11px;overflow-wrap:anywhere;margin-top:8px}.metrics{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:10px}.metric{padding:13px 16px}.metric strong{display:block;font-size:24px;line-height:1.25;letter-spacing:-.03em}.metric span{font-size:12px;color:var(--muted)}.legend{display:flex;gap:18px;flex-wrap:wrap;color:var(--muted);font-size:12px;margin:20px 0}.dot{display:inline-block;width:9px;height:9px;border-radius:2px;margin-right:5px}.dot.old-dot{background:#dc8a94}.dot.new-dot{background:#6dbd99}.sheet-tabs{margin-top:26px}.tab-input{position:absolute;width:1px;height:1px;clip-path:inset(50%);overflow:hidden}.tab-label{display:inline-flex;align-items:center;gap:8px;cursor:pointer;padding:10px 15px;margin:0 6px 8px 0;background:#e6edf1;border:1px solid transparent;border-radius:8px;font-weight:650}.tab-input:checked+.tab-label{background:#e6f4f6;color:#086476;border-color:#8fc6cf}.tab-input:focus-visible+.tab-label{outline:3px solid #087e91;outline-offset:2px}.badge{display:inline-block;border-radius:4px;padding:2px 6px;font-size:10px;font-weight:700;line-height:1.3;letter-spacing:.02em;background:#e8eef2;color:#4a6071}.badge.added{background:#dbf0e5;color:var(--green)}.badge.removed{background:#fae3e6;color:var(--red)}.badge.formula_changed{background:#fff1c9;color:#886002}.badge.format_changed{background:#f2e3f7;color:#80478f}.sheet-panel{display:none;overflow:hidden}.sheet-intro{padding:20px 22px 15px;border-bottom:1px solid var(--line)}.sheet-intro .badge{vertical-align:middle;margin-left:8px}.sheet-meta{font-size:12px;color:var(--muted)}.merge-note{font-size:12px;margin-top:4px}.merge-note code{background:#edf3f6;padding:1px 4px;border-radius:3px}.grid-wrap{overflow:auto;max-height:76vh}table{border-collapse:separate;border-spacing:0;width:100%;font-size:13px}th,td{border-right:1px solid var(--line);border-bottom:1px solid var(--line);padding:11px 13px;vertical-align:top;min-width:150px;max-width:440px;overflow-wrap:anywhere}th:last-child,td:last-child{border-right:0}thead th{background:#eaf0f4;position:sticky;top:0;z-index:2;text-align:left;font-size:12px;color:#4e6575}thead th:first-child{left:0;z-index:3;min-width:125px}tbody th{position:sticky;left:0;z-index:1;background:#f6f8fa;font-size:12px;color:var(--muted);min-width:125px;font-weight:500;text-align:left}.row-number{font-variant-numeric:tabular-nums;font-family:ui-monospace,SFMono-Regular,Consolas,monospace;color:#344e60;white-space:nowrap}.row-label{display:block;font-size:10px;margin-top:3px}.column-name{display:block;color:#6c8090;font-size:11px;font-weight:500;max-width:220px}.cell-coordinate{display:block;color:#6d7e8a;font-size:10px;margin-bottom:5px;font-family:ui-monospace,SFMono-Regular,Consolas,monospace}.cell-badges{display:flex;gap:4px;flex-wrap:wrap;margin-top:7px}.old,.new,.same{display:block;white-space:pre-wrap;overflow-wrap:anywhere;border-radius:3px;padding:3px 6px}.old{background:#fbe7e9;color:var(--red)}.new{background:#e0f3e9;color:var(--green)}.same{padding-left:0;color:#526b7b}.arrow{display:block;color:#91a0ab;font-size:11px;padding-left:6px;line-height:1.35}.changed{background:#fffdf6}.context-row td{background:#fafcfd}.not-included{color:#b6c3cc;text-align:center;background:#fafcfd}.row-added th{box-shadow:inset 3px 0 #57a77e}.row-removed th{box-shadow:inset 3px 0 #cf7a85}.truncation,.warning{padding:12px 18px;background:#fff4d9;border:1px solid #ead6a5;border-radius:7px;color:#785819;font-size:12px;margin:12px 0}.sheet-panel .truncation{margin:14px 18px}.empty{padding:30px 22px;color:var(--muted)}.unchanged{padding:15px 18px;margin-top:18px;color:#75838e;background:#edf1f4}.unchanged summary{cursor:pointer;font-weight:650}.unchanged ul{padding-left:20px;margin-bottom:0}.no-changes{background:#e6f3ed;border:1px solid #a9cfbc;border-radius:10px;padding:24px;color:#205d43;margin:25px 0}footer{font-size:11px;color:#7e8e9a;margin:24px 0 8px}.print-note{font-size:11px;color:#7b8d99}@media(max-width:750px){main{padding:22px 14px}.files{grid-template-columns:1fr}.metrics{grid-template-columns:repeat(2,1fr)}h1{font-size:26px}.tab-label{font-size:12px;padding:8px 10px}.sheet-intro{padding:15px}}@media print{body{background:white}main{padding:0;max-width:none}.sheet-panel{display:block!important;margin:18px 0;break-inside:avoid}.tab-input,.tab-label{display:none}.grid-wrap{overflow:visible;max-height:none}.metrics{grid-template-columns:repeat(5,1fr)}th,td{min-width:0;font-size:10px}thead th,tbody th{position:static}.unchanged{display:block}}
"#;

/// Build an offline report. All input text is escaped before entering markup.
pub fn render(report: &Report) -> String {
    let changed: Vec<_> = report
        .sheets
        .iter()
        .filter(|s| s.status != SheetStatus::Unchanged)
        .collect();
    let unchanged: Vec<_> = report
        .sheets
        .iter()
        .filter(|s| s.status == SheetStatus::Unchanged)
        .collect();
    let mut out = String::from("<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\"><title>sheetdiff review report</title><style>");
    out.push_str(CSS);
    for index in 0..changed.len() {
        out.push_str(&format!(
            "#tab-{index}:checked~.panels>#sheet-{index}{{display:block}}\n"
        ));
    }
    out.push_str("</style></head><body><main><header><p class=\"eyebrow\">sheetdiff · offline review</p><h1>Spreadsheet changes</h1><p class=\"muted\">Cell-level comparison with original coordinates, formulas, and formatting.</p><div class=\"files\">");
    file(&mut out, "Old", "old-file", &report.old);
    file(&mut out, "New", "new-file", &report.new);
    out.push_str("</div><div class=\"metrics\">");
    let s = &report.summary;
    for (label, value) in [
        ("Sheets changed", s.sheets_changed),
        ("Values modified", s.cells_modified),
        ("Formulas changed", s.formulas_changed),
        ("Formats changed", s.formats_changed),
        ("Cells added", s.cells_added),
        ("Cells removed", s.cells_removed),
        ("Rows added", s.rows_added),
        ("Rows removed", s.rows_removed),
        ("Sheets added", s.sheets_added),
        ("Sheets removed", s.sheets_removed),
    ] {
        out.push_str(&format!(
            "<div class=\"metric\"><strong>{value}</strong><span>{label}</span></div>"
        ));
    }
    out.push_str("</div></header><div class=\"legend\"><span><i class=\"dot old-dot\"></i>Old value</span><span><i class=\"dot new-dot\"></i>New value</span><span>Hover a changed cell for old → new details.</span><span>… = cell outside the displayed change/context set</span></div>");
    for warning in &report.warnings {
        out.push_str(&format!(
            "<aside class=\"warning\">{}</aside>",
            escape(warning)
        ));
    }
    if changed.is_empty() {
        out.push_str("<div class=\"no-changes\"><h2>No differences found</h2><p>The selected sheets are identical under the comparison options.</p></div>");
    } else {
        out.push_str("<section class=\"sheet-tabs\" aria-label=\"Changed sheets\">");
        for (index, sheet) in changed.iter().enumerate() {
            let checked = if index == 0 { " checked" } else { "" };
            out.push_str(&format!("<input class=\"tab-input\" type=\"radio\" name=\"sheet\" id=\"tab-{index}\"{checked}><label class=\"tab-label\" for=\"tab-{index}\">{}<span class=\"badge {}\">{}</span></label>", escape(&sheet.name), status_name(sheet.status), status_name(sheet.status)));
        }
        out.push_str("<div class=\"panels\">");
        for (index, sheet) in changed.iter().enumerate() {
            panel(&mut out, sheet, index);
        }
        out.push_str("</div></section>");
    }
    out.push_str(&format!(
        "<details class=\"unchanged\"><summary>Unchanged sheets ({})</summary>",
        unchanged.len()
    ));
    if unchanged.is_empty() {
        out.push_str("<p>No unchanged sheets in the selected comparison.</p>");
    } else {
        out.push_str("<ul>");
        for sheet in unchanged {
            out.push_str(&format!("<li>{}</li>", escape(&sheet.name)));
        }
        out.push_str("</ul>");
    }
    out.push_str("</details><footer>Generated by sheetdiff · sheetdiff/v1 · All resources are embedded; this report works offline without JavaScript.</footer></main></body></html>\n");
    out
}

fn file(out: &mut String, label: &str, class: &str, file: &FileInfo) {
    out.push_str(&format!("<div class=\"file {class}\"><div class=\"label\">{label} file</div><div class=\"path\">{}</div><code class=\"hash\">SHA256: {}</code></div>", escape(&file.path), escape(&file.sha256)));
}

enum GridCell<'a> {
    Changes(Vec<&'a Change>),
    Context(&'a Cell),
}
struct GridRow<'a> {
    old_row: Option<usize>,
    new_row: Option<usize>,
    kind: &'static str,
    cells: BTreeMap<usize, GridCell<'a>>,
}
type GridKey = (usize, u8, Option<usize>, Option<usize>);

fn panel(out: &mut String, sheet: &SheetDiff, index: usize) {
    let mut rows = BTreeMap::<GridKey, GridRow<'_>>::new();
    let mut column_names = BTreeMap::new();
    for change in &sheet.changes {
        let Some((row, col)) = parse_a1(&change.cell) else {
            continue;
        };
        let old_row = change
            .old_cell
            .as_deref()
            .and_then(parse_a1)
            .map(|(r, _)| r + 1);
        let new_row = change
            .new_cell
            .as_deref()
            .and_then(parse_a1)
            .map(|(r, _)| r + 1);
        let (order, kind) = if change.row_change {
            if change.kind == ChangeKind::Removed {
                (0, "removed")
            } else {
                (2, "added")
            }
        } else {
            (1, "modified")
        };
        let grid = rows
            .entry((row, order, old_row, new_row))
            .or_insert_with(|| GridRow {
                old_row,
                new_row,
                kind,
                cells: BTreeMap::new(),
            });
        match grid
            .cells
            .entry(col)
            .or_insert_with(|| GridCell::Changes(vec![]))
        {
            GridCell::Changes(changes) => changes.push(change),
            GridCell::Context(_) => unreachable!("changes are inserted first"),
        }
        if let Some(label) = &change.column_name {
            column_names.entry(col).or_insert(label);
        }
    }
    for change in &sheet.row_changes {
        let (old_row, new_row, order, kind) = if change.kind == ChangeKind::Removed {
            (Some(change.row), None, 0, "removed")
        } else {
            (None, Some(change.row), 2, "added")
        };
        rows.entry((change.row - 1, order, old_row, new_row))
            .or_insert_with(|| GridRow {
                old_row,
                new_row,
                kind,
                cells: BTreeMap::new(),
            });
    }
    for context in &sheet.context {
        let grid = rows
            .entry((
                context.new_row - 1,
                1,
                Some(context.old_row),
                Some(context.new_row),
            ))
            .or_insert_with(|| GridRow {
                old_row: Some(context.old_row),
                new_row: Some(context.new_row),
                kind: "context",
                cells: BTreeMap::new(),
            });
        for (col, cell) in &context.cells {
            grid.cells.entry(*col).or_insert(GridCell::Context(cell));
        }
    }
    let all_columns: BTreeSet<_> = rows
        .values()
        .flat_map(|row| row.cells.keys().copied())
        .collect();
    let columns: Vec<_> = all_columns
        .iter()
        .copied()
        .take(MAX_DISPLAY_COLUMNS)
        .collect();
    out.push_str(&format!("<article class=\"sheet-panel\" id=\"sheet-{index}\"><div class=\"sheet-intro\"><h2>{}<span class=\"badge {}\">{}</span></h2><p class=\"sheet-meta\">{} cell events · {} row additions/deletions · {} context rows</p>", escape(&sheet.name), status_name(sheet.status), status_name(sheet.status), sheet.changes.len(), sheet.row_changes.len(), sheet.context.len()));
    for (label, merges) in [("Old", &sheet.merged_old), ("New", &sheet.merged_new)] {
        for merge in merges {
            out.push_str(&format!(
                "<div class=\"merge-note\">{label}: <code>(merged {})</code></div>",
                escape(merge)
            ));
        }
    }
    out.push_str("</div>");
    if rows.is_empty() {
        out.push_str(&format!(
            "<p class=\"empty\">Empty worksheet {}.</p>",
            status_name(sheet.status)
        ));
    } else {
        out.push_str(
            "<div class=\"grid-wrap\"><table><thead><tr><th scope=\"col\">Old / New row</th>",
        );
        for col in &columns {
            let letter = a1(0, *col);
            let letter = letter.trim_end_matches('1');
            out.push_str(&format!("<th scope=\"col\">{letter}"));
            if let Some(label) = column_names.get(col) {
                out.push_str(&format!(
                    "<span class=\"column-name\">{}</span>",
                    escape(label)
                ));
            }
            out.push_str("</th>");
        }
        if columns.is_empty() {
            out.push_str("<th scope=\"col\">Row content</th>");
        }
        out.push_str("</tr></thead><tbody>");
        for row in rows.values().take(MAX_DISPLAY_ROWS) {
            let old = row
                .old_row
                .map(|r| r.to_string())
                .unwrap_or_else(|| "—".into());
            let new = row
                .new_row
                .map(|r| r.to_string())
                .unwrap_or_else(|| "—".into());
            let context = if row.kind == "context" {
                " context-row"
            } else {
                ""
            };
            out.push_str(&format!("<tr class=\"row-{}{context}\"><th scope=\"row\"><span class=\"row-number\">{old} / {new}</span><span class=\"row-label\">{}</span></th>",row.kind,row.kind));
            for col in &columns {
                match row.cells.get(col) {
                    Some(cell) => grid_cell(out,cell,*col,row),
                    None => out.push_str("<td class=\"not-included\" title=\"Cell outside the displayed change/context set\">…</td>"),
                }
            }
            if columns.is_empty() {
                out.push_str("<td class=\"same\">(empty row)</td>");
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table></div>");
    }
    if rows.len() > MAX_DISPLAY_ROWS || all_columns.len() > MAX_DISPLAY_COLUMNS {
        out.push_str(&format!("<p class=\"truncation\">Display truncated: showing at most {MAX_DISPLAY_ROWS} rows and {MAX_DISPLAY_COLUMNS} populated columns per sheet ({} rows, {} columns in the report). JSON retains all changes.</p>",rows.len(),all_columns.len()));
    }
    out.push_str("</article>");
}

fn grid_cell(out: &mut String, cell: &GridCell<'_>, col: usize, row: &GridRow<'_>) {
    match cell {
        GridCell::Context(cell) => out.push_str(&format!(
            "<td title=\"Unchanged context\"><span class=\"same\">{}</span></td>",
            escape(cell.display())
        )),
        GridCell::Changes(changes) => {
            let first = changes[0];
            let details = changes
                .iter()
                .map(|change| change_details(change))
                .collect::<Vec<_>>()
                .join("\n");
            let mut kinds = Vec::new();
            for change in changes {
                if !kinds.contains(&change.kind) {
                    kinds.push(change.kind);
                }
            }
            let formula = kinds.contains(&ChangeKind::FormulaChanged);
            let display = |cell: Option<&Cell>| match cell {
                Some(cell) if formula => cell.display().to_owned(),
                Some(cell) => cell.formatted.clone(),
                None => "(missing)".into(),
            };
            let old_coord = first
                .old_cell
                .clone()
                .or_else(|| row.old_row.map(|r| a1(r - 1, col)))
                .unwrap_or_else(|| "—".into());
            let new_coord = first
                .new_cell
                .clone()
                .or_else(|| row.new_row.map(|r| a1(r - 1, col)))
                .unwrap_or_else(|| "—".into());
            out.push_str(&format!("<td class=\"changed\" title=\"{}\"><span class=\"cell-coordinate\">{} → {}</span><span class=\"old\">{}</span><span class=\"arrow\" aria-hidden=\"true\">→</span><span class=\"new\">{}</span><span class=\"cell-badges\">",escape(&details),escape(&old_coord),escape(&new_coord),escape(&display(first.old.as_ref())),escape(&display(first.new.as_ref()))));
            for kind in kinds {
                let (class, label) = match kind {
                    ChangeKind::Modified => ("modified", "Value"),
                    ChangeKind::Added => ("added", "Added"),
                    ChangeKind::Removed => ("removed", "Removed"),
                    ChangeKind::FormulaChanged => ("formula_changed", "Formula"),
                    ChangeKind::FormatChanged => ("format_changed", "Format"),
                };
                out.push_str(&format!("<span class=\"badge {class}\">{label}</span>"));
            }
            out.push_str("</span></td>");
        }
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\0' => out.push_str("&#xfffd;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_handles_both_attribute_and_text_contexts() {
        assert_eq!(
            escape("<script a=\"x\" b='y'>&\0"),
            "&lt;script a=&quot;x&quot; b=&#39;y&#39;&gt;&amp;&#xfffd;"
        );
    }
    #[test]
    fn empty_report_is_offline_and_escapes_file_metadata() {
        let report = Report {
            schema: "sheetdiff/v1",
            old: FileInfo {
                path: "<script>alert(1)</script>".into(),
                sha256: "\"unsafe\"".into(),
            },
            new: FileInfo {
                path: "new.csv".into(),
                sha256: "abc".into(),
            },
            summary: Summary::default(),
            sheets: vec![],
            warnings: vec![],
        };
        let html = render(&report);
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script"));
        assert!(html.contains("default-src 'none'; style-src 'unsafe-inline'"));
        assert!(html.contains("No differences found"));
    }
}
