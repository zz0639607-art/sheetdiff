# Examples / 示例导览

All files below are generated from synthetic fixtures. They contain no user workbook data.

以下示例全部由合成夹具生成，不包含用户工作簿内容。

- [Interactive HTML report / HTML 报告](https://zz0639607-art.github.io/sheetdiff/)
- [Standalone HTML source / 可下载的单文件 HTML](index.html)
- [Complete JSON report / 完整 JSON](examples/report.json)
- [Plain terminal transcript / 终端输出](examples/terminal.txt)
- [JSON field documentation / JSON 字段说明](json-schema.md)
- [Formal JSON Schema / 正式 Schema](sheetdiff-v1.schema.json)
- [Source fixture generator / 夹具生成代码](../tests/fixtures.rs)

```sh
cargo run -- tests/fixtures/budget_v1.xlsx tests/fixtures/budget_v2.xlsx \
  --html report.html --json report.json --color never
```

Exit code **1** means differences were found. The sample includes B2 `123 → 456`, C3 formula and cached-value changes, D1 bold formatting, an inserted/deleted row, added/deleted sheets, an unchanged-sheet fold and ISO dates.

退出码 **1** 表示发现正常差异。示例包含 B2 值变化、C3 公式与缓存值变化、D1 粗体变化、整行增删、sheet 增删、未变化 sheet 折叠区及 ISO 日期。

The report works offline without JavaScript. GitHub Pages serves the same standalone report as a convenient public preview.

HTML 报告离线可用，不依赖 JavaScript；GitHub Pages 仅提供同一报告的公开预览。
