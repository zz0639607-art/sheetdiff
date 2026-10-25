//! Date normalization must preserve the distinction between value and format edits.
use rust_xlsxwriter::{Format, Workbook};
use sheetdiff::{compare_files, model::Options};
use tempfile::tempdir;

#[test]
fn date_number_format_alone_is_only_a_format_change() {
    let dir = tempdir().unwrap();
    let old = dir.path().join("old.xlsx");
    let new = dir.path().join("new.xlsx");
    for (path, date) in [(&old, false), (&new, true)] {
        let mut book = Workbook::new();
        let sheet = book.add_worksheet();
        if date {
            sheet
                .write_number_with_format(
                    0,
                    0,
                    46000.0,
                    &Format::new().set_num_format("yyyy-mm-dd"),
                )
                .unwrap();
        } else {
            sheet.write_number(0, 0, 46000.0).unwrap();
        }
        book.save(path).unwrap();
    }
    let report = compare_files(&old, &new, &Options::default()).unwrap();
    assert_eq!(report.summary.cells_modified, 0);
    assert_eq!(report.summary.formats_changed, 1);
    let options = Options {
        skip_format: true,
        ..Options::default()
    };
    assert!(!compare_files(&old, &new, &options).unwrap().has_changes());
    assert!(!compare_files(&new, &old, &options).unwrap().has_changes());
}
