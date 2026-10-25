//! Reproducible, synthetic demo workbooks generated using a dev-only writer.
use assert_cmd::Command;
use predicates::prelude::*;
use rust_xlsxwriter::{DocProperties, ExcelDateTime, Format, Formula, Workbook, XlsxError};
use sheetdiff::{
    compare_files,
    model::{ChangeKind, Options},
};
use std::{fs, path::Path};

/// Write all checked-in fixtures. Ordinary tests only read these files.
fn generate(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(directory)?;
    budget(&directory.join("budget_v1.xlsx"), false)?;
    budget(&directory.join("budget_v2.xlsx"), true)?;
    fs::write(
        directory.join("budget_v1.csv"),
        "项目,预算,说明\n研发费用,123,中文内容\n设计费用,200,保持不变\n旧项目,999,删除项目\n运营费用,300,保持不变\n测试费用,400,保持不变\n交付费用,700,保持不变\n",
    )?;
    fs::write(
        directory.join("budget_v2.csv"),
        "项目,预算,说明\n研发费用,456,中文内容\n设计费用,200,保持不变\n运营费用,300,保持不变\n测试费用,400,保持不变\n新项目,888,新增项目\n交付费用,700,保持不变\n",
    )?;
    fs::write(directory.join("no_header_v1.csv"), "研发;123\n设计;200\n")?;
    fs::write(directory.join("no_header_v2.csv"), "研发;456\n设计;200\n")?;
    fs::write(
        directory.join("duplicate_headers_v1.csv"),
        "金额,金额,金额#2\n10,20,30\n",
    )?;
    fs::write(
        directory.join("duplicate_headers_v2.csv"),
        "金额,金额,金额#2\n10,21,30\n",
    )?;
    Ok(())
}

fn budget(path: &Path, revised: bool) -> Result<(), XlsxError> {
    let mut book = Workbook::new();
    let created = ExcelDateTime::from_ymd(2026, 1, 1)?;
    book.set_properties(
        &DocProperties::new()
            .set_author("sheetdiff contributors")
            .set_title("Synthetic sheetdiff budget fixture")
            .set_creation_datetime(&created),
    );
    let sheet = book.add_worksheet();
    sheet.set_name("预算表")?;
    sheet.set_column_width(0, 20)?;
    sheet.set_column_width(1, 20)?;
    sheet.set_column_width(2, 24)?;
    sheet.set_column_width(3, 20)?;
    sheet.set_column_width(4, 15)?;
    for (col, header) in ["项目", "预算", "公式", "说明", "计算金额"]
        .iter()
        .enumerate()
    {
        if col == 3 && revised {
            sheet.write_string_with_format(0, col as u16, *header, &Format::new().set_bold())?;
        } else {
            sheet.write_string(0, col as u16, *header)?;
        }
    }
    sheet.write_string(1, 0, "研发费用")?;
    sheet.write_number(1, 1, if revised { 456.0 } else { 123.0 })?;
    sheet.write_string(1, 3, "中文内容 / Unicode 🚀")?;
    sheet.write_number(1, 4, 10.0)?;
    sheet.write_string(2, 0, "合计")?;
    let formula = if revised {
        "=SUM(E2:E5)"
    } else {
        "=SUM(E2:E4)"
    };
    let cached = if revised { "110" } else { "60" };
    sheet.write_formula(2, 2, Formula::new(formula).set_result(cached))?;
    sheet.write_number(2, 4, 20.0)?;
    sheet.write_string(3, 0, "设计费用")?;
    sheet.write_number(3, 1, 200.0)?;
    sheet.write_number(3, 4, 30.0)?;
    // Equal rows on both sides of each gap make the insertion/deletion
    // unambiguous to the LCS alignment. The gaps are separated by anchors.
    let rows: Vec<(u32, &str, f64, f64)> = if revised {
        vec![
            (4, "运营费用", 300.0, 50.0),
            (5, "测试费用", 400.0, 60.0),
            (6, "新项目", 888.0, 80.0),
            (7, "交付费用", 700.0, 70.0),
        ]
    } else {
        vec![
            (4, "旧项目", 999.0, 40.0),
            (5, "运营费用", 300.0, 50.0),
            (6, "测试费用", 400.0, 60.0),
            (7, "交付费用", 700.0, 70.0),
        ]
    };
    for (row, name, amount, calculation) in rows {
        sheet.write_string(row, 0, name)?;
        sheet.write_number(row, 1, amount)?;
        sheet.write_number(row, 4, calculation)?;
        if name == "新项目" {
            sheet.write_blank(row, 2, &Format::new())?;
        }
        if name == "旧项目" {
            sheet.write_number(row, 2, 12.5)?;
        }
    }
    sheet.write_string(8, 0, "发布日期")?;
    let date = ExcelDateTime::from_ymd(2026, 10, if revised { 25 } else { 24 })?;
    sheet.write_datetime_with_format(8, 1, &date, &Format::new().set_num_format("yyyy-mm-dd"))?;
    sheet.write_string(9, 0, "备注")?;
    sheet.write_string(9, 1, "合成数据，仅用于测试与演示")?;

    if revised {
        let added = book.add_worksheet();
        added.set_name("新增表")?;
        for row in 0..10 {
            added.write_string(row, 0, format!("新项目 {}", row + 1))?;
            added.write_number(row, 1, 100.0 + f64::from(row))?;
            added.write_string(row, 2, "新增内容")?;
        }
    } else {
        let removed = book.add_worksheet();
        removed.set_name("历史表")?;
        removed.write_string(0, 0, "历史项目")?;
        removed.write_string(0, 1, "归档预算")?;
        removed.write_string(1, 0, "已结束项目")?;
        removed.write_number(1, 1, 1000.0)?;
    }
    let reference = book.add_worksheet();
    reference.set_name("不变表")?;
    reference.merge_range(0, 0, 1, 1, "参考信息（保持不变）", &Format::new())?;
    reference.write_string(3, 0, "Currency")?;
    reference.write_string(3, 1, "CNY")?;
    book.save(path)?;
    Ok(())
}

