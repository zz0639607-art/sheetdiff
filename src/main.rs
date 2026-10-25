//! Thin CLI adapter; comparison logic lives in the library.
use clap::Parser;
use sheetdiff::{
    cli::{Cli, Color},
    render::terminal,
};
use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(changed) => ExitCode::from(u8::from(changed)),
        Err(error) => {
            eprintln!("sheetdiff: {}", diagnostic(&error));
            ExitCode::from(2)
        }
    }
}

// OS error Display text is localized on Windows. Keep our diagnostics English
// while retaining the platform error code for troubleshooting.
fn diagnostic(error: &anyhow::Error) -> String {
    if let Some(io_error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<io::Error>())
    {
        let description = match io_error.kind() {
            io::ErrorKind::NotFound => "file not found".into(),
            io::ErrorKind::PermissionDenied => "permission denied".into(),
            io::ErrorKind::AlreadyExists => "file already exists".into(),
            io::ErrorKind::InvalidInput => "invalid input".into(),
            io::ErrorKind::InvalidData => "invalid data".into(),
            io::ErrorKind::UnexpectedEof => "unexpected end of file".into(),
            io::ErrorKind::BrokenPipe => "broken pipe".into(),
            io::ErrorKind::WriteZero => "unable to write data".into(),
            kind => format!("I/O error ({kind:?})"),
        };
        let context = if error
            .chain()
            .next()
            .is_some_and(|cause| cause.downcast_ref::<io::Error>().is_some())
        {
            "I/O operation failed".into()
        } else {
            error.to_string()
        };
        let code = io_error
            .raw_os_error()
            .map(|code| format!(" (OS code {code})"))
            .unwrap_or_default();
        format!("{context}: {description}{code}")
    } else {
        format!("{error:#}")
    }
}
fn run(cli: &Cli) -> anyhow::Result<bool> {
    let report = sheetdiff::compare_files(&cli.old, &cli.new, &cli.options())?;
    validate_outputs(cli)?;
    if let Some(path) = &cli.json {
        write_report(path, &sheetdiff::render::json::render(&report)?)?;
    }
    if let Some(path) = &cli.html {
        write_report(path, &sheetdiff::render::html::render(&report))?;
    }
    let color = match cli.color {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => io::stdout().is_terminal(),
    };
    io::stdout()
        .lock()
        .write_all(terminal::render(&report, color, cli.summary_only).as_bytes())?;
    Ok(report.has_changes())
}

fn write_report(path: &Path, text: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    fs::write(path, text).with_context(|| format!("cannot write report {}", path.display()))
}

// Resolve symlinks and existing parents before writing either report, protecting
// both inputs and preventing two requested formats from overwriting each other.
fn validate_outputs(cli: &Cli) -> anyhow::Result<()> {
    use anyhow::Context;
    let inputs = [fs::canonicalize(&cli.old)?, fs::canonicalize(&cli.new)?];
    let mut outputs = Vec::new();
    for path in [&cli.json, &cli.html].into_iter().flatten() {
        anyhow::ensure!(
            !path.is_dir(),
            "report output is a directory: {}",
            path.display()
        );
        let resolved = resolve_output(path)
            .with_context(|| format!("invalid report output {}", path.display()))?;
        anyhow::ensure!(
            !inputs.iter().any(|input| same_path(input, &resolved)),
            "report output would overwrite an input: {}",
            path.display()
        );
        anyhow::ensure!(
            !outputs
                .iter()
                .any(|output: &PathBuf| same_path(output, &resolved)),
            "HTML and JSON outputs must use different files"
        );
        outputs.push(resolved);
    }
    Ok(())
}
fn resolve_output(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("output path needs a filename"))?;
    Ok(fs::canonicalize(parent)?.join(name))
}
fn same_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}
