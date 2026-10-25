//! Regression coverage for comparison policy, row alignment, and XLSX metadata.
use assert_cmd::Command;
use predicates::prelude::*;
use rust_xlsxwriter::{ExcelDateTime, Format, FormatAlign, Workbook};
use sheetdiff::{
    compare_files,
    model::{ChangeKind, Options, SheetStatus},
    reader,
};
use std::{fs, path::Path};
use tempfile::{tempdir, TempDir};

fn cli() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("sheetdiff"))
}

fn save(path: &Path, populate: impl FnOnce(&mut Workbook)) {
    let mut book = Workbook::new();
    populate(&mut book);
    book.save(path).unwrap();
}

fn csv_pair(old: &str, new: &str) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.csv");
    let b = dir.path().join("new.csv");
    fs::write(&a, old).unwrap();
    fs::write(&b, new).unwrap();
    (dir, a, b)
}

#[test]
fn formulas_and_skip_formula() {
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
            .write_formula(0, 0, "=SUM(B1:B3)")
            .unwrap();
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.formulas_changed, 1);
    assert!(report.sheets[0].changes.iter().any(|c| {
        c.kind == ChangeKind::FormulaChanged
            && c.old.as_ref().unwrap().formula.as_deref() == Some("=SUM(B1:B2)")
            && c.new.as_ref().unwrap().formula.as_deref() == Some("=SUM(B1:B3)")
    }));
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("[F]"));
    cli().args([&a, &b]).arg("--skip-formula").assert().code(0);
}

#[test]
fn all_required_format_properties_and_skip_format() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    let formats = [
        Format::new().set_bold(),
        Format::new().set_italic(),
        Format::new().set_background_color("#FFCC00"),
        Format::new().set_font_name("Courier New"),
        Format::new().set_num_format("0.00"),
        Format::new().set_align(FormatAlign::Center),
    ];
    save(&a, |book| {
        let sheet = book.add_worksheet();
        for col in 0..formats.len() {
            sheet.write_number(0, col as u16, 123.0).unwrap();
        }
    });
    save(&b, |book| {
        let sheet = book.add_worksheet();
        for (col, format) in formats.iter().enumerate() {
            sheet
                .write_number_with_format(0, col as u16, 123.0, format)
                .unwrap();
        }
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.formats_changed, formats.len());
    assert_eq!(report.summary.cells_modified, 0);
    for col in 0..formats.len() {
        assert!(report.sheets[0].changes.iter().any(|c| {
            c.kind == ChangeKind::FormatChanged && c.cell == sheetdiff::model::a1(0, col)
        }));
    }
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("[FMT]"));
    cli().args([&a, &b]).arg("--skip-format").assert().code(0);
}

#[test]
fn workbook_local_style_ids_do_not_create_false_differences() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    let bold = Format::new().set_bold();
    let italic = Format::new().set_italic();
    save(&a, |book| {
        let sheet = book.add_worksheet();
        sheet.write_string_with_format(0, 0, "bold", &bold).unwrap();
        sheet
            .write_string_with_format(0, 1, "italic", &italic)
            .unwrap();
    });
    save(&b, |book| {
        let sheet = book.add_worksheet();
        sheet
            .write_string_with_format(0, 1, "italic", &italic)
            .unwrap();
        sheet.write_string_with_format(0, 0, "bold", &bold).unwrap();
    });
    cli().args([&a, &b]).assert().code(0);
}

#[test]
fn inherited_row_and_column_formats_are_compared() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        let sheet = book.add_worksheet();
        sheet.write_string(0, 0, "row format").unwrap();
        sheet.write_string(1, 1, "column format").unwrap();
    });
    save(&b, |book| {
        let sheet = book.add_worksheet();
        sheet.set_row_format(0, &Format::new().set_bold()).unwrap();
        sheet
            .set_column_format(1, &Format::new().set_italic())
            .unwrap();
        sheet.write_string(0, 0, "row format").unwrap();
        sheet.write_string(1, 1, "column format").unwrap();
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.formats_changed, 2);
    cli().args([&a, &b]).arg("--skip-format").assert().code(0);
}