#[test]
#[ignore = "explicitly rewrites the checked-in synthetic demo fixtures"]
fn regenerate() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    generate(&directory).unwrap();
}

#[test]
fn checked_in_budget_demonstrates_every_diff_layer() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let a = directory.join("budget_v1.xlsx");
    let b = directory.join("budget_v2.xlsx");
    let report = compare_files(&a, &b, &Options::default()).unwrap();
    assert_eq!(report.summary.sheets_added, 1);
    assert_eq!(report.summary.sheets_removed, 1);
    assert_eq!(report.summary.sheets_changed, 3);
    let budget = report.sheets.iter().find(|s| s.name == "预算表").unwrap();
    assert_eq!(
        budget
            .row_changes
            .iter()
            .filter(|r| r.kind == ChangeKind::Added)
            .count(),
        1
    );
    assert_eq!(
        budget
            .row_changes
            .iter()
            .filter(|r| r.kind == ChangeKind::Removed)
            .count(),
        1
    );
    assert!(budget.changes.iter().any(|c| c.cell == "B2"
        && c.kind == ChangeKind::Modified
        && c.old.as_ref().unwrap().value.as_f64() == Some(123.0)
        && c.new.as_ref().unwrap().value.as_f64() == Some(456.0)));
    assert!(budget.changes.iter().any(|c| c.cell == "C3"
        && c.kind == ChangeKind::FormulaChanged
        && c.old.as_ref().unwrap().formula.as_deref() == Some("=SUM(E2:E4)")
        && c.new.as_ref().unwrap().formula.as_deref() == Some("=SUM(E2:E5)")));
    assert!(budget
        .changes
        .iter()
        .any(|c| c.cell == "D1" && c.kind == ChangeKind::FormatChanged));
    assert!(budget
        .changes
        .iter()
        .any(|c| c.cell == "B9" && c.new.as_ref().unwrap().formatted.starts_with("2026-10-25")));
    Command::new(assert_cmd::cargo::cargo_bin!("sheetdiff"))
        .args([&a, &b])
        .arg("--context=0")
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("[M]")
                .and(predicate::str::contains("[F]"))
                .and(predicate::str::contains("[FMT]"))
                .and(predicate::str::contains("Row 5 (removed)"))
                .and(predicate::str::contains("Row 7 (added)")),
        );
}
