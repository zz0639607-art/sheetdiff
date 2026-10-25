//! Bounded local-only Open Packaging Convention member access.
use anyhow::{anyhow, ensure, Context, Result};
use roxmltree::{Document, ParsingOptions};
use std::{fs::File, io::Read, path::Path};
use zip::ZipArchive;

const MAX_XML_PART: u64 = 64 * 1024 * 1024;
const MAX_XML_TOTAL: u64 = 256 * 1024 * 1024;

pub(super) struct Package {
    zip: ZipArchive<File>,
}
impl Package {
    /// Check advertised XML sizes before handing the package to either parser.
    pub(super) fn open(path: &Path) -> Result<Self> {
        let mut zip = ZipArchive::new(File::open(path)?)?;
        let mut total = 0u64;
        for i in 0..zip.len() {
            let file = zip.by_index(i)?;
            let name = file.name()?;
            if name.ends_with(".xml") || name.ends_with(".rels") {
                ensure!(
                    file.size() <= MAX_XML_PART,
                    "XLSX XML part {name} exceeds the 64 MiB resource limit"
                );
                total = total
                    .checked_add(file.size())
                    .ok_or_else(|| anyhow!("XLSX XML sizes overflow"))?;
                ensure!(
                    total <= MAX_XML_TOTAL,
                    "XLSX XML content exceeds the 256 MiB resource limit"
                );
            }
        }
        Ok(Self { zip })
    }
    pub(super) fn xml(&mut self, name: &str) -> Result<String> {
        let file = self
            .zip
            .by_name(name)
            .with_context(|| format!("missing XLSX part {name}"))?;
        let mut text = String::new();
        file.take(MAX_XML_PART + 1)
            .read_to_string(&mut text)
            .with_context(|| format!("invalid UTF-8 XLSX part {name}"))?;
        ensure!(
            text.len() as u64 <= MAX_XML_PART,
            "XLSX XML part {name} exceeds the 64 MiB resource limit"
        );
        Ok(text)
    }
}

/// DTDs stay disabled; the node bound limits DOM memory amplification.
pub(super) fn parse_xml(text: &str) -> Result<Document<'_>> {
    Ok(Document::parse_with_options(
        text,
        ParsingOptions {
            nodes_limit: 2_000_000,
            ..ParsingOptions::default()
        },
    )?)
}
pub(super) fn relationships_path(part: &str) -> String {
    let (dir, file) = part.rsplit_once('/').unwrap_or(("", part));
    if dir.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{dir}/_rels/{file}.rels")
    }
}
/// Normalize targets without extracting archive members or opening URLs.
pub(super) fn resolve_target(source: &str, target: &str) -> Result<String> {
    ensure!(
        !target.contains('\\')
            && !target.contains(':')
            && !target.contains('?')
            && !target.contains('#'),
        "unsupported XLSX relationship target {target}"
    );
    let mut components: Vec<&str> = if target.starts_with('/') {
        vec![]
    } else {
        source
            .rsplit_once('/')
            .map(|(dir, _)| dir.split('/').collect())
            .unwrap_or_default()
    };
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                ensure!(
                    components.pop().is_some(),
                    "XLSX relationship escapes the package root"
                );
            }
            other => components.push(other),
        }
    }
    ensure!(!components.is_empty(), "empty XLSX relationship target");
    Ok(components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relationship_paths_resolve_from_the_source_part() {
        assert_eq!(
            resolve_target("xl/workbook.xml", "worksheets/sheet7.xml").unwrap(),
            "xl/worksheets/sheet7.xml"
        );
        assert_eq!(
            resolve_target("xl/workbook.xml", "../custom/data.xml").unwrap(),
            "custom/data.xml"
        );
        assert_eq!(
            resolve_target("xl/workbook.xml", "/custom/data.xml").unwrap(),
            "custom/data.xml"
        );
        assert!(resolve_target("", "../data.xml").is_err());
        assert!(resolve_target("", "https://example.com").is_err());
    }
}
