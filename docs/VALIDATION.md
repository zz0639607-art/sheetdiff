# Local validation / 本地验收

Validation date: **2026-10-25**, Windows x86_64 GNU, Rust stable **1.99.1**, edition 2021.

The isolated development toolchain and dependencies were downloaded into the task's scratch directory. They are not included in the source or runtime distribution. Runtime execution is offline.

| Stage | Result |
| --- | --- |
| Phase 1 | 5 integration tests passed before starting Phase 2. |
| Phase 2 | 48 tests passed before starting Phase 3. |
| Phase 3 | 64 tests passed before starting Phase 4. |
| Final | **66 tests passed**, 0 failed; 1 fixture-regeneration test intentionally ignored in the default run. |
| Fixture regeneration | Explicitly executed successfully; fixed creation metadata makes XLSX bytes reproducible. |
| `cargo fmt --check` | Passed. |
| `cargo clippy --all-targets -- -D warnings` | Passed, including tests. |
| `cargo test --locked --offline` | Passed. |
| `cargo build --release --locked --offline` | Passed. |
| `scripts/build-release.ps1 -Target x86_64-pc-windows-gnu` | Passed; produced a ZIP plus SHA256. |
| `cargo package --locked --offline --allow-dirty` | Passed verification. Package metadata warns that future repository/homepage/documentation URLs have not been assigned. |
| Exit codes | Release executable: identical = 0, changed = 1, missing input = 2. |
| JSON | Parsed by **jq 1.8.2**; required fields checked; full report validated with `docs/sheetdiff-v1.schema.json`. |
| HTML | Report tests passed: CSS tabs, inline styles, no scripts/external assets, escaping, context, unchanged-sheet fold and truncation. |
| Chrome visual inspection | **Not completed**: Computer Use stopped because it could not reliably identify the current Windows browser URL. No further browser actions were performed. |
| Linux/macOS/MSVC jobs | Workflows configured; not executed from this local Windows GNU session. |

The Windows executable's import table contains only operating-system DLLs; it does not need Rust, Excel, Python, libgcc, or winpthread installed. MSVC builds are configured to link the CRT statically. Linux release builds target musl. Platform-specific release jobs still need their first hosted CI run.

The included sample budget report has **3 changed sheets, 3 value events, 1 formula event, 1 format event, 33 cells added, 8 cells removed, 11 rows added, 3 rows removed, 1 sheet added and 1 sheet removed**. Counts include the contents of whole added/removed sheets. The formula's cached result and text are separate events.

最终本地测试为 **66 项通过、0 项失败**；夹具再生成测试默认忽略，但已单独成功运行。演示工作簿、JSON、HTML、Windows 发布二进制与发行 ZIP 均已生成。Chrome 实际视觉验收因工具无法确认浏览器 URL 而未完成；Linux、macOS 和 MSVC 工作流已配置，尚未在此会话中运行。不声称已经发布 crates.io 包或 GitHub Release。

Run the usual commands from the repository root on a machine with Rust stable:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
cargo run -- tests/fixtures/budget_v1.xlsx tests/fixtures/budget_v2.xlsx
```

The final command intentionally returns **1** because it finds differences. The generated HTML should be opened manually in Chrome to complete visual acceptance. Demo recording and publication instructions are in [README.md](../README.md) and [demo.md](demo.md).