#[test]
fn row_insertion_is_one_block_and_does_not_modify_shifted_rows() {
    let (_dir, a, b) = csv_pair(
        "name,value\nalpha,10\nomega,30\n",
        "name,value\nalpha,10\nfresh,20\nomega,30\n",
    );
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.rows_added, 1);
    assert_eq!(report.summary.rows_removed, 0);
    assert_eq!(report.summary.cells_modified, 0);
    assert_eq!(report.sheets[0].row_changes.len(), 1);
    assert_eq!(report.sheets[0].row_changes[0].row, 3);
    assert!(report.sheets[0].changes.iter().all(|c| c.row_change));
    let output = cli()
        .args([&a, &b])
        .arg("--context=0")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(text.matches("[+]").count(), 1, "{text}");
    assert!(text.contains("fresh"));
}

#[test]
fn row_removal_is_one_block_and_does_not_modify_shifted_rows() {
    let (_dir, a, b) = csv_pair(
        "name,value\nalpha,10\nobsolete,20\nomega,30\n",
        "name,value\nalpha,10\nomega,30\n",
    );
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.rows_removed, 1);
    assert_eq!(report.summary.cells_modified, 0);
    assert_eq!(report.sheets[0].row_changes[0].row, 3);
    let output = cli()
        .args([&a, &b])
        .arg("--context=0")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(text.matches("[-]").count(), 1, "{text}");
}

#[test]
fn modification_after_insertion_keeps_both_coordinates() {
    let (_dir, a, b) = csv_pair(
        "name,value\nalpha,10\nbeta,20\nomega,30\n",
        "name,value\nfresh,5\nalpha,10\nbeta,21\nomega,30\n",
    );
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.rows_added, 1);
    assert_eq!(report.summary.cells_modified, 1);
    let change = report.sheets[0]
        .changes
        .iter()
        .find(|c| c.kind == ChangeKind::Modified)
        .unwrap();
    assert_eq!(change.old_cell.as_deref(), Some("B3"));
    assert_eq!(change.new_cell.as_deref(), Some("B4"));
}

#[test]
fn sheets_added_removed_and_filtered() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet()
            .set_name("Keep")
            .unwrap()
            .write_string(0, 0, "same")
            .unwrap();
        book.add_worksheet()
            .set_name("Gone")
            .unwrap()
            .write_string(0, 0, "old")
            .unwrap();
    });
    save(&b, |book| {
        book.add_worksheet()
            .set_name("Keep")
            .unwrap()
            .write_string(0, 0, "same")
            .unwrap();
        book.add_worksheet()
            .set_name("New")
            .unwrap()
            .write_string(0, 0, "new")
            .unwrap();
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.sheets_added, 1);
    assert_eq!(report.summary.sheets_removed, 1);
    assert_eq!(
        report
            .sheets
            .iter()
            .find(|s| s.name == "New")
            .unwrap()
            .status,
        SheetStatus::Added
    );
    assert!(report
        .sheets
        .iter()
        .find(|s| s.name == "New")
        .unwrap()
        .changes
        .iter()
        .all(|c| c.kind == ChangeKind::Added));
    assert!(report
        .sheets
        .iter()
        .find(|s| s.name == "Gone")
        .unwrap()
        .changes
        .iter()
        .all(|c| c.kind == ChangeKind::Removed));
    cli()
        .args([&a, &b])
        .args(["--sheet", "Keep"])
        .assert()
        .code(0);
    cli()
        .args([&a, &b])
        .args(["--sheet", "Gone", "--sheet", "New"])
        .assert()
        .code(1);
    cli()
        .args([&a, &b])
        .args(["--sheet", "Unknown"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("sheet not found"));
}

#[test]
fn empty_sheet_addition_is_still_a_difference() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet().set_name("Keep").unwrap();
    });
    save(&b, |book| {
        book.add_worksheet().set_name("Keep").unwrap();
        book.add_worksheet().set_name("Empty").unwrap();
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.sheets_added, 1);
    assert!(report.has_changes());
    cli().args([&a, &b]).assert().code(1);
    cli().args([&b, &a]).assert().code(1);
}

#[test]
fn absolute_epsilon_for_xlsx_and_csv_numbers() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet().write_number(0, 0, 1.0).unwrap();
    });
    save(&b, |book| {
        book.add_worksheet()
            .write_number(0, 0, 1.0000000001)
            .unwrap();
    });
    cli().args([&a, &b]).assert().code(0);
    cli()
        .args([&a, &b])
        .args(["--epsilon", "0"])
        .assert()
        .code(1);
    let (_dir, a, b) = csv_pair("n\n1.0\n", "n\n1.0000000001\n");
    cli().args([&a, &b]).assert().code(0);
    cli()
        .args([&a, &b])
        .args(["--epsilon", "0"])
        .assert()
        .code(1);
}

