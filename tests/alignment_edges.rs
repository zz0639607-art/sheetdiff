//! Alignment regressions for sparse worksheet positions and numeric boundaries.
use serde_json::{json, Value};
use sheetdiff::{
    diff,
    model::{Cell, FileInfo, Options, Row, Sheet, Workbook},
};

fn sparse(rows: &[(usize, Value)]) -> Workbook {
    Workbook {
        file: FileInfo {
            path: "synthetic.xlsx".into(),
            sha256: String::new(),
        },
        sheets: vec![Sheet {
            name: "Data".into(),
            rows: rows
                .iter()
                .map(|(row, value)| (*row, Row::from([(0, Cell::new(value.clone()))])))
                .collect(),
            ..Sheet::default()
        }],
    }
}

#[test]
fn single_populated_cell_moving_to_another_physical_row_is_a_change() {
    let old = sparse(&[(0, json!("same content"))]);
    let new = sparse(&[(1, json!("same content"))]);
    assert!(diff::compare(&old, &new, &Options::default()).has_changes());
}

#[test]
fn sparse_tail_moving_without_inserted_populated_row_is_a_change() {
    let old = sparse(&[(0, json!("head")), (9, json!("tail"))]);
    let new = sparse(&[(0, json!("head")), (10, json!("tail"))]);
    assert!(diff::compare(&old, &new, &Options::default()).has_changes());
}

#[test]
fn populating_a_sparse_gap_does_not_change_a_stationary_tail() {
    let old = sparse(&[(0, json!("head")), (9, json!("tail"))]);
    let new = sparse(&[
        (0, json!("head")),
        (4, json!("inserted")),
        (9, json!("tail")),
    ]);
    let report = diff::compare(&old, &new, &Options::default());
    assert_eq!(report.summary.cells_added, 1);
    assert_eq!(report.summary.cells_removed, 0);
    assert_eq!(report.summary.cells_modified, 0);
    assert!(report.sheets[0].changes.iter().all(|c| c.cell != "A10"));
}

#[test]
fn inserted_sparse_row_can_explain_a_shifted_tail() {
    let old = sparse(&[(0, json!("head")), (9, json!("tail"))]);
    let new = sparse(&[
        (0, json!("head")),
        (4, json!("inserted")),
        (10, json!("tail")),
    ]);
    let report = diff::compare(&old, &new, &Options::default());
    assert_eq!(report.summary.rows_added, 1);
    assert_eq!(report.summary.cells_added, 1);
    assert_eq!(report.summary.cells_removed, 0);
    assert_eq!(report.summary.cells_modified, 0);
}

#[test]
fn mixed_integer_float_comparison_preserves_large_integer_precision() {
    let old = sparse(&[(0, json!(9_007_199_254_740_993u64))]);
    let new = sparse(&[(0, json!(9_007_199_254_740_992.0f64))]);
    let report = diff::compare(&old, &new, &Options::default());
    assert_eq!(report.summary.cells_modified, 1);
}

#[test]
fn absolute_epsilon_boundary_is_inclusive_for_exact_binary_values() {
    let old = sparse(&[(0, json!(1.0))]);
    let new = sparse(&[(0, json!(1.5))]);
    let on_boundary = Options {
        epsilon: 0.5,
        ..Options::default()
    };
    assert!(!diff::compare(&old, &new, &on_boundary).has_changes());
    let below_boundary = Options {
        epsilon: 0.499,
        ..Options::default()
    };
    assert!(diff::compare(&old, &new, &below_boundary).has_changes());
}
