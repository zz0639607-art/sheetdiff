//! Stable report contract, offline HTML, and safe CLI report destinations.
use assert_cmd::Command;
use predicates::prelude::*;
use rust_xlsxwriter::{Format, Workbook};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sheetdiff::{compare_files, model::Options};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};

fn cli() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("sheetdiff"))
}

fn save(path: &Path, populate: impl FnOnce(&mut Workbook)) {
    let mut book = Workbook::new();
    populate(&mut book);
    book.save(path).unwrap();
}

fn csv_pair(old: &str, new: &str) -> (TempDir, PathBuf, PathBuf) {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.csv");
    let b = dir.path().join("new.csv");
    fs::write(&a, old).unwrap();
    fs::write(&b, new).unwrap();
    (dir, a, b)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn json_schema_preserves_identity_types_unicode_and_presence() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        let sheet = book.add_worksheet();
        sheet.set_name("预算表").unwrap();
        sheet.write_number(0, 0, 123.0).unwrap();
        sheet.write_boolean(0, 1, false).unwrap();
        sheet.write_blank(0, 2, &Format::new().set_bold()).unwrap();
        sheet.write_string(0, 3, "旧项目").unwrap();
    });
    save(&b, |book| {
        let sheet = book.add_worksheet();
        sheet.set_name("预算表").unwrap();
        sheet.write_number(0, 0, 456.0).unwrap();
        sheet.write_boolean(0, 1, true).unwrap();
        sheet
            .write_blank(0, 2, &Format::new().set_italic())
            .unwrap();
        sheet.write_string(0, 3, "新项目🚀").unwrap();
        sheet.write_string(0, 4, "added").unwrap();
    });
    let output = dir.path().join("report.json");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&output)
        .assert()
        .code(1);
    let raw = fs::read_to_string(&output).unwrap();
    assert!(raw.contains("预算表"));
    assert!(raw.contains("新项目🚀"));
    let value = read_json(&output);
    assert_eq!(value["schema"], "sheetdiff/v1");
    assert_eq!(value["old"]["path"], a.to_string_lossy().as_ref());
    assert_eq!(value["new"]["path"], b.to_string_lossy().as_ref());
    for (side, path) in [("old", &a), ("new", &b)] {
        let expected = format!("{:x}", Sha256::digest(fs::read(path).unwrap()));
        assert_eq!(value[side]["sha256"], expected);
    }
    let summary = &value["summary"];
    for field in [
        "sheets_changed",
        "cells_modified",
        "cells_added",
        "cells_removed",
        "rows_added",
        "rows_removed",
        "sheets_added",
        "sheets_removed",
    ] {
        assert!(
            summary[field].is_u64(),
            "missing numeric summary field {field}"
        );
    }
    let changes = value["sheets"][0]["changes"].as_array().unwrap();
    let find = |cell: &str| changes.iter().find(|c| c["cell"] == cell).unwrap();
    assert_eq!(find("A1")["kind"], "modified");
    assert_eq!(find("A1")["old"]["value"].as_f64(), Some(123.0));
    assert_eq!(find("A1")["new"]["value"].as_f64(), Some(456.0));
    assert_eq!(find("B1")["old"]["value"], false);
    assert_eq!(find("B1")["new"]["value"], true);
    assert!(find("C1")["old"].is_object());
    assert!(find("C1")["old"]["value"].is_null());
    assert!(find("E1")["old"].is_null());
    assert!(find("A1")["old"]["formula"].is_null());
    assert!(find("A1")["old"]["formatted"].is_string());
}

#[test]
fn unchanged_inputs_still_generate_both_reports_and_exit_zero() {
    let (dir, a, _b) = csv_pair("项目,金额\n预算,123\n", "unused");
    let json = dir.path().join("identical.json");
    let html = dir.path().join("identical.html");
    cli()
        .args([&a, &a])
        .arg("--json")
        .arg(&json)
        .arg("--html")
        .arg(&html)
        .assert()
        .code(0);
    assert_eq!(read_json(&json)["summary"]["sheets_changed"], 0);
    let html = fs::read_to_string(html).unwrap();
    assert!(html.contains("<details"));
    assert!(html.contains("CSV"));
}

