# Run from the repository root after the documented cargo build step.
Require cargo
Require bash
Output docs/demo.gif
Set Shell bash
Set Width 1440
Set Height 1050
Set FontSize 18
Set FontFamily "DejaVu Sans Mono"
Set TypingSpeed 22ms
Set Padding 24
Set Theme "Catppuccin Mocha"

# Cached build confirms the single CLI exists before hiding shell setup.
Type "cargo build --release --locked"
Enter
Wait+Screen /Finished/
Hide
Type "export PATH=\"$PWD/target/release:$PATH\"; clear"
Enter
Sleep 500ms
Show
Type "sheetdiff tests/fixtures/budget_v1.xlsx tests/fixtures/budget_v2.xlsx --context 0 --color always || test $? -eq 1"
Enter
Sleep 7s
Type "sheetdiff tests/fixtures/budget_v1.xlsx tests/fixtures/budget_v2.xlsx --summary-only || test $? -eq 1"
Enter
Sleep 4s
Type "sheetdiff tests/fixtures/budget_v1.xlsx tests/fixtures/budget_v2.xlsx --html target/demo.html --json target/demo.json --summary-only || test $? -eq 1"
Enter
Sleep 5s
