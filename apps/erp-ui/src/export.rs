use erp_grid_core::{Column, ValueKind, money_minor};
use std::io::Write;
#[path = "export_image.rs"]
mod image;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Csv,
    Xlsx,
    Html,
}

/// One worksheet in an exported workbook. Rows must have the same width as
/// `columns`; the first row in every sheet is generated from the column titles.
#[derive(Clone, Debug)]
pub struct WorkbookSheet {
    pub name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct SheetOptions {
    pub images: Vec<SheetImage>,
    pub bold_rows: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct SheetImage {
    /// Zero-based data row, excluding the generated header row.
    pub row: usize,
    pub column: usize,
    pub png: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
}

const MAX_WORKBOOK_SHEETS: usize = 32;
const MAX_EXPORT_CELLS: usize = 100_000;
const MAX_EXPORT_TEXT: usize = 8_388_608;
pub fn encode(columns: &[Column], rows: &[Vec<String>], format: Format) -> Result<Vec<u8>, String> {
    validate_sheet("Data", columns, rows, MAX_EXPORT_CELLS, MAX_EXPORT_TEXT)?;
    match format {
        Format::Csv => {
            fn csv(value: &str, kind: ValueKind) -> String {
                let value = erp_grid_core::spreadsheet_value(value, kind);
                format!("\"{}\"", value.replace('"', "\"\""))
            }
            let mut out = String::from("\u{feff}");
            out.push_str(
                &columns
                    .iter()
                    .map(|c| csv(&c.title, ValueKind::Text))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push_str("\r\n");
            for row in rows {
                out.push_str(
                    &row.iter()
                        .zip(columns)
                        .map(|(value, column)| csv(value, column.value_kind))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                out.push_str("\r\n");
            }
            Ok(out.into_bytes())
        }
        Format::Xlsx => encode_workbook(&[WorkbookSheet {
            name: "Data".into(),
            columns: columns.to_vec(),
            rows: rows.to_vec(),
        }]),
        Format::Html => {
            let repeats = columns.len().saturating_sub(1).div_ceil(7).max(1) - 1;
            let repeated_text = rows
                .iter()
                .filter_map(|row| row.first())
                .map(String::len)
                .sum::<usize>();
            let text = rows.iter().flatten().map(String::len).sum::<usize>();
            if rows
                .len()
                .saturating_mul(columns.len().saturating_add(repeats))
                > MAX_EXPORT_CELLS
                || text.saturating_add(repeated_text.saturating_mul(repeats)) > MAX_EXPORT_TEXT
            {
                return Err(
                    "Print bands exceed export budget; choose fewer rows or columns".into(),
                );
            }
            Ok(printable_html(columns, rows).into_bytes())
        }
    }
}

fn html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn printable_html(columns: &[Column], rows: &[Vec<String>]) -> String {
    let mut out = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Print preview</title><style>@page{size:landscape;margin:12mm}*{box-sizing:border-box}body{margin:0;color:#111;font:12px/1.35 sans-serif}.toolbar{padding:10px 0}button{padding:6px 12px}table{width:100%;table-layout:fixed;border-collapse:collapse}.column-band{break-before:page;page-break-before:always;margin-top:16px}thead{display:table-header-group}tr{break-inside:avoid;page-break-inside:avoid}th,td{border:1px solid #999;padding:4px 6px;text-align:left;vertical-align:top;white-space:pre-wrap;overflow-wrap:anywhere}th{background:#eee;font-weight:600}@media print{.toolbar{display:none}}</style></head><body><div class=\"toolbar\"><button type=\"button\" onclick=\"window.print()\">Print</button></div><table><thead><tr>",
    );
    // Bound printed width without omitting fields or shrinking to unreadable text.
    // Repeating the first column lets the reader correlate rows between bands.
    let bands = columns.len().saturating_sub(1).div_ceil(7).max(1);
    for band in 0..bands {
        if band > 0 {
            out.push_str("<table class=\"column-band\"><thead><tr>");
        }
        let indices: Vec<_> = (0..columns.len().min(1))
            .chain((1 + band * 7)..columns.len().min(1 + (band + 1) * 7))
            .collect();
        for &index in &indices {
            out.push_str("<th>");
            out.push_str(&html(&columns[index].title));
            out.push_str("</th>");
        }
        out.push_str("</tr></thead><tbody>");
        for row in rows {
            out.push_str("<tr>");
            for &index in &indices {
                out.push_str("<td>");
                out.push_str(&html(&row[index]));
                out.push_str("</td>");
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table>");
    }
    out.push_str("</body></html>");
    out
}

/// Encode several independent sheets into one XLSX archive.
pub fn encode_workbook(sheets: &[WorkbookSheet]) -> Result<Vec<u8>, String> {
    if sheets.len() > MAX_WORKBOOK_SHEETS {
        return Err(format!("Workbook limit: {MAX_WORKBOOK_SHEETS} sheets"));
    }
    encode_workbook_with_options(sheets, &vec![SheetOptions::default(); sheets.len()])
}

/// Encode worksheets with optional embedded PNGs and bold data rows.
pub fn encode_workbook_with_options(
    sheets: &[WorkbookSheet],
    options: &[SheetOptions],
) -> Result<Vec<u8>, String> {
    if sheets.is_empty() {
        return Err("Workbook must contain at least one sheet".into());
    }
    if sheets.len() > MAX_WORKBOOK_SHEETS {
        return Err(format!("Workbook limit: {MAX_WORKBOOK_SHEETS} sheets"));
    }
    if options.len() != sheets.len() {
        return Err("Workbook options must match the worksheet count".into());
    }
    let mut cells = 0usize;
    let mut text = 0usize;
    let mut image_count = 0usize;
    let mut image_bytes = 0usize;
    let mut names = std::collections::HashSet::new();
    for (sheet, sheet_options) in sheets.iter().zip(options) {
        validate_sheet_name(&sheet.name)?;
        if !names.insert(sheet.name.to_lowercase()) {
            return Err(format!("Duplicate worksheet name: {}", sheet.name));
        }
        validate_sheet(
            &sheet.name,
            &sheet.columns,
            &sheet.rows,
            usize::MAX,
            usize::MAX,
        )?;
        cells = cells.saturating_add((sheet.rows.len() + 1).saturating_mul(sheet.columns.len()));
        text = text.saturating_add(
            sheet.columns.iter().map(|c| c.title.len()).sum::<usize>()
                + sheet.rows.iter().flatten().map(String::len).sum::<usize>(),
        );
        let bold_rows = sheet_options
            .bold_rows
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if bold_rows.len() != sheet_options.bold_rows.len() {
            return Err(format!("Worksheet {} has duplicate bold rows", sheet.name));
        }
        for row in bold_rows {
            if row >= sheet.rows.len() {
                return Err(format!("Worksheet {} has an invalid bold row", sheet.name));
            }
        }
        for image in &sheet_options.images {
            image_count = image_count.saturating_add(1);
            if image_count > 128 {
                return Err("Workbook limit: 128 images".into());
            }
            image_bytes = image_bytes.saturating_add(image.png.len());
            if image_bytes > 8_388_608 {
                return Err("Workbook limit: 8 MiB of PNG data".into());
            }
            if image.row >= sheet.rows.len() || image.column >= sheet.columns.len() {
                return Err(format!(
                    "Worksheet {} has an image outside its data",
                    sheet.name
                ));
            }
            if image.width_px == 0
                || image.height_px == 0
                || image.width_px > 1024
                || image.height_px > 1024
            {
                return Err("Image dimensions must be between 1 and 1024 pixels".into());
            }
            image::validate_png(&image.png)?;
        }
    }
    if cells > MAX_EXPORT_CELLS {
        return Err("Export limit: 100,000 cells".into());
    }
    if text > MAX_EXPORT_TEXT {
        return Err("Export limit: 8 MiB of cell text".into());
    }
    xlsx_workbook(sheets, options)
}

fn validate_sheet_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed != name {
        return Err("Worksheet name cannot have leading or trailing whitespace".into());
    }
    if trimmed.is_empty() || trimmed.chars().count() > 31 {
        return Err("Worksheet name must contain 1-31 characters".into());
    }
    if trimmed.starts_with('\'') || trimmed.ends_with('\'') {
        return Err("Worksheet name cannot start or end with an apostrophe".into());
    }
    if trimmed
        .chars()
        .any(|c| c.is_control() || matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']'))
    {
        return Err(format!("Invalid worksheet name: {name}"));
    }
    Ok(())
}

fn validate_sheet(
    name: &str,
    columns: &[Column],
    rows: &[Vec<String>],
    cell_limit: usize,
    text_limit: usize,
) -> Result<(), String> {
    if columns.len() > 16_384 {
        return Err(format!("Worksheet {name} has invalid column count"));
    }
    if rows.len() >= 1_048_576 {
        return Err(format!("Worksheet {name} has too many rows"));
    }
    if rows.iter().any(|row| row.len() != columns.len()) {
        return Err("Export row schema mismatch".into());
    }
    if rows.len().saturating_mul(columns.len()) > cell_limit {
        return Err("Export limit: 100,000 cells".into());
    }
    if columns
        .iter()
        .map(|column| column.title.encode_utf16().count())
        .chain(
            rows.iter()
                .flatten()
                .map(|value| value.encode_utf16().count()),
        )
        .any(|length| length > 32_767)
    {
        return Err("Excel cell text limit: 32,767 UTF-16 code units".into());
    }
    if rows.iter().flatten().map(String::len).sum::<usize>() > text_limit {
        return Err("Export limit: 8 MiB of cell text".into());
    }
    Ok(())
}

fn safe_numeric(value: &str, kind: ValueKind) -> bool {
    if !matches!(kind, ValueKind::Integer | ValueKind::Money)
        || value.is_empty()
        || value
            .chars()
            .any(|character| matches!(character, 'e' | 'E'))
        || value.chars().filter(|c| c.is_ascii_digit()).count() > 15
    {
        return false;
    }
    let body = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    let decimal_parts = body.split('.').collect::<Vec<_>>();
    if decimal_parts.len() > 2
        || decimal_parts
            .iter()
            .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    match kind {
        ValueKind::Integer => decimal_parts.len() == 1 && value.parse::<i64>().is_ok(),
        ValueKind::Money => money_minor(value).is_ok_and(|minor| minor.is_some()),
        _ => false,
    }
}
fn xml(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c >= ' ' || matches!(*c, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn xlsx_workbook(
    sheets: &[WorkbookSheet],
    sheet_options: &[SheetOptions],
) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    );
    for i in 1..=sheets.len() {
        types.push_str(&format!(r#"<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#));
        if !sheet_options[i - 1].images.is_empty() {
            types.push_str(&format!(r#"<Override PartName="/xl/drawings/drawing{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#));
        }
    }
    types.push_str("</Types>");
    let mut workbook = String::from(
        r#"<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
    );
    let mut rels = String::from(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (i, source) in sheets.iter().enumerate() {
        let id = i + 1;
        workbook.push_str(&format!(
            r#"<sheet name="{}" sheetId="{id}" r:id="rId{id}"/>"#,
            xml(&source.name)
        ));
        rels.push_str(&format!(r#"<Relationship Id="rId{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{id}.xml"/>"#));
    }
    workbook.push_str("</sheets></workbook>");
    let styles_rel_id = sheets.len() + 1;
    rels.push_str(&format!(r#"<Relationship Id="rId{styles_rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#));
    rels.push_str("</Relationships>");
    let styles = r##"<?xml version="1.0"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="#,##0.00##############"/></numFmts><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFD9EAF7"/><bgColor indexed="64"/></patternFill></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="6"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="0" applyFont="1" applyFill="1"/><xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" applyFont="1"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" applyFont="1"/><xf numFmtId="164" fontId="1" fillId="0" borderId="0" applyFont="1" applyNumberFormat="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"##;
    let mut entries: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), types.into_bytes()),
        ("_rels/.rels".into(), br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_vec()),
        ("xl/workbook.xml".into(), workbook.into_bytes()),
        ("xl/_rels/workbook.xml.rels".into(), rels.into_bytes()),
        ("xl/styles.xml".into(), styles.as_bytes().to_vec()),
    ];
    let mut image_id = 0usize;
    for (sheet_id, (source, rich)) in sheets.iter().zip(sheet_options).enumerate() {
        let bold_rows = rich
            .bold_rows
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut sheet = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews>"#,
        );
        sheet.push_str("<cols>");
        for (column_index, column) in source.columns.iter().enumerate() {
            let image_width = rich
                .images
                .iter()
                .filter(|image| image.column == column_index)
                .map(|image| image.width_px.saturating_add(8))
                .max()
                .unwrap_or(0);
            let width_px = f64::from(column.width().max(image_width as f32));
            let width = (width_px / 7.0).clamp(1.0, 255.0);
            sheet.push_str(&format!(
                r#"<col min="{}" max="{}" width="{width:.2}" customWidth="1"/>"#,
                column_index + 1,
                column_index + 1
            ));
        }
        sheet.push_str("</cols><sheetData>");
        let headers: Vec<_> = source.columns.iter().map(|c| c.title.clone()).collect();
        for (index, row) in std::iter::once(&headers)
            .chain(source.rows.iter())
            .enumerate()
        {
            let row_height = index.checked_sub(1).and_then(|data_row| {
                rich.images
                    .iter()
                    .filter(|image| image.row == data_row)
                    .map(|image| (f64::from(image.height_px) * 0.75 + 6.0).min(409.5))
                    .max_by(f64::total_cmp)
            });
            if let Some(height) = row_height {
                // Excel caps explicit row height at 409.5 pt; taller images may span rows.
                sheet.push_str(&format!(
                    "<row r=\"{}\" ht=\"{height:.2}\" customHeight=\"1\">",
                    index + 1
                ));
            } else {
                sheet.push_str(&format!("<row r=\"{}\">", index + 1));
            }
            for (column, value) in row.iter().enumerate() {
                let bold = index > 0 && bold_rows.contains(&(index - 1));
                let numeric = index > 0
                    && source
                        .columns
                        .get(column)
                        .is_some_and(|column| safe_numeric(value, column.value_kind));
                if numeric {
                    let style = if source.columns[column].value_kind == ValueKind::Money {
                        if bold { 5 } else { 2 }
                    } else if bold {
                        4
                    } else {
                        0
                    };
                    sheet.push_str(&format!("<c s=\"{style}\"><v>{}</v></c>", xml(value)));
                } else {
                    sheet.push_str(&format!(
                        "<c s=\"{}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                        if index == 0 {
                            1
                        } else if bold {
                            3
                        } else {
                            0
                        },
                        xml(value)
                    ));
                }
            }
            sheet.push_str("</row>");
        }
        sheet.push_str("</sheetData>");
        if !rich.images.is_empty() {
            sheet.push_str(r#"<drawing xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId1"/>"#);
        }
        sheet.push_str("</worksheet>");
        entries.push((
            format!("xl/worksheets/sheet{}.xml", sheet_id + 1),
            sheet.into_bytes(),
        ));
        if !rich.images.is_empty() {
            let mut drawing = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?><xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
            );
            let mut drawing_rels = String::from(
                r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            );
            for (local_id, image) in rich.images.iter().enumerate() {
                image_id += 1;
                let relationship_id = local_id + 1;
                let column = image.column;
                let row = image.row + 1;
                let cx = u64::from(image.width_px) * 9_525;
                let cy = u64::from(image.height_px) * 9_525;
                drawing.push_str(&format!(r#"<xdr:oneCellAnchor><xdr:from><xdr:col>{column}</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>{row}</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:ext cx="{cx}" cy="{cy}"/><xdr:pic><xdr:nvPicPr><xdr:cNvPr id="{relationship_id}" name="Image {image_id}"/><xdr:cNvPicPr/></xdr:nvPicPr><xdr:blipFill><a:blip r:embed="rId{relationship_id}"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill><xdr:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr></xdr:pic><xdr:clientData/></xdr:oneCellAnchor>"#));
                drawing_rels.push_str(&format!(r#"<Relationship Id="rId{relationship_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{image_id}.png"/>"#));
                entries.push((format!("xl/media/image{image_id}.png"), image.png.clone()));
            }
            drawing.push_str("</xdr:wsDr>");
            drawing_rels.push_str("</Relationships>");
            entries.push((
                format!("xl/drawings/drawing{}.xml", sheet_id + 1),
                drawing.into_bytes(),
            ));
            entries.push((
                format!("xl/drawings/_rels/drawing{}.xml.rels", sheet_id + 1),
                drawing_rels.into_bytes(),
            ));
            entries.push((
                format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_id + 1),
                format!(r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing{}.xml"/></Relationships>"#, sheet_id + 1).into_bytes(),
            ));
        }
    }
    for (name, bytes) in entries {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    zip.finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|e| e.to_string())
}

pub struct Delivery {
    result: std::sync::mpsc::Receiver<Result<String, String>>,
}
impl Delivery {
    pub fn start(bytes: Vec<u8>, format: Format) -> Self {
        let (send, result) = std::sync::mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let save = || -> Result<String, String> {
                let base = std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(std::path::PathBuf::from)
                    .ok_or("Home directory unavailable")?
                    .join("Downloads");
                std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_millis();
                let extension = match format {
                    Format::Csv => "csv",
                    Format::Xlsx => "xlsx",
                    Format::Html => "html",
                };
                let path = base.join(format!("erp-grid-{stamp}.{extension}"));
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                file.write_all(&bytes).map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
                Ok(path.display().to_string())
            };
            let _ = send.send(save());
        });
        #[cfg(target_arch = "wasm32")]
        {
            let download = || -> Result<String, String> {
                use wasm_bindgen::JsCast;
                let parts = js_sys::Array::new();
                parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));
                let options = web_sys::BlobPropertyBag::new();
                options.set_type(match format {
                    Format::Csv => "text/csv;charset=utf-8",
                    Format::Xlsx => {
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    }
                    Format::Html => "text/html;charset=utf-8",
                });
                let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
                    .map_err(|e| format!("{e:?}"))?;
                let url = web_sys::Url::create_object_url_with_blob(&blob)
                    .map_err(|e| format!("{e:?}"))?;
                let result: Result<String, &str> = (|| {
                    let document = web_sys::window()
                        .and_then(|w| w.document())
                        .ok_or("Document unavailable")?;
                    let anchor = document
                        .create_element("a")
                        .map_err(|_| "Cannot create download")?
                        .dyn_into::<web_sys::HtmlAnchorElement>()
                        .map_err(|_| "Invalid anchor")?;
                    anchor.set_href(&url);
                    anchor.set_download(match format {
                        Format::Csv => "erp-grid.csv",
                        Format::Xlsx => "erp-grid.xlsx",
                        Format::Html => "erp-grid.html",
                    });
                    let body = document.body().ok_or("Body unavailable")?;
                    body.append_child(&anchor)
                        .map_err(|_| "Cannot attach download")?;
                    anchor.click();
                    anchor.remove();
                    Ok("Download requested".into())
                })();
                // Release after the browser has consumed the URL, not during the click.
                let cleanup = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let _ = web_sys::Url::revoke_object_url(&url);
                });
                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cleanup.unchecked_ref(),
                        1000,
                    );
                }
                result.map_err(String::from)
            };
            let _ = send.send(download());
        }
        Self { result }
    }
    pub fn poll(&self) -> Option<Result<String, String>> {
        self.result.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xlsx_round_trip_preserves_long_ids_formulas_and_numeric_amounts() {
        use calamine::{Data, Reader};
        let mut columns = vec![
            Column::new(erp_grid_core::ColumnId(0), "ID", 100.0),
            Column::new(erp_grid_core::ColumnId(1), "Amount", 100.0),
        ];
        columns[1].value_kind = ValueKind::Money;
        let rows = vec![
            vec!["00012345678901234567890".into(), "12.34".into()],
            vec!["=1+2".into(), "-1.20".into()],
        ];
        let bytes = encode(&columns, &rows, Format::Xlsx).unwrap();
        let mut workbook = calamine::Xlsx::new(std::io::Cursor::new(bytes)).unwrap();
        let range = workbook.worksheet_range("Data").unwrap();
        assert_eq!(
            range.get_value((1, 0)),
            Some(&Data::String("00012345678901234567890".into()))
        );
        assert_eq!(range.get_value((2, 0)), Some(&Data::String("=1+2".into())));
        assert_eq!(range.get_value((1, 1)), Some(&Data::Float(12.34)));
        let csv = String::from_utf8(encode(&columns, &rows, Format::Csv).unwrap()).unwrap();
        assert!(csv.contains("'=1+2"));
        assert!(csv.starts_with('\u{feff}'));
    }

    #[test]
    fn workbook_round_trip_has_two_named_sheets_and_rejects_bad_names() {
        use calamine::{Data, Reader};
        let columns = vec![Column::new(erp_grid_core::ColumnId(0), "Code", 100.0)];
        let bytes = encode_workbook(&[
            WorkbookSheet {
                name: "Orders".into(),
                columns: columns.clone(),
                rows: vec![vec!["00012345678901234567890".into()]],
            },
            WorkbookSheet {
                name: "Goods".into(),
                columns,
                rows: vec![vec!["Widget".into()]],
            },
        ])
        .unwrap();
        let mut workbook = calamine::Xlsx::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(
            workbook.sheet_names(),
            vec!["Orders".to_string(), "Goods".to_string()]
        );
        assert_eq!(
            workbook
                .worksheet_range("Orders")
                .unwrap()
                .get_value((1, 0)),
            Some(&Data::String("00012345678901234567890".into()))
        );
        assert_eq!(
            workbook.worksheet_range("Goods").unwrap().get_value((1, 0)),
            Some(&Data::String("Widget".into()))
        );
        let bad = WorkbookSheet {
            name: "bad/name".into(),
            columns: vec![Column::new(erp_grid_core::ColumnId(0), "x", 1.0)],
            rows: vec![],
        };
        assert!(encode_workbook(&[bad]).is_err());
    }

    #[test]
    fn workbook_rejects_unsafe_names_numbers_and_cell_text() {
        let mut integer = Column::new(erp_grid_core::ColumnId(0), "id", 1.0);
        integer.value_kind = ValueKind::Integer;
        let sheet = |name: &str, value: &str| WorkbookSheet {
            name: name.into(),
            columns: vec![integer.clone()],
            rows: vec![vec![value.into()]],
        };
        assert!(encode_workbook(&[sheet("A\nB", "1")]).is_err());
        assert!(encode_workbook(&[sheet("Data", "1e30")]).is_ok());
        assert!(!safe_numeric("1e30", ValueKind::Integer));
        assert!(!safe_numeric("NaN", ValueKind::Money));
        assert!(encode_workbook(&[sheet("Data", "1"), sheet("data", "2")]).is_err());
        assert!(encode_workbook(&[sheet("Data", &"x".repeat(32_768))]).is_err());
    }

    #[test]
    fn rich_workbook_packages_image_relationships_and_styles() {
        use std::io::Read;
        fn entry(archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, name: &str) -> String {
            let mut text = String::new();
            archive
                .by_name(name)
                .unwrap()
                .read_to_string(&mut text)
                .unwrap();
            text
        }

        let mut amount = Column::new(erp_grid_core::ColumnId(0), "Amount", 98.0);
        amount.value_kind = ValueKind::Money;
        let sheets = [WorkbookSheet {
            name: "Totals".into(),
            columns: vec![amount],
            rows: vec![vec!["12.34".into()]],
        }];
        let options = [SheetOptions {
            images: vec![SheetImage {
                row: 0,
                column: 0,
                png: include_bytes!("../assets/export-sample.png").to_vec(),
                width_px: 16,
                height_px: 16,
            }],
            bold_rows: vec![0],
        }];
        let bytes = encode_workbook_with_options(&sheets, &options).unwrap();
        if let Some(path) = std::env::var_os("ERP_GRID_EXPORT_QA_PATH") {
            std::fs::write(path, &bytes).unwrap();
        }
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert!(archive.by_name("xl/media/image1.png").is_ok());
        assert!(entry(&mut archive, "xl/_rels/workbook.xml.rels").contains("relationships/styles"));
        assert!(
            entry(&mut archive, "xl/worksheets/_rels/sheet1.xml.rels")
                .contains("../drawings/drawing1.xml")
        );
        assert!(
            entry(&mut archive, "xl/drawings/_rels/drawing1.xml.rels")
                .contains("../media/image1.png")
        );
        let styles = entry(&mut archive, "xl/styles.xml");
        assert!(styles.contains("patternType=\"gray125\""));
        assert!(styles.contains("<cellStyle name=\"Normal\""));
        let sheet = entry(&mut archive, "xl/worksheets/sheet1.xml");
        assert!(sheet.contains("s=\"5\"><v>12.34</v>"));
        assert!(sheet.contains("customHeight=\"1\""));
    }

    #[test]
    fn rich_workbook_rejects_invalid_options_before_packaging() {
        let sheet = WorkbookSheet {
            name: "Data".into(),
            columns: vec![Column::new(erp_grid_core::ColumnId(0), "Code", 100.0)],
            rows: vec![vec!["A".into()]],
        };
        assert!(encode_workbook_with_options(std::slice::from_ref(&sheet), &[]).is_err());
        assert!(
            encode_workbook_with_options(
                std::slice::from_ref(&sheet),
                &[SheetOptions {
                    images: vec![],
                    bold_rows: vec![1]
                }]
            )
            .is_err()
        );
        assert!(
            encode_workbook_with_options(
                &[sheet],
                &[SheetOptions {
                    images: vec![SheetImage {
                        row: 0,
                        column: 0,
                        png: vec![0; 8],
                        width_px: 16,
                        height_px: 16,
                    }],
                    bold_rows: vec![],
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn printable_html_bands_preserve_all_fields_and_repeat_first_column() {
        let columns: Vec<_> = (0..100)
            .map(|i| Column::new(erp_grid_core::ColumnId(i), format!("Field {i}"), 100.0))
            .collect();
        let rows = vec![(0..100).map(|i| format!("中文值-{i}")).collect()];
        let document = String::from_utf8(encode(&columns, &rows, Format::Html).unwrap()).unwrap();
        let tables: Vec<_> = document.split("<table").skip(1).collect();
        assert_eq!(tables.len(), 15);
        for table in tables {
            assert!(table.matches("<th>").count() <= 8);
            assert!(table.contains("<td>中文值-0</td>"));
        }
        for i in 1..100 {
            assert_eq!(document.matches(&format!("<td>中文值-{i}</td>")).count(), 1);
        }
        assert!(document.contains("table-layout:fixed"));
        let oversized = vec![vec![String::new(); 100]; 1000];
        assert!(encode(&columns, &oversized, Format::Html).is_err());
    }

    #[test]
    fn printable_html_escapes_titles_and_cells_and_preserves_utf8_text() {
        let columns = vec![
            Column::new(
                erp_grid_core::ColumnId(0),
                "商品 <script>alert(1)</script>",
                100.0,
            ),
            Column::new(erp_grid_core::ColumnId(1), "金额 & 备注", 100.0),
        ];
        let rows = vec![vec![
            "中文商品 </td><script>alert('cell')</script>".into(),
            "0012.3400 & \"quoted\"".into(),
        ]];
        let document = String::from_utf8(encode(&columns, &rows, Format::Html).unwrap()).unwrap();
        assert!(document.contains("商品 &lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(
            document
                .contains("中文商品 &lt;/td&gt;&lt;script&gt;alert(&#39;cell&#39;)&lt;/script&gt;")
        );
        assert!(document.contains("0012.3400 &amp; &quot;quoted&quot;"));
        assert_eq!(document.matches("<script").count(), 0);
        assert!(document.contains("onclick=\"window.print()\""));
    }

    #[test]
    fn printable_html_uses_the_existing_schema_and_size_limits() {
        let columns = vec![Column::new(erp_grid_core::ColumnId(0), "Code", 100.0)];
        assert!(encode(&columns, &[vec![]], Format::Html).is_err());
        let too_many = vec![vec![String::new()]; MAX_EXPORT_CELLS + 1];
        assert!(encode(&columns, &too_many, Format::Html).is_err());
    }
}
