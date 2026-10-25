# JSON reports / JSON 报告

`--json FILE` writes a complete UTF-8 report with `"schema": "sheetdiff/v1"`. The formal [Draft 2020-12 JSON Schema](sheetdiff-v1.schema.json) uses only local `$ref` definitions; no schema or other resource is fetched by sheetdiff at runtime. Chinese and other Unicode strings remain readable in JSON.

`--json FILE` 输出完整的 UTF-8 报告，版本标识为 `sheetdiff/v1`。[正式 Schema](sheetdiff-v1.schema.json) 采用 Draft 2020-12，仅包含本地引用。sheetdiff 运行时不会下载 Schema 或其他资源。

## Generate and inspect

Differences produce exit code **1**, so shell pipelines should accept both 0 and 1 before reading the report:

```sh
sheetdiff old.xlsx new.xlsx --json report.json
result=$?
if [ "$result" -gt 1 ]; then exit "$result"; fi

# Parse JSON and check the version plus essential field types.
# jq is an optional inspection tool, not a sheetdiff dependency.
jq -e '
  .schema == "sheetdiff/v1" and
  (.old.sha256 | test("^[a-f0-9]{64}$")) and
  (.new.sha256 | test("^[a-f0-9]{64}$")) and
  (.summary.sheets_changed | type == "number") and
  (.sheets | type == "array") and
  (.warnings | type == "array")
' report.json

# List modified values with coordinates on both sides of row insertions.
jq -r '.sheets[] | .name as $sheet | .changes[] |
  select(.kind == "modified") |
  [$sheet, .old_cell, .new_cell, .old.formatted, .new.formatted] | @tsv' report.json
```

The `jq` check above verifies a useful subset of the contract; it is not a complete JSON Schema validator. To validate every field against Draft 2020-12 with an independently installed Python `jsonschema` package:

```sh
python -c 'import json; from jsonschema import Draft202012Validator; schema=json.load(open("docs/sheetdiff-v1.schema.json", encoding="utf-8")); Draft202012Validator.check_schema(schema); Draft202012Validator(schema).validate(json.load(open("report.json", encoding="utf-8")))'
```

差异退出码为 **1**，运行错误为 **2**；脚本应先接受退出码 0/1，再读取报告。`jq` 示例可检查 JSON 语法、版本和关键类型；完整校验需要单独安装的 JSON Schema 校验器。这些工具都不是 sheetdiff 的运行时依赖。

## Top-level fields

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | string | Exactly `sheetdiff/v1`. |
| `old`, `new` | object | Each contains `path` as supplied to the CLI and lowercase `sha256` of the original file bytes. Hashes do not depend on comparison options. |
| `summary` | object | Nonnegative integer event counters, defined below. |
| `sheets` | array | An index of all compared sheets, including unchanged sheets. `--sheet` excludes unselected sheets from this index. |
| `warnings` | string array | English diagnostics such as alignment fallback notices. They do not remove report events. |

## Summary counters

| Field | Counted events |
| --- | --- |
| `sheets_changed` | Compared sheets whose status is `modified`, `added`, or `removed`. |
| `sheets_added`, `sheets_removed` | Added or removed named sheets, including empty sheets. |
| `cells_modified` | `modified` value events. |
| `cells_added`, `cells_removed` | `added` or `removed` cell events, **including cells within added/deleted rows and sheets**. An explicitly present blank cell also counts when its presence changes. |
| `formulas_changed` | `formula_changed` events on compared cells. |
| `formats_changed` | `format_changed` events, including a merge-layout event when a common sheet's merge set changes. |
| `rows_added`, `rows_removed` | Added/deleted represented rows, including rows belonging to added/deleted sheets. Sparse unstored gaps are not expanded into millions of empty rows. |

Counters count events, not a deduplicated set of coordinates. A cell whose cached value, formula, and style all change can contribute **three separate events** at one coordinate. Added/deleted row cells are counted as additions/removals rather than also being counted as formula or format changes.

统计包含新增/删除行及 sheet 中的单元格。数值、公式和格式是独立事件，因此同一坐标可以出现多次；不能把所有事件计数相加后当作唯一变更单元格数量。稀疏表中未存储的空白间隙不会展开成大量空行。

## Sheet records

Every sheet has these fields:

| Field | Meaning |
| --- | --- |
| `name` | Exact sheet name; CSV inputs use the logical name `CSV`. |
| `status` | `unchanged`, `modified`, `added`, or `removed`. |
| `changes` | Complete list of changed-cell events. There is **no `unchanged` cell event**. |
| `row_changes` | One record per added/deleted row for block rendering. |
| `context` | Unchanged neighboring rows selected by `--context`; separate from changed-cell events. |
| `merged_old`, `merged_new` | Old/new A1 merge ranges such as `A1:B2`, retained as display metadata. |

