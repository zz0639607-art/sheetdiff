# Native Windows packaging, including the default MSVC and GNU Rust targets.
param([string]$Target = '')
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    if (-not $Target) {
        $Target = (& rustc -vV | Select-String '^host: ').Line.Substring(6)
    }
    if ($Target -notmatch '^[a-zA-Z0-9_-]+$') { throw 'Invalid Rust target' }
    $version = [regex]::Match((Get-Content Cargo.toml -Raw),'(?m)^version = "([^"]+)"').Groups[1].Value
    $artifact = "sheetdiff-v$version-$Target"
    & cargo build --locked --release --target $Target
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    $stage = Join-Path $PWD "dist/$artifact"
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Copy-Item -LiteralPath "target/$Target/release/sheetdiff.exe",'README.md','LICENSE' -Destination $stage -Force
    $archive = Join-Path $PWD "dist/$artifact.zip"
    Compress-Archive -LiteralPath $stage -DestinationPath $archive -Force
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$archive.sha256","$hash  $artifact.zip`n")
    Write-Output "Archive: $archive"
} finally {
    Pop-Location
}