#[test]
fn summary_only_limits_terminal_output_but_writes_full_reports() {
    let (dir, a, b) = csv_pair("name,value\nold,1\n", "name,value\nnew,2\n");
    let json = dir.path().join("report.json");
    let html = dir.path().join("report.html");
    let output = cli()
        .args([&a, &b])
        .arg("--summary-only")
        .arg("--json")
        .arg(&json)
        .arg("--html")
        .arg(&html)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(text.starts_with("Summary:"));
    assert_eq!(text.lines().count(), 1);
    assert!(!text.contains("Sheet:"));
    assert!(!text.contains("--- a:"));
    assert!(!text.contains("[M]"));
    assert_eq!(
        read_json(&json)["sheets"][0]["changes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(fs::read_to_string(&html).unwrap().contains("old"));
}

#[test]
fn html_is_standalone_with_css_tabs_hover_details_and_old_new_highlights() {
    let (dir, a, b) = csv_pair("name,value\n预算,123\n", "name,value\n预算,456\n");
    let html_path = dir.path().join("report.html");
    cli()
        .args([&a, &b])
        .arg("--html")
        .arg(&html_path)
        .assert()
        .code(1);
    let html = fs::read_to_string(html_path).unwrap();
    assert!(html.to_ascii_lowercase().contains("<!doctype html>"));
    assert!(html.contains("<style>"));
    assert!(html.contains("</style>"));
    assert!(html.contains("type=\"radio\""));
    assert!(html.contains(":checked"));
    assert!(html.contains("title=\""));
    assert!(html.contains("123"));
    assert!(html.contains("456"));
    assert!(html.contains("<table"));
    assert!(!html.contains("<script"));
    assert!(!html.contains("<link"));
    assert!(!html.contains("src=\"http"));
    assert!(!html.contains("href=\"http"));
    let old_hash = format!("{:x}", Sha256::digest(fs::read(&a).unwrap()));
    let new_hash = format!("{:x}", Sha256::digest(fs::read(&b).unwrap()));
    assert!(html.contains(&old_hash));
    assert!(html.contains(&new_hash));
    // Named old/new classes must have visible contrasting red/green CSS colors.
    // The precise palette and class naming remain a renderer design choice.
    assert!(html.contains("background") && html.contains("color"));
}

#[test]
fn html_escapes_cell_text_sheet_names_and_hover_attributes() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    let old = "<img src=x onerror=alert(1)> & 'old'";
    let new = "\"quoted\" & <script>alert(1)</script> 中文";
    for (path, text) in [(&a, old), (&b, new)] {
        save(path, |book| {
            book.add_worksheet()
                .set_name("<unsafe & sheet>")
                .unwrap()
                .write_string(0, 0, text)
                .unwrap();
        });
    }
    let html_path = dir.path().join("safe.html");
    let json_path = dir.path().join("safe.json");
    cli()
        .args([&a, &b])
        .arg("--html")
        .arg(&html_path)
        .arg("--json")
        .arg(&json_path)
        .assert()
        .code(1);
    let html = fs::read_to_string(html_path).unwrap();
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img "));
    assert!(!html.contains("<unsafe"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&lt;unsafe &amp; sheet&gt;"));
    assert!(html.contains("&quot;quoted&quot;"));
    let value = read_json(&json_path);
    assert_eq!(value["sheets"][0]["name"], "<unsafe & sheet>");
    assert_eq!(value["sheets"][0]["changes"][0]["old"]["value"], old);
    assert_eq!(value["sheets"][0]["changes"][0]["new"]["value"], new);
}

#[test]
fn json_records_formula_and_format_events_independently() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet()
            .write_formula(0, 0, "=SUM(B1:B2)")
            .unwrap();
    });
    save(&b, |book| {
        book.add_worksheet()
            .write_formula_with_format(0, 0, "=SUM(B1:B3)", &Format::new().set_bold())
            .unwrap();
    });
    let path = dir.path().join("report.json");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&path)
        .assert()
        .code(1);
    let value = read_json(&path);
    let changes = value["sheets"][0]["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().any(|c| c["kind"] == "formula_changed"));
    assert!(changes.iter().any(|c| c["kind"] == "format_changed"));
    for change in changes {
        assert_eq!(change["cell"], "A1");
        assert_eq!(change["old"]["formula"], "=SUM(B1:B2)");
        assert_eq!(change["new"]["formula"], "=SUM(B1:B3)");
    }
}

#[test]
fn json_preserves_old_and_new_coordinates_after_row_insertion() {
    let (dir, a, b) = csv_pair(
        "name,value\nalpha,10\nbeta,20\nomega,30\n",
        "name,value\nfresh,5\nalpha,10\nbeta,21\nomega,30\n",
    );
    let path = dir.path().join("report.json");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&path)
        .assert()
        .code(1);
    let value = read_json(&path);
    let changes = value["sheets"][0]["changes"].as_array().unwrap();
    let modified = changes.iter().find(|c| c["kind"] == "modified").unwrap();
    assert_eq!(modified["old_cell"], "B3");
    assert_eq!(modified["new_cell"], "B4");
    assert_eq!(value["summary"]["rows_added"], 1);
    assert_eq!(value["sheets"][0]["row_changes"][0]["row"], 2);
}

