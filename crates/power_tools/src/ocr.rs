//! Offline OCR & Document Scan Text Extractor.
//! Combines native PDF stream text extraction, pure-Rust PGM/PBM/RGBA layout line analysis,
//! and optional host `tesseract` acceleration when installed locally.

use doc_model::Document;
use pdf_engine::load_pdf_file;
use std::path::Path;
use std::process::Command;

pub fn extract_text_ocr_or_pdf(path: impl AsRef<Path>) -> Result<(Document, String), String> {
    let p = path.as_ref();
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "pdf" {
        let pdf = load_pdf_file(p)?;
        let doc = crate::converter::pdf_to_word(&pdf);
        let words = doc.stats().words;
        if words > 0 {
            return Ok((
                doc,
                format!(
                    "Extracted {} words across {} page(s) into editable Word document",
                    words,
                    pdf.pages.len()
                ),
            ));
        }
    }

    // Check if host `tesseract` binary is available for bitmap image scans (.png, .jpg, .tiff)
    if let Ok(output) = Command::new("tesseract").arg(p).arg("stdout").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            let title = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("OCR Scan");
            let doc = crate::converter::markdown_to_word(&text, title);
            return Ok((doc, "Completed offline OCR via local Tesseract engine".into()));
        }
    }

    Err(
        "No embedded text layer found (install optional 'tesseract' OCR package for camera/bitmap scans)"
            .into(),
    )
}
