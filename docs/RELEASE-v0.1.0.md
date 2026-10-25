# sheetdiff v0.1.0

Review Excel and CSV changes one cell at a time — entirely offline.

逐单元格审阅 Excel/CSV 变化，完全离线运行。

## Highlights / 主要功能

- **Excel + CSV:** `.xlsx` values/formulas, CSV headers/delimiters, Unicode and Chinese content.
- **Reviewable changes:** independent value, formula and direct-format events with A1 coordinates.
- **Structural diff:** row additions/removals rendered as blocks; sheet additions/removals detected.
- **Correctness controls:** absolute epsilon, format/formula skipping, ISO dates/durations, explicit blanks vs missing cells, merged-cell annotations.
- **Three outputs:** colored terminal diff, standalone offline HTML, stable `sheetdiff/v1` JSON with file SHA256.
- **Git integration:** difftool instructions and exit codes 0 = identical, 1 = differences, 2 = error.
- **Single binary:** no Excel, Python, Rust or application runtime needed after installation; normal OS libraries apply.

支持值、公式、粗体/斜体/填充/字体/数字格式/对齐比较，行和 sheet 增删检测、浮点容差、日期与合并标注，提供终端、HTML 和 JSON 报告。

## Download / 下载

Choose the archive matching your OS and architecture. Every platform archive includes the executable, README and MIT license; verify the corresponding `.sha256` file before extraction. Source archives are provided by GitHub automatically.

选择对应系统和架构的压缩包，核对同名 SHA256 校验文件后解压。平台压缩包包含程序、README 和 MIT 许可证，源码压缩包由 GitHub 自动提供。

- Linux x86_64: `x86_64-unknown-linux-musl`.
- macOS Apple Silicon: `aarch64-apple-darwin`.
- Windows x86_64: `x86_64-pc-windows-msvc`; a locally validated GNU build may also be attached.

```sh
sheetdiff old.xlsx new.xlsx
sheetdiff old.xlsx new.xlsx --html report.html --json report.json
sheetdiff old.csv new.csv --delimiter ";" --no-header
```

Exit code **1** is an expected result when changes exist.

## See everything / 查看完整内容

- [Project and bilingual documentation](https://github.com/zz0639607-art/sheetdiff)
- [Live HTML example](https://zz0639607-art.github.io/sheetdiff/)
- [Example JSON, transcript and fixture code](https://github.com/zz0639607-art/sheetdiff/blob/main/docs/EXAMPLES.md)
- [JSON Schema](https://github.com/zz0639607-art/sheetdiff/blob/main/docs/sheetdiff-v1.schema.json)
- [Test and build workflows](https://github.com/zz0639607-art/sheetdiff/actions)

## Validation and limits / 验证与限制

The source was locally validated with **66 passing tests** plus an explicitly executed fixture-regeneration test, formatting, clippy, release build, packaging, exit-code smoke checks, jq parsing and JSON Schema validation. Hosted CI status is visible in Actions.

本地已通过 66 项测试，夹具再生成已单独执行，完成 fmt、clippy、release 构建、打包、退出码、jq 和 Schema 验证。远端构建结果以 Actions 为准。

Formula results use the saved cached values; formulas are not recalculated. Direct cell formatting is compared, not conditional-format evaluation. Row identity is heuristic around completely rewritten rows. Documented display limits keep reports usable while JSON retains every event. This release supports `.xlsx` and CSV; `.xls`/`.xlsb` are roadmap items.

MIT licensed. The project has not been published to crates.io or Homebrew.