#[test]
fn epsilon_does_not_collapse_text_or_leading_zero_identifiers() {
    let (_dir, a, b) = csv_pair("id\n001\n", "id\n1\n");
    cli()
        .args([&a, &b])
        .args(["--epsilon", "100"])
        .assert()
        .code(1);
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet().write_string(0, 0, "1").unwrap();
    });
    save(&b, |book| {
        book.add_worksheet().write_string(0, 0, "1.0").unwrap();
    });
    cli().args([&a, &b]).assert().code(1);
}

#[test]
fn csv_no_header_and_custom_delimiter() {
    let (_dir, a, b) = csv_pair("项目;金额\n测试;10\n", "项目;金额\n测试;11\n");
    let options = Options {
        no_header: true,
        delimiter: b';',
        ..Options::default()
    };
    let report = compare_files(&a, &b, &options).unwrap();
    assert_eq!(report.summary.cells_modified, 1);
    let change = &report.sheets[0].changes[0];
    assert_eq!(change.cell, "B2");
    assert_eq!(change.column_name, None);
    cli()
        .args([&a, &b])
        .args(["--no-header", "--delimiter", ";"])
        .assert()
        .code(1);
}

#[test]
fn duplicate_headers_get_unique_labels_and_are_displayed() {
    let (_dir, a, b) = csv_pair(
        "金额,金额,金额#2\n10,20,30\n",
        "金额,金额,金额#2\n10,21,30\n",
    );
    let workbook = reader::read(&a, &Options::default()).unwrap();
    let columns = &workbook.sheets[0].columns;
    assert_eq!(columns[0], "金额");
    assert_eq!(columns[1], "金额#2");
    assert_eq!(
        columns
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(
        report.sheets[0].changes[0].column_name.as_deref(),
        Some("金额#2")
    );
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("金额#2"));
}

#[test]
fn ragged_csv_distinguishes_explicit_empty_from_missing() {
    let (_dir, a, b) = csv_pair("a,b\nleft,\n", "a,b\nleft\n");
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    let change = report.sheets[0]
        .changes
        .iter()
        .find(|c| c.cell == "B2")
        .unwrap();
    assert_eq!(change.kind, ChangeKind::Removed);
    assert!(change.old.as_ref().unwrap().value.is_null());
    assert!(change.new.is_none());
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("(empty)").and(predicate::str::contains("(missing)")));
}

#[test]
fn xlsx_explicit_blank_is_distinct_from_absent_cell() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        let sheet = book.add_worksheet();
        sheet.write_string(0, 0, "anchor").unwrap();
        sheet.write_blank(0, 1, &Format::new().set_bold()).unwrap();
    });
    save(&b, |book| {
        book.add_worksheet().write_string(0, 0, "anchor").unwrap();
    });
    let options = Options {
        skip_format: true,
        ..Options::default()
    };
    let report = compare_files(&a, &b, &options).unwrap();
    assert!(report.has_changes());
    let change = report.sheets[0]
        .changes
        .iter()
        .find(|c| c.cell == "B1")
        .unwrap();
    assert!(change.old.as_ref().unwrap().value.is_null());
    assert!(change.new.is_none());
}

#[test]
fn blank_to_value_and_value_to_blank_are_addition_and_removal() {
    let (_dir, a, b) = csv_pair("name,value\nrow,\n", "name,value\nrow,filled\n");
    let forward = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(forward.summary.cells_added, 1);
    assert_eq!(forward.summary.cells_modified, 0);
    assert_eq!(forward.sheets[0].changes[0].kind, ChangeKind::Added);
    let reverse = compare_files(&b, &a, &Options::default()).unwrap();
    assert_eq!(reverse.summary.cells_removed, 1);
    assert_eq!(reverse.summary.cells_modified, 0);
    assert_eq!(reverse.sheets[0].changes[0].kind, ChangeKind::Removed);
}

#[test]
fn sparse_xlsx_positions_keep_original_a1_coordinates() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet().write_string(99, 26, "before").unwrap();
    });
    save(&b, |book| {
        book.add_worksheet().write_string(99, 26, "after").unwrap();
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.sheets[0].changes[0].cell, "AA100");
}

#[test]
fn unicode_values_are_preserved() {
    let (_dir, a, b) = csv_pair("项目\n研发🚀\n", "项目\n开发👩‍💻\n");
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("研发🚀").and(predicate::str::contains("开发👩‍💻")));
}