A changed merge layout produces a `format_changed` event at an affected anchor cell. The complete range sets in `merged_old` and `merged_new` describe the layout difference. This event may share a coordinate with an ordinary cell-format event. `--skip-format` ignores style and merge-layout differences when deciding whether cells/sheets changed, while presence differences such as an explicit blank becoming missing still count.

## Cell events and coordinates

```json
{
  "cell": "B4",
  "old_cell": "B3",
  "new_cell": "B4",
  "column_name": "amount",
  "kind": "modified",
  "old": {"value": 20, "formula": null, "formatted": "20", "format": {}},
  "new": {"value": 21, "formula": null, "formatted": "21", "format": {}},
  "row_change": false
}
```

`kind` is one of `modified`, `formula_changed`, `format_changed`, `added`, or `removed`. `cell` uses the new coordinate when an aligned new row exists, otherwise the old coordinate. `old_cell` and `new_cell` preserve original positions across row insertions/deletions and are null when that side's row does not exist. A coordinate can still be known when its cell record is missing; inspect `old`/`new` for actual cell presence.

`column_name` is a CSV header label, or null when unavailable or using `--no-header`. Duplicate labels receive suffixes such as `amount#2`. Headers provide display labels; columns still align by index. The CSV header record itself remains row 1 and participates in comparison.

`row_change: true` marks a cell event already grouped into a `row_changes` block. Consumers rendering row blocks should avoid printing those same events a second time. The events remain in `changes` so machine consumers can inspect every added/deleted cell.

坐标使用 A1 风格，`old_cell`/`new_cell` 分别保留两侧原始位置。`row_change: true` 表示该事件已归入整行增删块；渲染行块时应避免再次逐单元格展示，但 JSON 仍保留全部事件。

## Missing, blank, and typed values

These states are intentionally different:

```json
{
  "old": null,
  "new": {"value": null, "formula": null, "formatted": "(empty)", "format": {}}
}
```

- `old: null` means the cell is **missing** on that side.
- A present cell object with `value: null` means an **explicit blank**. An XLSX formula without a cached result may also have a null value; its `formula` field identifies it.
- A string value of `""` is an empty string, distinct from both missing and blank.
- XLSX numeric/boolean cells remain JSON numbers/booleans. Dates become ISO 8601 strings without an invented timezone, elapsed durations become ISO 8601 duration strings, and Excel errors are strings.
- CSV data fields that parse as finite JSON numbers become JSON numbers. Leading-zero identifiers such as `001` remain strings; header fields remain strings. Other CSV text, including `true`, remains a string. The original CSV numeric spelling is retained in `formatted`.
- `formula` is null or normalized formula text beginning with `=`. Formulas are read, not evaluated; `value` is the cached result when present.
- `formatted` is sheetdiff's normalized display value, not a full reproduction of Excel's locale-dependent formatted appearance. Formula text has its own field.
- `format` maps semantic property names to **string** values, such as `{"bold":"true","number_format":"0.00"}`. Properties refer to resolved styles, not workbook-local style IDs.

缺失单元格用整个对象为 `null` 表示；已存在的空单元格用对象内的 `value: null` 表示。字符串、数字和布尔值保持 JSON 类型，中文原样保留。公式不重新计算，`value` 使用文件内缓存结果。

## Row blocks and context

`row_changes` records have `row` (one-based original row number on the added/deleted side), `kind` (`added` or `removed`), and `cells`. `context` records have one-based `old_row`, `new_row`, and `cells` from the unchanged new-side row.

`cells` is a sparse object keyed by **zero-based decimal column indices**, for example:

```json
{"row": 5, "kind": "added", "cells": {"0": {"value": "new item", "formula": null, "formatted": "new item", "format": {}}}}
```

Object key `"0"` means column A, `"1"` means B, and so on. Missing keys denote missing cells. Context rows are display assistance and do not affect counters.

## Completeness and compatibility

Terminal and HTML presentation limits do **not** truncate JSON `changes`, `row_changes`, summary counters, or merge metadata. Alignment fallback warnings describe the matching strategy; every represented cell is still compared. Reports are written for identical inputs too, and `--summary-only` affects terminal detail only.

Version 1 permits additive object fields. Consumers should ignore unknown object fields while checking the exact `schema` value and known enum values. Removing/renaming fields, changing their types or existing meanings, or introducing incompatible event/status variants requires a new schema identifier. UTF-8, snake_case names, nullable presence semantics, and one-based A1 coordinates are part of the contract.

JSON 不受终端或 HTML 展示截断影响。相同文件也会生成报告；`--summary-only` 仅限制终端详情。v1 允许新增字段，消费者应忽略未知字段；删除/重命名字段、改变类型或现有含义，以及引入不兼容的事件状态，必须升级 Schema 标识。
