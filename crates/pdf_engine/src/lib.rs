//! Pure-Rust PDF 1.4–1.7 Parser, Visual Page Model & Full PDF Studio Editing Engine.
//! Supports: Create Blank PDF, Parse PDFs, Inline Text Block Editing, Add Text Boxes,
//! Highlights, Sticky Notes, Stamps, Smooth Ink Signatures, True Redaction, Watermarks,
//! Rotate/Reorder/Delete Pages, Merge PDFs, Split/Extract Page Ranges, and Compress PDF.

pub mod parser;
pub mod writer;

use doc_model::{FontFamily, PaperSize, RgbColor};
use std::path::Path;

pub use parser::{load_pdf_bytes, load_pdf_file};
pub use writer::{compress_pdf_bytes, save_pdf_bytes, save_pdf_file};

#[derive(Debug, Clone, PartialEq)]
pub struct PdfTextItem {
    pub id: u32,
    /// X in PDF points from left edge.
    pub x: f32,
    /// Y in PDF points from bottom edge.
    pub y: f32,
    pub font_family: FontFamily,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub color: RgbColor,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfRectItem {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: Option<RgbColor>,
    pub stroke: Option<(RgbColor, f32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PdfAnnotation {
    Highlight {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: RgbColor,
    },
    StickyNote {
        x: f32,
        y: f32,
        author: String,
        note: String,
    },
    Stamp {
        x: f32,
        y: f32,
        label: String,
        color: RgbColor,
    },
    Signature {
        /// Stroke polylines in PDF page coordinates (x, y from bottom-left).
        strokes: Vec<Vec<(f32, f32)>>,
        signer: String,
        color: RgbColor,
    },
    Redaction {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfPage {
    pub width_pt: f32,
    pub height_pt: f32,
    pub rotation_deg: u16,
    pub text_items: Vec<PdfTextItem>,
    pub rects: Vec<PdfRectItem>,
    pub annotations: Vec<PdfAnnotation>,
    pub watermark: Option<String>,
}

impl PdfPage {
    pub fn blank(paper: PaperSize) -> Self {
        let (w, h) = paper.dimensions_pt(false);
        Self {
            width_pt: w,
            height_pt: h,
            rotation_deg: 0,
            text_items: Vec::new(),
            rects: Vec::new(),
            annotations: Vec::new(),
            watermark: None,
        }
    }

    pub fn next_item_id(&self) -> u32 {
        self.text_items
            .iter()
            .map(|t| t.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    pub fn add_text_box(
        &mut self,
        x: f32,
        y: f32,
        text: impl Into<String>,
        font_size: f32,
        color: RgbColor,
    ) -> u32 {
        let id = self.next_item_id();
        self.text_items.push(PdfTextItem {
            id,
            x,
            y,
            font_family: FontFamily::Sans,
            font_size,
            bold: false,
            italic: false,
            color,
            text: text.into(),
        });
        id
    }

    /// Permanently removes underlying text items overlapping the redaction rectangle
    /// and records an opaque redaction block.
    pub fn apply_redaction(&mut self, rx: f32, ry: f32, rw: f32, rh: f32) -> usize {
        let x0 = rx.min(rx + rw);
        let x1 = rx.max(rx + rw);
        let y0 = ry.min(ry + rh);
        let y1 = ry.max(ry + rh);

        let before = self.text_items.len();
        self.text_items.retain(|item| {
            let approx_w =
                pdf_create::estimate_text_width(&item.text, item.font_size, item.font_family);
            let ix0 = item.x;
            let ix1 = item.x + approx_w;
            let iy0 = item.y - 2.0;
            let iy1 = item.y + item.font_size;
            let overlaps = ix0 < x1 && ix1 > x0 && iy0 < y1 && iy1 > y0;
            !overlaps
        });
        let removed = before - self.text_items.len();
        self.annotations.push(PdfAnnotation::Redaction {
            x: x0,
            y: y0,
            w: (x1 - x0).max(8.0),
            h: (y1 - y0).max(8.0),
        });
        removed
    }

    pub fn plain_text(&self) -> String {
        let mut sorted = self.text_items.clone();
        sorted.sort_by(|a, b| {
            let dy = (b.y - a.y).abs();
            if dy > 4.0 {
                b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
        let mut out = String::new();
        let mut last_y: Option<f32> = None;
        for item in sorted {
            if let Some(py) = last_y {
                if (py - item.y).abs() > 5.0 {
                    out.push('\n');
                } else if !out.ends_with(' ') && !item.text.starts_with(' ') {
                    out.push(' ');
                }
            }
            out.push_str(&item.text);
            last_y = Some(item.y);
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfDocument {
    pub title: String,
    pub author: String,
    pub pages: Vec<PdfPage>,
}

impl PdfDocument {
    pub fn new_blank(title: impl Into<String>, num_pages: usize, paper: PaperSize) -> Self {
        let count = num_pages.max(1);
        let mut pages = Vec::with_capacity(count);
        for i in 0..count {
            let mut p = PdfPage::blank(paper);
            if i == 0 {
                p.add_text_box(
                    64.0,
                    p.height_pt - 92.0,
                    "New PDF Canvas — Click any text or use the toolbar to add text, highlights, stamps & signatures",
                    13.0,
                    RgbColor::INK_SLATE,
                );
            }
            pages.push(p);
        }
        Self {
            title: title.into(),
            author: "Nour Yahyaoui".into(),
            pages,
        }
    }

    pub fn merge(docs: &[PdfDocument], merged_title: impl Into<String>) -> Self {
        let mut pages = Vec::new();
        for d in docs {
            pages.extend(d.pages.iter().cloned());
        }
        if pages.is_empty() {
            pages.push(PdfPage::blank(PaperSize::A4));
        }
        Self {
            title: merged_title.into(),
            author: "Nour Yahyaoui".into(),
            pages,
        }
    }

    pub fn merge_files(paths: &[impl AsRef<Path>], title: impl Into<String>) -> Result<Self, String> {
        let mut loaded = Vec::with_capacity(paths.len());
        for p in paths {
            loaded.push(load_pdf_file(p)?);
        }
        Ok(Self::merge(&loaded, title))
    }

    /// Parses 1-based page range syntax like `"1-3, 5"` into a new `PdfDocument`.
    pub fn extract_ranges(&self, spec: &str) -> Result<Self, String> {
        let indices = parse_page_ranges(spec, self.pages.len())?;
        let pages: Vec<PdfPage> = indices.into_iter().map(|i| self.pages[i].clone()).collect();
        Ok(Self {
            title: format!("{} (Extracted)", self.title),
            author: self.author.clone(),
            pages,
        })
    }

    pub fn rotate_page(&mut self, idx: usize, delta_deg: u16) {
        if let Some(p) = self.pages.get_mut(idx) {
            p.rotation_deg = (p.rotation_deg + delta_deg) % 360;
            if delta_deg % 180 == 90 {
                std::mem::swap(&mut p.width_pt, &mut p.height_pt);
            }
        }
    }

    pub fn move_page(&mut self, from: usize, to: usize) {
        if from < self.pages.len() && to < self.pages.len() && from != to {
            let page = self.pages.remove(from);
            self.pages.insert(to, page);
        }
    }

    pub fn delete_page(&mut self, idx: usize) -> bool {
        if self.pages.len() <= 1 || idx >= self.pages.len() {
            return false;
        }
        self.pages.remove(idx);
        true
    }

    pub fn insert_blank_page(&mut self, after_idx: usize) {
        let pos = (after_idx + 1).min(self.pages.len());
        self.pages.insert(pos, PdfPage::blank(PaperSize::A4));
    }

    pub fn apply_watermark_all(&mut self, text: Option<String>) {
        for p in &mut self.pages {
            p.watermark = text.clone();
        }
    }

    pub fn extract_all_text(&self) -> String {
        self.pages
            .iter()
            .map(|p| p.plain_text())
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

pub fn parse_page_ranges(spec: &str, max_pages: usize) -> Result<Vec<usize>, String> {
    if max_pages == 0 {
        return Err("PDF has no pages".into());
    }
    let mut out = Vec::new();
    for part in spec.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        if let Some((a, b)) = t.split_once('-') {
            let start: usize = a
                .trim()
                .parse()
                .map_err(|_| format!("Invalid page number in range '{t}'"))?;
            let end: usize = b
                .trim()
                .parse()
                .map_err(|_| format!("Invalid page number in range '{t}'"))?;
            if start == 0 || end == 0 || start > end || end > max_pages {
                return Err(format!("Page range '{t}' out of bounds (1..={max_pages})"));
            }
            for p in start..=end {
                out.push(p - 1);
            }
        } else {
            let p: usize = t
                .parse()
                .map_err(|_| format!("Invalid page number '{t}'"))?;
            if p == 0 || p > max_pages {
                return Err(format!("Page {p} out of bounds (1..={max_pages})"));
            }
            out.push(p - 1);
        }
    }
    if out.is_empty() {
        return Err("No valid pages specified".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_model::TemplateKind;
    use pdf_create::{export_document_to_pdf_bytes, PdfCreateOptions};

    #[test]
    fn test_parse_edit_merge_split_sign_redact_compress_pdf() {
        let doc = TemplateKind::WelcomeGuide.build();
        let raw_pdf = export_document_to_pdf_bytes(&doc, &PdfCreateOptions::default());

        let mut parsed = load_pdf_bytes(&raw_pdf).expect("parse generated PDF");
        assert!(!parsed.pages.is_empty());
        let first_text = parsed.pages[0].plain_text();
        assert!(first_text.contains("RustPad"));

        // Inline text edit + signature + stamp
        parsed.pages[0].text_items[0].text = "Edited PDF Heading Inside RustPad".into();
        parsed.pages[0].annotations.push(PdfAnnotation::Signature {
            strokes: vec![vec![(80.0, 120.0), (130.0, 145.0), (190.0, 125.0)]],
            signer: "Nour Yahyaoui".into(),
            color: RgbColor::COBALT,
        });
        parsed.pages[0].annotations.push(PdfAnnotation::Stamp {
            x: 380.0,
            y: 740.0,
            label: "APPROVED".into(),
            color: RgbColor::EMERALD,
        });

        // Merge with a blank 2-page PDF, then split
        let second = PdfDocument::new_blank("Appendix", 2, PaperSize::A4);
        let mut merged = PdfDocument::merge(&[parsed, second], "Merged Studio Deck");
        assert!(merged.pages.len() >= 3);

        // True redaction removes overlapping text item
        let target_x = merged.pages[0].text_items[0].x;
        let target_y = merged.pages[0].text_items[0].y;
        let scrubbed = merged.pages[0].apply_redaction(target_x - 2.0, target_y - 2.0, 260.0, 30.0);
        assert!(scrubbed >= 1);

        let split = merged.extract_ranges("1-2").expect("extract pages 1-2");
        assert_eq!(split.pages.len(), 2);

        // Uncompressed -> Compressed check
        let uncomp = save_pdf_bytes(&split, false);
        let (comp, before, after) = compress_pdf_bytes(&uncomp).expect("compress pdf");
        assert!(after < before);
        let reloaded = load_pdf_bytes(&comp).expect("reload compressed pdf");
        assert_eq!(reloaded.pages.len(), 2);
    }
}