#[test]
fn dates_are_iso_and_elapsed_time_is_a_duration() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    for (path, day) in [(&a, 24), (&b, 25)] {
        save(path, |book| {
            let sheet = book.add_worksheet();
            let date = ExcelDateTime::from_ymd(2026, 10, day)
                .unwrap()
                .and_hms(12, 30, 0)
                .unwrap();
            sheet
                .write_datetime_with_format(
                    0,
                    0,
                    &date,
                    &Format::new().set_num_format("yyyy-mm-dd hh:mm:ss"),
                )
                .unwrap();
            sheet
                .write_number_with_format(1, 0, 1.5, &Format::new().set_num_format("[h]:mm:ss"))
                .unwrap();
        });
    }
    let old = reader::read(&a, &Options::default()).unwrap();
    assert!(old.sheets[0].rows[&0][&0]
        .formatted
        .starts_with("2026-10-24T12:30:00"));
    let duration = &old.sheets[0].rows[&1][&0].formatted;
    assert!(
        duration.starts_with('P'),
        "duration should be ISO 8601: {duration}"
    );
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("2026-10-25T12:30:00"));
}

#[test]
fn merges_are_annotated_and_merge_only_changes_can_be_skipped() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("old.xlsx");
    let b = dir.path().join("new.xlsx");
    save(&a, |book| {
        book.add_worksheet()
            .merge_range(0, 0, 1, 1, "标题", &Format::new())
            .unwrap();
    });
    save(&b, |book| {
        book.add_worksheet()
            .merge_range(0, 0, 1, 2, "标题", &Format::new())
            .unwrap();
    });
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert!(report.has_changes());
    assert!(report.sheets[0].merged_old.contains(&"A1:B2".into()));
    assert!(report.sheets[0].merged_new.contains(&"A1:C2".into()));
    cli().args([&a, &b]).assert().code(1).stdout(
        predicate::str::contains("(merged A1:B2)").and(predicate::str::contains("(merged A1:C2)")),
    );
    // Merge ranges also write physical blank cells; those remain distinguishable.
    // Compare equal cell populations to isolate layout policy below.
    save(&a, |book| {
        let sheet = book.add_worksheet();
        sheet
            .merge_range(0, 0, 0, 1, "标题", &Format::new())
            .unwrap();
        sheet.write_blank(1, 0, &Format::new()).unwrap();
        sheet.write_blank(1, 1, &Format::new()).unwrap();
    });
    save(&b, |book| {
        let sheet = book.add_worksheet();
        sheet
            .merge_range(0, 0, 1, 1, "标题", &Format::new())
            .unwrap();
    });
    cli().args([&a, &b]).arg("--skip-format").assert().code(0);
}

#[test]
fn ansi_color_policy_and_non_tty_auto() {
    let (_dir, a, b) = csv_pair("n\n1\n", "n\n2\n");
    cli()
        .args([&a, &b])
        .args(["--color", "always"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\u{1b}["));
    cli()
        .args([&a, &b])
        .args(["--color", "never"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\u{1b}[").not());
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\u{1b}[").not());
}

#[test]
fn context_flag_controls_unchanged_neighbor_rows() {
    let (_dir, a, b) = csv_pair(
        "n\nneighbor-before\nold\nneighbor-after\n",
        "n\nneighbor-before\nnew\nneighbor-after\n",
    );
    let without = Options {
        context: 0,
        ..Options::default()
    };
    assert!(compare_files(&a, &b, &without).unwrap().sheets[0]
        .context
        .is_empty());
    let with = Options {
        context: 1,
        ..Options::default()
    };
    assert_eq!(
        compare_files(&a, &b, &with).unwrap().sheets[0]
            .context
            .len(),
        2
    );
    cli()
        .args([&a, &b])
        .arg("--context=0")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("neighbor-before").not());
    cli()
        .args([&a, &b])
        .arg("--context=1")
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("neighbor-before")
                .and(predicate::str::contains("neighbor-after")),
        );
}

#[test]
fn invalid_policy_arguments_return_two() {
    for epsilon in ["NaN", "inf", "-1"] {
        cli()
            .args(["old.csv", "new.csv", "--epsilon", epsilon])
            .assert()
            .code(2);
    }
    for delimiter in ["", "::", "，"] {
        cli()
            .args(["old.csv", "new.csv", "--delimiter", delimiter])
            .assert()
            .code(2);
    }
}
