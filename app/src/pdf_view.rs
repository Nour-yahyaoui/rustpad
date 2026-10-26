//! Native PDF (`.pdf`) Studio View: PDF Editing & Annotation Ribbon, Page Thumbnails Rail,
//! Interactive Vector PDF Page Canvas (Inline Text Edit, Highlights, Sticky Notes, Ink Signatures,
//! Stamps, Redaction, Watermarks), and PDF Inspector Panel.

use crate::commands::CommandId;
use crate::state::{HitRegion, PdfTabState, PdfTool, UiAction};
use pdf_engine::PdfAnnotation;
use rustpad_render::{Color, DaylightTheme, FontBook, FontSlot, Rect, Surface};

pub fn paint_pdf_tab(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    is_focused_pane: bool,
    cursor_blink_on: bool,
    mouse_x: f32,
    mouse_y: f32,
    area: Rect,
    p: &PdfTabState,
) {
    // 1. PDF Studio Ribbon (42px)
    let toolbar_h = 42.0;
    let toolbar_rect = Rect::new(area.x, area.y, area.w, toolbar_h);
    surface.fill_rect(toolbar_rect, DaylightTheme::CHROME);
    surface.fill_rect(
        Rect::new(area.x, area.y + toolbar_h - 1.0, area.w, 1.0),
        DaylightTheme::BORDER,
    );

    if is_focused_pane {
        paint_pdf_toolbar(
            surface,
            fonts,
            hit_regions,
            tab_idx,
            mouse_x,
            mouse_y,
            toolbar_rect,
            p,
        );
    } else {
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            12.5,
            toolbar_rect.x + 16.0,
            toolbar_rect.y + 26.0,
            DaylightTheme::PDF_ACCENT,
            &format!(
                "PDF STUDIO COMPANION — {} ({} page{})",
                p.pdf.title,
                p.pdf.pages.len(),
                if p.pdf.pages.len() == 1 { "" } else { "s" }
            ),
        );
    }

    let body = Rect::new(
        area.x,
        area.y + toolbar_h,
        area.w,
        (area.h - toolbar_h).max(1.0),
    );
    surface.fill_rect(body, DaylightTheme::DESK_BG);

    // 2. Left Page Thumbnails Rail
    let show_thumbs = body.w >= 720.0;
    let thumb_w = if show_thumbs { 184.0 } else { 0.0 };
    if show_thumbs {
        paint_pdf_thumbnails(
            surface,
            fonts,
            hit_regions,
            tab_idx,
            mouse_x,
            mouse_y,
            Rect::new(body.x, body.y, thumb_w, body.h),
            p,
        );
    }

    // 3. Right PDF Inspector Rail when wide enough
    let show_inspector = body.w >= 1120.0;
    let inspector_w = if show_inspector { 224.0 } else { 0.0 };
    if show_inspector {
        paint_pdf_inspector(
            surface,
            fonts,
            hit_regions,
            mouse_x,
            mouse_y,
            Rect::new(body.x + body.w - inspector_w, body.y, inspector_w, body.h),
            p,
        );
    }

    // 4. Center Interactive PDF Page Sheet
    let canvas = Rect::new(
        body.x + thumb_w,
        body.y,
        (body.w - thumb_w - inspector_w).max(1.0),
        body.h,
    );
    surface.set_clip(canvas);

    let page_idx = p.current_page.min(p.pdf.pages.len().saturating_sub(1));
    if let Some(page) = p.pdf.pages.get(page_idx) {
        let fit_w = (canvas.w - 48.0) / page.width_pt;
        let fit_h = (canvas.h - 44.0) / page.height_pt;
        let base_scale = fit_w.min(fit_h).clamp(0.45, 1.15) * p.zoom;
        let sheet_w = page.width_pt * base_scale;
        let sheet_h = page.height_pt * base_scale;
        let sx = canvas.x + ((canvas.w - sheet_w) * 0.5).max(16.0);
        let sy = canvas.y + ((canvas.h - sheet_h) * 0.5).max(18.0) - p.scroll_y;
        let sheet = Rect::new(sx, sy, sheet_w, sheet_h);

        surface.draw_paper_shadow(sheet, 3.0, 14.0, 30);
        surface.fill_rounded_rect(sheet, 3.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(sheet, 3.0, 1.0, DaylightTheme::BORDER);

        // Background click region for adding text/highlight/sticky/redaction
        let click_pdf_x = ((mouse_x - sheet.x) / base_scale).clamp(24.0, page.width_pt - 40.0);
        let click_pdf_y =
            (page.height_pt - ((mouse_y - sheet.y) / base_scale)).clamp(24.0, page.height_pt - 24.0);
        hit_regions.push(HitRegion {
            rect: sheet,
            action: UiAction::ClickPdfPageCanvas {
                tab_idx,
                page_idx,
                pdf_x: click_pdf_x,
                pdf_y: click_pdf_y,
            },
        });

        // Vector rectangles (table borders, rules, fills)
        for r_item in &page.rects {
            let rx = sheet.x + r_item.x * base_scale;
            let ry = sheet.y + (page.height_pt - r_item.y - r_item.h) * base_scale;
            let rw = (r_item.w * base_scale).max(1.0);
            let rh = (r_item.h * base_scale).max(1.0);
            let rr = Rect::new(rx, ry, rw, rh);
            if let Some(fill) = r_item.fill {
                surface.fill_rect(rr, Color::from_doc(fill));
            }
            if let Some((sc, sw)) = r_item.stroke {
                surface.stroke_rounded_rect(rr, 0.0, (sw * base_scale).max(1.0), Color::from_doc(sc));
            }
        }

        // Highlights underneath text
        for ann in &page.annotations {
            if let PdfAnnotation::Highlight { x, y, w, h, color } = ann {
                let hx = sheet.x + x * base_scale;
                let hy = sheet.y + (page.height_pt - y - h) * base_scale;
                surface.fill_rounded_rect(
                    Rect::new(hx, hy, w * base_scale, h * base_scale),
                    3.0,
                    Color::from_doc(*color).with_alpha(165),
                );
            }
        }

        // Optional diagonal page watermark
        if let Some(wm) = &page.watermark {
            let wm_px = (34.0 * base_scale).clamp(16.0, 56.0);
            let wm_w = fonts.measure_text(FontSlot::SansBold, wm, wm_px);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                wm_px,
                sheet.x + (sheet_w - wm_w) * 0.5,
                sheet.y + sheet_h * 0.52,
                Color::rgba(225, 29, 72, 48),
                wm,
            );
        }

        // Editable PDF text items
        for item in &page.text_items {
            let slot = FontSlot::select(item.font_family, item.bold);
            let px_size = (item.font_size * base_scale).clamp(6.0, 48.0);
            let tx = sheet.x + item.x * base_scale;
            let baseline_y = sheet.y + (page.height_pt - item.y) * base_scale;
            let tw = fonts.measure_text(slot, &item.text, px_size).max(22.0);
            let box_rect = Rect::new(tx - 3.0, baseline_y - px_size * 0.9, tw + 6.0, px_size * 1.18);

            let selected = is_focused_pane && p.selected_text_id == Some(item.id);
            if selected {
                surface.fill_rounded_rect(box_rect, 3.0, DaylightTheme::WORD_SOFT);
                surface.stroke_rounded_rect(box_rect, 3.0, 1.5, DaylightTheme::WORD_ACCENT);
            } else if p.tool == PdfTool::SelectEdit && box_rect.contains(mouse_x, mouse_y) {
                surface.stroke_rounded_rect(box_rect, 3.0, 1.0, DaylightTheme::BORDER_STRONG);
            }

            let end_x = surface.draw_text(
                fonts,
                slot,
                px_size,
                tx,
                baseline_y,
                Color::from_doc(item.color),
                &item.text,
            );
            if selected && cursor_blink_on {
                surface.fill_rect(
                    Rect::new(end_x + 1.0, baseline_y - px_size * 0.82, 2.0, px_size),
                    DaylightTheme::WORD_ACCENT,
                );
            }

            if p.tool == PdfTool::SelectEdit {
                hit_regions.push(HitRegion {
                    rect: box_rect,
                    action: UiAction::ClickPdfTextItem {
                        tab_idx,
                        page_idx,
                        item_id: item.id,
                    },
                });
            }
        }

        // Top-layer annotations: Sticky notes, Stamps, Ink Signatures, Redactions
        for ann in &page.annotations {
            match ann {
                PdfAnnotation::Highlight { .. } => {}
                PdfAnnotation::StickyNote { x, y, author, note } => {
                    let nx = sheet.x + x * base_scale;
                    let ny = sheet.y + (page.height_pt - y) * base_scale - 24.0;
                    let card = Rect::new(nx, ny, 168.0 * base_scale, 46.0 * base_scale);
                    surface.draw_paper_shadow(card, 5.0, 6.0, 22);
                    surface.fill_rounded_rect(card, 5.0, Color::rgb(254, 249, 195));
                    surface.stroke_rounded_rect(card, 5.0, 1.2, Color::rgb(234, 179, 8));
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SansBold,
                        10.0 * base_scale,
                        card.x + 7.0,
                        card.y + 14.0 * base_scale,
                        card.w - 14.0,
                        Color::rgb(133, 77, 14),
                        author,
                    );
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SansRegular,
                        10.0 * base_scale,
                        card.x + 7.0,
                        card.y + 30.0 * base_scale,
                        card.w - 14.0,
                        DaylightTheme::INK,
                        note,
                    );
                }
                PdfAnnotation::Stamp {
                    x,
                    y,
                    label,
                    color,
                } => {
                    let sx_pt = sheet.x + x * base_scale;
                    let sy_pt = sheet.y + (page.height_pt - y - 34.0) * base_scale;
                    let sr = Rect::new(sx_pt, sy_pt, 128.0 * base_scale, 34.0 * base_scale);
                    let c = Color::from_doc(*color);
                    surface.fill_rounded_rect(sr, 5.0, c.with_alpha(26));
                    surface.stroke_rounded_rect(sr, 5.0, 2.2, c);
                    let fs = (13.0 * base_scale).clamp(10.0, 22.0);
                    let lw = fonts.measure_text(FontSlot::SansBold, label, fs);
                    surface.draw_text(
                        fonts,
                        FontSlot::SansBold,
                        fs,
                        sr.x + (sr.w - lw) * 0.5,
                        sr.y + sr.h * 0.67,
                        c,
                        label,
                    );
                }
                PdfAnnotation::Signature {
                    strokes,
                    signer,
                    color,
                } => {
                    let ink = Color::from_doc(*color);
                    let mut min_x = f32::MAX;
                    let mut max_y = f32::MIN;
                    for stroke in strokes {
                        for win in stroke.windows(2) {
                            let x0 = sheet.x + win[0].0 * base_scale;
                            let y0 = sheet.y + (page.height_pt - win[0].1) * base_scale;
                            let x1 = sheet.x + win[1].0 * base_scale;
                            let y1 = sheet.y + (page.height_pt - win[1].1) * base_scale;
                            min_x = min_x.min(x0).min(x1);
                            max_y = max_y.max(y0).max(y1);
                            surface.draw_line(x0, y0, x1, y1, (2.2 * base_scale).max(1.6), ink);
                        }
                    }
                    if min_x < f32::MAX {
                        surface.draw_text(
                            fonts,
                            FontSlot::SansRegular,
                            (9.5 * base_scale).max(8.5),
                            min_x,
                            max_y + 13.0 * base_scale,
                            DaylightTheme::INK_SUB,
                            &format!("Signed by {signer}"),
                        );
                    }
                }
                PdfAnnotation::Redaction { x, y, w, h } => {
                    let rx = sheet.x + x * base_scale;
                    let ry = sheet.y + (page.height_pt - y - h) * base_scale;
                    let rr = Rect::new(rx, ry, w * base_scale, h * base_scale);
                    surface.fill_rect(rr, Color::rgb(15, 23, 42));
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::MonoRegular,
                        (9.0 * base_scale).max(8.0),
                        rr.x + 6.0,
                        rr.y + rr.h * 0.72,
                        rr.w - 10.0,
                        Color::rgb(248, 250, 252),
                        "REDACTED",
                    );
                }
            }
        }
    }

    surface.reset_clip();
}

