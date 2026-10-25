# Changelog

## 0.1.0 — 2026-10-25

First public release of the standalone, offline Rust CLI.

- Cell-level `.xlsx` and CSV comparisons with A1 coordinates and Unicode text.
- Independent value, formula and direct-format events.
- Bounded LCS row alignment, whole-row blocks, sheet additions and removals.
- Sparse streaming XLSX reading, explicit blanks, inherited styles and merge annotations.
- ISO 8601 dates/durations and configurable absolute numeric tolerance.
- CSV headers, unique duplicate labels, delimiter and no-header modes.
- ANSI terminal diff, self-contained HTML and complete versioned JSON reports.
- Git-compatible 0/1/2 exit codes and difftool documentation.
- Synthetic fixtures, regression tests, MIT license and multi-platform release automation.

### Known limits

- Formula results are read from the saved workbook cache; formulas are not recalculated.
- Direct cell formatting is compared; conditional-format rules are not evaluated.
- Ambiguous inserted/rewritten rows may use positional replacement alignment.
- Reader resource limits and display limits are documented in README; JSON events are not display-truncated.
- `.xls`, `.xlsb`, VBA, workbook merging and formula dependency graphs are outside 0.1.

首个公开版本支持离线单元格 diff、公式/格式/行/sheet 检测、终端与 HTML/JSON 报告、Git 集成及三平台发布自动化。具体语义与限制见双语 README。