#[test]
fn changed_sheets_get_tables_and_unchanged_sheets_get_a_fold() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet()
            .set_name("Stable")
            .unwrap()
            .write_string(0, 0, "same")
            .unwrap();
        book.add_worksheet()
            .set_name("Gone")
            .unwrap()
            .write_string(0, 0, "removed")
            .unwrap();
    });
    save(&b, |book| {
        book.add_worksheet()
            .set_name("Stable")
            .unwrap()
            .write_string(0, 0, "same")
            .unwrap();
        book.add_worksheet()
            .set_name("Added")
            .unwrap()
            .write_string(0, 0, "added")
            .unwrap();
    });
    let json_path = dir.path().join("report.json");
    let html_path = dir.path().join("report.html");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&json_path)
        .arg("--html")
        .arg(&html_path)
        .assert()
        .code(1);
    let value = read_json(&json_path);
    let sheets = value["sheets"].as_array().unwrap();
    for (name, status) in [
        ("Stable", "unchanged"),
        ("Gone", "removed"),
        ("Added", "added"),
    ] {
        assert_eq!(
            sheets.iter().find(|s| s["name"] == name).unwrap()["status"],
            status
        );
    }
    let html = fs::read_to_string(html_path).unwrap();
    let fold = html.find("<details").unwrap();
    assert!(!html[..fold].contains("Stable"));
    assert!(html[fold..].contains("Stable"));
    assert!(html[..fold].contains("Added"));
    assert!(html[..fold].contains("Gone"));
}

#[test]
fn report_destinations_cannot_overwrite_either_input() {
    let (_dir, a, b) = csv_pair("name\nold\n", "name\nnew\n");
    let before_a = fs::read(&a).unwrap();
    let before_b = fs::read(&b).unwrap();
    for flag in ["--json", "--html"] {
        for input in [&a, &b] {
            cli()
                .args([&a, &b])
                .arg(flag)
                .arg(input)
                .assert()
                .code(2)
                .stderr(predicate::str::is_empty().not());
            assert_eq!(fs::read(&a).unwrap(), before_a);
            assert_eq!(fs::read(&b).unwrap(), before_b);
        }
    }
}

#[test]
fn report_destinations_cannot_alias_each_other() {
    let (dir, a, b) = csv_pair("name\nold\n", "name\nnew\n");
    let output = dir.path().join("report");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&output)
        .arg("--html")
        .arg(&output)
        .assert()
        .code(2);
    assert!(!output.exists());
    fs::write(&output, "existing report").unwrap();
    let alias = dir.path().join(".").join("report");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&output)
        .arg("--html")
        .arg(alias)
        .assert()
        .code(2);
    assert_eq!(fs::read_to_string(&output).unwrap(), "existing report");
}

#[test]
fn invalid_report_parent_and_directory_destination_return_two() {
    let (dir, a, b) = csv_pair("name\nold\n", "name\nnew\n");
    let missing_parent = dir.path().join("missing").join("report.json");
    cli()
        .args([&a, &b])
        .arg("--json")
        .arg(&missing_parent)
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty().not());
    assert!(!missing_parent.exists());
    cli()
        .args([&a, &b])
        .arg("--html")
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty().not());
}

#[test]
fn terminal_truncation_never_truncates_json_events() {
    let count = 1100usize;
    let old = format!(
        "n\n{}",
        (0..count).map(|n| format!("{n}\n")).collect::<String>()
    );
    let new = format!(
        "n\n{}",
        (0..count)
            .map(|n| format!("{}\n", n + 10000))
            .collect::<String>()
    );
    let (dir, a, b) = csv_pair(&old, &new);
    let json_path = dir.path().join("large.json");
    let output = cli()
        .args([&a, &b])
        .arg("--context=0")
        .arg("--json")
        .arg(&json_path)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("Display truncated"));
    let value = read_json(&json_path);
    assert_eq!(value["summary"]["cells_modified"], json!(count));
    assert_eq!(
        value["sheets"][0]["changes"].as_array().unwrap().len(),
        count
    );
    assert!(value["sheets"][0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["cell"] == "A1101"));
}

#[test]
fn public_renderers_work_without_cli_or_filesystem_output() {
    let (_dir, a, b) = csv_pair("n\n1\n", "n\n2\n");
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    let json = sheetdiff::render::json::render(&report).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["schema"], "sheetdiff/v1");
    let html = sheetdiff::render::html::render(&report);
    assert!(html.contains("<style>"));
    assert!(html.contains("<table"));
}
