# Contributing / 贡献指南

Thank you for helping improve spreadsheet review. Use Rust stable and keep changes focused on offline `.xlsx`/CSV comparison. The project uses edition 2021 and one executable named `sheetdiff`.

欢迎改进表格 review 体验。请使用 Rust stable，并将改动聚焦于离线 `.xlsx`/CSV 比较。项目采用 edition 2021，只有一个名为 `sheetdiff` 的可执行文件。

## Build and verify / 构建与检查

Run from the project root:

在项目根目录执行：

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test --locked
cargo build --release --locked
```

The first dependency download needs network access; the tool itself must remain completely offline. With cached dependencies, add `--offline` to Cargo commands. `rust_xlsxwriter`, `assert_cmd`, `predicates`, and `tempfile` are development dependencies only.

首次获取构建依赖需要联网，但工具运行时必须保持完全离线。依赖已缓存时，Cargo 命令可追加 `--offline`。`rust_xlsxwriter`、`assert_cmd`、`predicates` 和 `tempfile` 只用于开发和测试。

## Fixtures and tests / 夹具与测试

Generate the checked-in demo fixtures from synthetic Rust data:

从合成 Rust 数据生成演示夹具：

```sh
cargo test --test fixtures regenerate -- --ignored
```

Use `rust_xlsxwriter` in tests to create XLSX inputs in temporary directories. Prefer small, synthetic documents that isolate the behavior under test; do not add private workbooks. Include both comparison directions when addition/removal semantics matter. Test report values and event kinds rather than incidental table whitespace or a specific HTML palette.

测试中使用 `rust_xlsxwriter` 在临时目录创建 XLSX。优先选择小型合成数据并隔离待测行为，不要提交私有工作簿。涉及新增/删除语义时应覆盖两个比较方向；断言报告值和事件类别，避免依赖表格空白或固定 HTML 配色。

Meaningful regressions include sparse physical coordinates, repeated rows, modified rows near insertions, formula/cache differences, semantic style equality across different style IDs, merge ranges, explicit blank versus missing cells, mixed numeric precision, Unicode, report escaping, and exit codes. Reader limits and presentation limits must have explicit diagnostics and must never silently discard machine-readable changes.

有价值的回归场景包括稀疏物理坐标、重复行、插入附近的修改行、公式与缓存值、不同样式 ID 的语义等价、合并区域、显式空值与缺失、混合数字精度、Unicode、报告转义及退出码。读取资源限制和展示截断都应明确提示，不能悄悄丢失机器可读变更。

## Implementation expectations / 实现约定

- Keep reusable data/comparison logic in the library and keep `main.rs` thin. / 可复用读取及比较逻辑放在库中，CLI 层保持简洁。
- Explain alignment decisions in comments, particularly ambiguous row identity and resource fallback. / 注释说明对齐决策，尤其是行身份不明确及资源降级策略。
- Verify unfamiliar crate APIs against their source or docs.rs before use. / 不熟悉的 crate API 应先核实源码或 docs.rs。
- Keep user-facing output and errors in English; maintain Chinese and English README content. / 用户输出及错误使用英文，README 保持中英双语。
- Preserve the no-network runtime and avoid AI/LLM dependencies, async runtimes, web frameworks, and unrelated large dependencies. / 保持运行时无网络请求，不引入 AI/LLM 依赖、异步运行时、Web 框架及无关重型依赖。
- Keep HTML a single escaped document with embedded CSS and no external assets or JavaScript. / HTML 必须是正确转义、内嵌 CSS、无外部资源及 JavaScript 的单文件。
- Preserve the [`sheetdiff/v1` contract](docs/json-schema.md), updating the formal schema and tests when additive fields change. / 保持 v1 契约；新增字段时同步正式 Schema 和测试。

## Submitting changes / 提交改动

Describe the concrete input that was handled incorrectly, the corrected behavior, and how you verified it. Include a small synthetic reproduction for reader or alignment bugs. If scope changes during implementation, make the final title and description match the final patch. Update docs for user-visible changes.

请说明具体触发输入、修复后的行为及验证方式。读取或对齐问题应附小型合成复现。实现范围发生变化时，最终标题与描述应对应最终补丁。面向用户的行为变化需要同步文档。

Publishing packages or releases is a maintainer action; normal contributions only need reviewed source and passing checks. Contributions are licensed under [MIT](LICENSE).

包和 Release 由维护者发布；普通贡献只需提交可 review 的源码并通过检查。贡献采用 [MIT 许可](LICENSE)。