#[allow(clippy::too_many_arguments)]
fn paint_pdf_toolbar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    bar: Rect,
    p: &PdfTabState,
) {
    let tools: &[(&str, PdfTool)] = &[
        ("Select / Edit", PdfTool::SelectEdit),
        ("+ Text Box", PdfTool::AddText),
        ("Highlight", PdfTool::Highlight),
        ("Sticky Note", PdfTool::StickyNote),
        ("Redact", PdfTool::Redact),
    ];

    let mut x = bar.x + 12.0;
    let btn_y = bar.y + 7.0;
    let btn_h = 28.0;

    for (label, tool) in tools {
        let tw = fonts.measure_text(FontSlot::SansBold, label, 11.5);
        let bw = tw + 16.0;
        let r = Rect::new(x, btn_y, bw, btn_h);
        let active = p.tool == *tool;
        let hovered = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            r,
            6.0,
            if active {
                DaylightTheme::PDF_SOFT
            } else if hovered {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(
            r,
            6.0,
            1.0,
            if active {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.5,
            r.x + 8.0,
            r.y + 18.5,
            if active {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::INK
            },
            label,
        );
        hit_regions.push(HitRegion {
            rect: r,
            action: UiAction::SetPdfTool {
                tab_idx,
                tool: *tool,
            },
        });
        x += bw + 5.0;
    }

    x += 6.0;
    let actions: &[(&str, CommandId, bool)] = &[
        ("Sign Ink", CommandId::OpenSignaturePad, true),
        ("Stamp", CommandId::StampApproved, false),
        ("Watermark", CommandId::OpenWatermarkModal, false),
        ("Rotate 90°", CommandId::PdfRotatePage, false),
        ("+ Page", CommandId::PdfAddBlankPage, false),
        ("Split", CommandId::PdfSplitModal, false),
        ("Merge", CommandId::PdfMergeFile, false),
        ("Compress", CommandId::PdfCompress, false),
        ("-> Word .docx", CommandId::ConvertPdfToWord, true),
    ];

    for (label, cmd, accent) in actions {
        let tw = fonts.measure_text(FontSlot::SansBold, label, 11.5);
        let bw = tw + 16.0;
        if x + bw > bar.x + bar.w - 10.0 {
            break;
        }
        let r = Rect::new(x, btn_y, bw, btn_h);
        let hovered = r.contains(mouse_x, mouse_y);
        let is_word = *cmd == CommandId::ConvertPdfToWord;
        let bg = if is_word {
            DaylightTheme::WORD_SOFT
        } else if *accent {
            DaylightTheme::PDF_SOFT
        } else if hovered {
            DaylightTheme::HOVER
        } else {
            DaylightTheme::PAPER
        };
        let fg = if is_word {
            DaylightTheme::WORD_ACCENT
        } else if *accent {
            DaylightTheme::PDF_ACCENT
        } else {
            DaylightTheme::INK
        };
        surface.fill_rounded_rect(r, 6.0, bg);
        surface.stroke_rounded_rect(r, 6.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.5,
            r.x + 8.0,
            r.y + 18.5,
            fg,
            label,
        );
        hit_regions.push(HitRegion {
            rect: r,
            action: UiAction::ExecCommand(*cmd),
        });
        x += bw + 5.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_pdf_thumbnails(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    rail: Rect,
    p: &PdfTabState,
) {
    surface.fill_rect(rail, DaylightTheme::SIDEBAR);
    surface.fill_rect(
        Rect::new(rail.x + rail.w - 1.0, rail.y, 1.0, rail.h),
        DaylightTheme::BORDER,
    );
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        11.0,
        rail.x + 14.0,
        rail.y + 24.0,
        DaylightTheme::INK_MUTED,
        &format!("PAGES ({})", p.pdf.pages.len()),
    );

    let mut y = rail.y + 38.0;
    for (idx, pg) in p.pdf.pages.iter().enumerate() {
        if y + 114.0 > rail.y + rail.h - 8.0 {
            break;
        }
        let card = Rect::new(rail.x + 16.0, y, rail.w - 32.0, 102.0);
        let active = idx == p.current_page;
        let hovered = card.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            card,
            7.0,
            if active {
                DaylightTheme::PDF_SOFT
            } else if hovered {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(
            card,
            7.0,
            if active { 2.0 } else { 1.0 },
            if active {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );

        // Mini sheet preview inside thumbnail
        let mini = Rect::new(card.x + 10.0, card.y + 8.0, 56.0, 76.0);
        surface.fill_rect(mini, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(mini, 2.0, 1.0, DaylightTheme::BORDER);
        for line_i in 0..pg.text_items.len().min(5) {
            surface.fill_rect(
                Rect::new(mini.x + 6.0, mini.y + 10.0 + (line_i as f32) * 11.0, 42.0, 3.0),
                DaylightTheme::BORDER_STRONG,
            );
        }
        if !pg.annotations.is_empty() {
            surface.fill_rounded_rect(
                Rect::new(mini.x + 30.0, mini.y + 56.0, 20.0, 10.0),
                2.0,
                DaylightTheme::PDF_ACCENT.with_alpha(180),
            );
        }

        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            12.0,
            mini.x + mini.w + 10.0,
            card.y + 26.0,
            if active {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::INK
            },
            &format!("Page {}", idx + 1),
        );
        surface.draw_text(
            fonts,
            FontSlot::SansRegular,
            10.5,
            mini.x + mini.w + 10.0,
            card.y + 44.0,
            DaylightTheme::INK_MUTED,
            &format!("{}° · {} items", pg.rotation_deg, pg.text_items.len()),
        );

        // Reorder buttons when multiple pages exist
        if idx > 0 {
            let up_btn = Rect::new(mini.x + mini.w + 10.0, card.y + 58.0, 28.0, 22.0);
            surface.fill_rounded_rect(up_btn, 4.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(up_btn, 4.0, 1.0, DaylightTheme::BORDER);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                10.5,
                up_btn.x + 7.0,
                up_btn.y + 15.0,
                DaylightTheme::INK_SUB,
                "Up",
            );
            hit_regions.push(HitRegion {
                rect: up_btn,
                action: UiAction::MovePdfPage {
                    tab_idx,
                    from: idx,
                    to: idx - 1,
                },
            });
        }

        hit_regions.push(HitRegion {
            rect: card,
            action: UiAction::SelectPdfPage {
                tab_idx,
                page_idx: idx,
            },
        });
        y += 112.0;
    }
}

fn paint_pdf_inspector(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    panel: Rect,
    p: &PdfTabState,
) {
    surface.fill_rect(panel, DaylightTheme::SIDEBAR);
    surface.fill_rect(
        Rect::new(panel.x, panel.y, 1.0, panel.h),
        DaylightTheme::BORDER,
    );
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        11.0,
        panel.x + 16.0,
        panel.y + 24.0,
        DaylightTheme::PDF_ACCENT,
        "PDF STUDIO INSPECTOR",
    );

    let cur_pg = p.pdf.pages.get(p.current_page);
    let text_cnt: usize = p.pdf.pages.iter().map(|pg| pg.text_items.len()).sum();
    let ann_cnt: usize = p.pdf.pages.iter().map(|pg| pg.annotations.len()).sum();
    let dims = cur_pg
        .map(|pg| format!("{:.0} × {:.0} pt", pg.width_pt, pg.height_pt))
        .unwrap_or_else(|| "595 × 842 pt".into());

    let info = [
        format!("Active tool: {}", p.tool.label()),
        format!("Page size: {dims}"),
        format!("Total pages: {}", p.pdf.pages.len()),
        format!("Text blocks: {text_cnt}"),
        format!("Annotations / Ink: {ann_cnt}"),
        format!("Zoom: {}%", (p.zoom * 100.0).round() as i32),
    ];
    for (i, line) in info.iter().enumerate() {
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansRegular,
            12.0,
            panel.x + 16.0,
            panel.y + 50.0 + (i as f32) * 21.0,
            panel.w - 28.0,
            DaylightTheme::INK_SUB,
            line,
        );
    }

    let ctas = [
        ("Draw Ink Signature", CommandId::OpenSignaturePad),
        ("Stamp [APPROVED]", CommandId::StampApproved),
        ("Compress PDF Streams", CommandId::PdfCompress),
        ("Convert PDF -> Word", CommandId::ConvertPdfToWord),
    ];
    let mut cy = panel.y + 194.0;
    for (lbl, cmd) in ctas {
        let btn = Rect::new(panel.x + 14.0, cy, panel.w - 28.0, 32.0);
        let hov = btn.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            btn,
            6.0,
            if hov {
                DaylightTheme::PDF_SOFT
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(btn, 6.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.5,
            btn.x + 12.0,
            btn.y + 20.5,
            DaylightTheme::INK,
            lbl,
        );
        hit_regions.push(HitRegion {
            rect: btn,
            action: UiAction::ExecCommand(cmd),
        });
        cy += 40.0;
    }
}
