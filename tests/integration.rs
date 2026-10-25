mod common;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn cli() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("sheetdiff"))
}

#[test]
fn value_modification() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a.xlsx");
    let b = dir.path().join("b.xlsx");
    common::basic_xlsx(&a, 123.0);
    common::basic_xlsx(&b, 456.0);
    cli().args([&a, &b]).assert().code(1).stdout(
        predicate::str::contains("[M]")
            .and(predicate::str::contains("B2"))
            .and(predicate::str::contains("123")),
    );
}
#[test]
fn identical_files() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("a.xlsx");
    common::basic_xlsx(&p, 1.0);
    cli()
        .args([&p, &p])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("0 sheets changed"));
}
#[test]
fn csv_with_headers_and_unicode() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a.csv");
    let b = dir.path().join("b.csv");
    common::csv(&a, "名字,金额\n预算,123\n");
    common::csv(&b, "名字,金额\n预算,456\n");
    cli()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("B2"));
}
#[test]
fn missing_file_error() {
    cli()
        .args(["missing.xlsx", "missing.xlsx"])
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("cannot open").and(predicate::str::contains("file not found")),
        );
}
#[test]
fn invalid_input_and_cli_help() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("bad.xlsx");
    common::csv(&p, "not a zip");
    cli()
        .args([&p, &p])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot read"));
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--epsilon"));
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sheetdiff"));
}
