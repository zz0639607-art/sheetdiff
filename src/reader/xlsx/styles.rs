//! Resolve style resources to semantic properties instead of local table IDs.
use super::package::parse_xml;
use crate::model::Format;
use anyhow::{anyhow, Result};
use calamine::ExcelDateTimeType;
use roxmltree::Node;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(super) struct Style {
    pub(super) format: Format,
    pub(super) date_type: Option<ExcelDateTimeType>,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            format: default_format(),
            date_type: None,
        }
    }
}
pub(super) struct Styles {
    cells: Vec<Style>,
}

impl Styles {
    pub(super) fn parse(xml: Option<&str>, theme: Option<&str>) -> Result<Self> {
        let Some(xml) = xml else {
            return Ok(Self {
                cells: vec![Style::default()],
            });
        };
        let doc = parse_xml(xml)?;
        let root = doc.root_element();
        let colors = Colors::parse(theme, root)?;
        let fonts = child(root, "fonts")
            .map(|n| {
                n.children()
                    .filter(|n| n.has_tag_name("font"))
                    .map(|n| font(n, &colors))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let fills = child(root, "fills")
            .map(|n| {
                n.children()
                    .filter(|n| n.has_tag_name("fill"))
                    .map(|n| fill(n, &colors))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let formats = child(root, "numFmts")
            .map(|n| {
                n.children()
                    .filter(|n| n.has_tag_name("numFmt"))
                    .map(|n| {
                        Ok((
                            number(n, "numFmtId", 0)?,
                            n.attribute("formatCode").unwrap_or("General").to_owned(),
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let resources = Resources {
            fonts,
            fills,
            formats,
        };
        let mut bases = Vec::new();
        if let Some(xfs) = child(root, "cellStyleXfs") {
            for xf in xfs.children().filter(|n| n.has_tag_name("xf")) {
                bases.push(resources.style(xf, &Style::default())?);
            }
        }
        let mut cells = Vec::new();
        let default = Style::default();
        if let Some(xfs) = child(root, "cellXfs") {
            for xf in xfs.children().filter(|n| n.has_tag_name("xf")) {
                let base = if xf.attribute("xfId").is_some() {
                    bases
                        .get(number(xf, "xfId", 0)? as usize)
                        .ok_or_else(|| anyhow!("invalid base style reference"))?
                } else {
                    &default
                };
                cells.push(resources.style(xf, base)?);
            }
        }
        if cells.is_empty() {
            cells.push(default);
        }
        Ok(Self { cells })
    }
    pub(super) fn get(&self, index: usize) -> Result<&Style> {
        self.cells
            .get(index)
            .ok_or_else(|| anyhow!("invalid XLSX cell style index {index}"))
    }
}

struct Resources {
    fonts: Vec<Format>,
    fills: Vec<String>,
    formats: BTreeMap<u32, String>,
}
impl Resources {
    /// Named style inheritance is resolved before comparison. Explicit apply*=0
    /// inherits; otherwise local resource references override the base. Resource
    /// IDs are discarded, so table reordering cannot create a format change.
    fn style(&self, xf: Node<'_, '_>, base: &Style) -> Result<Style> {
        let mut result = base.clone();
        if applies(xf, "applyFont") && xf.attribute("fontId").is_some() {
            let index = number(xf, "fontId", 0)? as usize;
            result.format.extend(
                self.fonts
                    .get(index)
                    .ok_or_else(|| anyhow!("invalid XLSX font index {index}"))?
                    .clone(),
            );
        }
        if applies(xf, "applyFill") && xf.attribute("fillId").is_some() {
            let index = number(xf, "fillId", 0)? as usize;
            result.format.insert(
                "fill".into(),
                self.fills
                    .get(index)
                    .ok_or_else(|| anyhow!("invalid XLSX fill index {index}"))?
                    .clone(),
            );
        }
        if applies(xf, "applyNumberFormat") && xf.attribute("numFmtId").is_some() {
            let index = number(xf, "numFmtId", 0)?;
            let format = self
                .formats
                .get(&index)
                .cloned()
                .unwrap_or_else(|| builtin_number_format(index));
            result.date_type = date_type(&format);
            result.format.insert("number_format".into(), format);
        }
        if applies(xf, "applyAlignment") {
            if let Some(alignment) = child(xf, "alignment") {
                result
                    .format
                    .insert("alignment".into(), alignment_format(alignment));
            }
        }
        Ok(result)
    }
}

fn default_format() -> Format {
    [
        ("bold","false"),("italic","false"),("font","Calibri"),("font_size","11"),
        ("font_color","#000000"),("font_underline","none"),("font_strike","false"),
        ("fill","none"),("number_format","General"),
        ("alignment","horizontal=general;vertical=bottom;rotation=0;wrap=false;shrink=false;indent=0;relative_indent=0;justify_last=false;reading_order=0"),
    ].into_iter().map(|(k,v)| (k.into(),v.into())).collect()
}
fn font(node: Node<'_, '_>, colors: &Colors) -> Format {
    let mut format = default_format();
    format.retain(|key, _| key.starts_with("font") || key == "bold" || key == "italic");
    for (tag, key) in [("b", "bold"), ("i", "italic"), ("strike", "font_strike")] {
        let value = child(node, tag).is_some_and(|n| n.attribute("val").is_none_or(boolean));
        format.insert(key.into(), value.to_string());
    }
    if let Some(value) = child(node, "name").and_then(|n| n.attribute("val")) {
        format.insert("font".into(), value.into());
    }
    if let Some(value) = child(node, "sz").and_then(|n| n.attribute("val")) {
        format.insert("font_size".into(), numeric_text(value));
    }
    if let Some(value) = child(node, "color") {
        format.insert("font_color".into(), colors.color(value));
    }
    if let Some(value) = child(node, "u") {
        format.insert(
            "font_underline".into(),
            value.attribute("val").unwrap_or("single").into(),
        );
    }
    format
}
fn fill(node: Node<'_, '_>, colors: &Colors) -> String {
    if let Some(pattern) = child(node, "patternFill") {
        let kind = pattern.attribute("patternType").unwrap_or("none");
        if kind == "none" {
            return "none".into();
        }
        let fg = child(pattern, "fgColor")
            .map(|n| colors.color(n))
            .unwrap_or_else(|| "auto".into());
        let bg = child(pattern, "bgColor")
            .map(|n| colors.color(n))
            .unwrap_or_else(|| "auto".into());
        if kind == "solid" {
            return format!("solid {fg}");
        }
        return format!("{kind} foreground={fg};background={bg}");
    }
    child(node, "gradientFill")
        .map(|n| canonical(n, colors))
        .unwrap_or_else(|| "none".into())
}
fn alignment_format(node: Node<'_, '_>) -> String {
    [
        ("horizontal", "horizontal", "general", false),
        ("vertical", "vertical", "bottom", false),
        ("textRotation", "rotation", "0", false),
        ("wrapText", "wrap", "false", true),
        ("shrinkToFit", "shrink", "false", true),
        ("indent", "indent", "0", false),
        ("relativeIndent", "relative_indent", "0", false),
        ("justifyLastLine", "justify_last", "false", true),
        ("readingOrder", "reading_order", "0", false),
    ]
    .into_iter()
    .map(|(attribute, key, default, is_bool)| {
        let value = node.attribute(attribute).unwrap_or(default);
        let value = if is_bool {
            boolean(value).to_string()
        } else {
            numeric_text(value)
        };
        format!("{key}={value}")
    })
    .collect::<Vec<_>>()
    .join(";")
}

struct Colors {
    theme: Vec<String>,
    indexed: Vec<String>,
}
impl Colors {
    fn parse(theme: Option<&str>, styles: Node<'_, '_>) -> Result<Self> {
        let mut result = Self {
            theme: [
                "FFFFFF", "000000", "E7E6E6", "44546A", "4472C4", "ED7D31", "A5A5A5", "FFC000",
                "5B9BD5", "70AD47", "0563C1", "954F72",
            ]
            .into_iter()
            .map(|s| format!("#{s}"))
            .collect(),
            indexed: default_palette(),
        };
        if let Some(theme) = theme {
            let doc = parse_xml(theme)?;
            if let Some(scheme) = doc.descendants().find(|n| n.has_tag_name("clrScheme")) {
                for (index, tag) in [
                    "lt1", "dk1", "lt2", "dk2", "accent1", "accent2", "accent3", "accent4",
                    "accent5", "accent6", "hlink", "folHlink",
                ]
                .into_iter()
                .enumerate()
                {
                    if let Some(color) =
                        child(scheme, tag).and_then(|n| n.children().find(Node::is_element))
                    {
                        if let Some(value) = color
                            .attribute("lastClr")
                            .or_else(|| color.attribute("val"))
                        {
                            result.theme[index] = rgb(value);
                        }
                    }
                }
            }
        }
        if let Some(palette) = styles
            .descendants()
            .find(|n| n.has_tag_name("indexedColors"))
        {
            result.indexed = palette
                .children()
                .filter(|n| n.has_tag_name("rgbColor"))
                .map(|n| rgb(n.attribute("rgb").unwrap_or("000000")))
                .collect();
        }
        Ok(result)
    }
    fn color(&self, node: Node<'_, '_>) -> String {
        let mut value = if let Some(value) = node.attribute("rgb") {
            rgb(value)
        } else if let Some(index) = node
            .attribute("theme")
            .and_then(|s| s.parse::<usize>().ok())
        {
            self.theme
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("theme:{index}"))
        } else if let Some(index) = node
            .attribute("indexed")
            .and_then(|s| s.parse::<usize>().ok())
        {
            self.indexed
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("indexed:{index}"))
        } else {
            "auto".into()
        };
        if let Some(tint) = node
            .attribute("tint")
            .filter(|s| s.parse::<f64>().ok() != Some(0.0))
        {
            value.push_str(&format!(" tint={}", numeric_text(tint)));
        }
        value
    }
}
fn rgb(value: &str) -> String {
    // Spreadsheet colors do not expose alpha; indexed palettes commonly use
    // 00RRGGBB while direct colors use FFRRGGBB for the same visible RGB.
    let value = if value.len() == 8 {
        value.get(2..).unwrap_or(value)
    } else {
        value
    };
    format!("#{}", value.to_ascii_uppercase())
}
fn default_palette() -> Vec<String> {
    [
        "000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF", "00FFFF", "000000",
        "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF", "00FFFF", "800000", "008000",
        "000080", "808000", "800080", "008080", "C0C0C0", "808080", "9999FF", "993366", "FFFFCC",
        "CCFFFF", "660066", "FF8080", "0066CC", "CCCCFF", "000080", "FF00FF", "FFFF00", "00FFFF",
        "800080", "800000", "008080", "0000FF", "00CCFF", "CCFFFF", "CCFFCC", "FFFF99", "99CCFF",
        "FF99CC", "CC99FF", "FFCC99", "3366FF", "33CCCC", "99CC00", "FFCC00", "FF9900", "FF6600",
        "666699", "969696", "003366", "339966", "003300", "333300", "993300", "993366", "333399",
        "333333",
    ]
    .into_iter()
    .map(|s| format!("#{s}"))
    .collect()
}
fn canonical(node: Node<'_, '_>, colors: &Colors) -> String {
    if matches!(node.tag_name().name(), "color" | "fgColor" | "bgColor") {
        return colors.color(node);
    }
    let attributes: BTreeMap<_, _> = node
        .attributes()
        .map(|a| (a.name(), numeric_text(a.value())))
        .collect();
    format!(
        "{}{:?}[{}]",
        node.tag_name().name(),
        attributes,
        node.children()
            .filter(Node::is_element)
            .map(|n| canonical(n, colors))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn child<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|n| n.has_tag_name(tag))
}
fn number(node: Node<'_, '_>, attribute: &str, default: u32) -> Result<u32> {
    node.attribute(attribute)
        .map(|s| {
            s.parse()
                .map_err(|_| anyhow!("invalid style attribute {attribute}"))
        })
        .unwrap_or(Ok(default))
}
fn applies(node: Node<'_, '_>, attribute: &str) -> bool {
    node.attribute(attribute).is_none_or(boolean)
}
pub(super) fn boolean(value: &str) -> bool {
    !matches!(value, "0" | "false" | "off")
}
fn numeric_text(value: &str) -> String {
    value
        .parse::<f64>()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| value.to_owned())
}

fn builtin_number_format(index: u32) -> String {
    match index {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "mm-dd-yy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yy h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return format!("builtin:{index}"),
    }
    .into()
}
fn date_type(format: &str) -> Option<ExcelDateTimeType> {
    let lower = format.to_ascii_lowercase();
    if let Some(index) = lower.strip_prefix("builtin:") {
        let index = index.parse::<u32>().ok()?;
        return ((27..=36).contains(&index) || (50..=58).contains(&index))
            .then_some(ExcelDateTimeType::DateTime);
    }
    // Ignore literals, escapes and bracketed colors/conditions. Bracketed
    // h/m/s tokens are elapsed time rather than calendar-date formats.
    let mut tokens = String::new();
    let mut chars = lower.chars();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' | '_' | '*' if !quoted => {
                chars.next();
            }
            '[' if !quoted => {
                let bracket: String = chars.by_ref().take_while(|c| *c != ']').collect();
                if !bracket.is_empty() && bracket.chars().all(|c| matches!(c, 'h' | 'm' | 's')) {
                    return Some(ExcelDateTimeType::TimeDelta);
                }
            }
            _ if !quoted => tokens.push(c),
            _ => {}
        }
    }
    tokens
        .chars()
        .any(|c| matches!(c, 'y' | 'm' | 'd' | 'h' | 's'))
        .then_some(ExcelDateTimeType::DateTime)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_unicode_color_does_not_panic() {
        assert_eq!(rgb("€abcde"), "#€ABCDE");
    }
    #[test]
    fn resource_ids_do_not_affect_semantic_style() {
        let a = Styles::parse(Some(r#"<styleSheet><fonts><font><name val="Calibri"/><sz val="11"/></font><font><b/><name val="Calibri"/><sz val="11"/></font></fonts><fills><fill><patternFill patternType="none"/></fill></fills><cellXfs><xf fontId="1" fillId="0" numFmtId="0"/></cellXfs></styleSheet>"#),None).unwrap();
        let b = Styles::parse(Some(r#"<styleSheet><fonts><font><b val="true"/><name val="Calibri"/><sz val="11.0"/></font></fonts><fills><fill><patternFill/></fill></fills><cellXfs><xf fontId="0" fillId="0" numFmtId="0"/></cellXfs></styleSheet>"#),None).unwrap();
        assert_eq!(a.get(0).unwrap().format, b.get(0).unwrap().format);
        assert_eq!(a.get(0).unwrap().format["bold"], "true");
    }
    #[test]
    fn explicit_false_and_default_alignment_match() {
        let a = Styles::parse(Some(r#"<styleSheet><fonts><font><b val="0"/></font></fonts><cellXfs><xf fontId="0"><alignment horizontal="general" vertical="bottom" wrapText="0"/></xf></cellXfs></styleSheet>"#),None).unwrap();
        assert_eq!(a.get(0).unwrap().format, Style::default().format);
    }
    #[test]
    fn formats_distinguish_dates_from_literals_and_duration() {
        assert_eq!(date_type("yyyy-mm-dd"), Some(ExcelDateTimeType::DateTime));
        assert_eq!(date_type("[h]:mm:ss"), Some(ExcelDateTimeType::TimeDelta));
        assert_eq!(date_type(r#"0.00 "days""#), None);
        assert_eq!(date_type("[Red]0.00"), None);
        assert_eq!(date_type("General"), None);
    }
}
