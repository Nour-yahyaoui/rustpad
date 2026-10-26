//! PDF Studio (`.pdf`) Canvas, Drag-Resizable Page Thumbnails & Inspector Rails,
//! Inline Text Editing, Ink Signature / Stamp / Highlight / Redact Overlay & Scrollbar.

use crate::commands::CommandId;
use crate::state::{DragInteraction, HitRegion, PdfTabState, PdfTool, UiAction};
use crate::word_view::paint_canvas_scrollbar;
use pdf_engine::PdfAnnotation;
use rustpad_render::{Color, DaylightTheme, FontBook, FontSlot, Rect, Surface, UiIcon};

#[allow(clippy::too_many_arguments)]
pub fn paint_pdf_tab(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    is_focused: bool,
    cursor_blink_on: bool,
    mouse_x: f32,
    mouse_y: f32,
    ui_scale: f32,
    left_open: bool,
    left_w: f32,
    right_open: bool,
    right_w: f32,
    area: Rect,
    p: &mut PdfTabState,
) {
    let s = ui_scale.clamp(0.85, 1.6);
    let toolbar_h = (44.0 * s).round();
    let toolbar_rect = Rect::new(area.x, area.y, area.w, toolbar_h);
    surface.fill_rect(toolbar_rect, DaylightTheme::CHROME);
    surface.fill_rect(
        Rect::new(
            toolbar_rect.x,
            toolbar_rect.y + toolbar_rect.h - 1.0,
            toolbar_rect.w,
            1.0,
        ),
        DaylightTheme::BORDER,
    );

    if is_focused {
        paint_pdf_toolbar(
            surface,
            fonts,
            hit_regions,
            tab_idx,
            mouse_x,
            mouse_y,
            s,
            left_open,
            right_open,
            toolbar_rect,
            p,
        );
    } else {
        let lbl = format!(
            "PDF STUDIO · {} ({} page{})",
            p.pdf.title,
            p.pdf.pages.len(),
            if p.pdf.pages.len() == 1 { "" } else { "s" }
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            12.0 * s,
            toolbar_rect.x + 16.0 * s,
            toolbar_rect.y + toolbar_h * 0.63,
            DaylightTheme::PDF_ACCENT,
            &lbl,
        );
        hit_regions.push(HitRegion::button(toolbar_rect, UiAction::SwitchTab(tab_idx)));
    }

    let body = Rect::new(
        area.x,
        area.y + toolbar_h,
        area.w,
        (area.h - toolbar_h).max(40.0),
    );
    surface.fill_rect(body, DaylightTheme::DESK_BG);

    let show_thumbs = left_open && body.w >= 640.0;
    let thumb_w = if show_thumbs {
        (left_w * s).clamp(140.0, body.w * 0.34).round()
    } else {
        0.0
    };
    let show_inspector = right_open && body.w >= 1020.0;
    let insp_w = if show_inspector {
        (right_w * s).clamp(176.0, body.w * 0.34).round()
    } else {
        0.0
    };

    if show_thumbs {
        let lrect = Rect::new(body.x, body.y, thumb_w, body.h);
        paint_pdf_thumbnails(surface, fonts, hit_regions, tab_idx, mouse_x, mouse_y, s, lrect, p);
        let split_l = Rect::new(lrect.x + lrect.w - 4.0, lrect.y, 8.0, lrect.h);
        if split_l.contains(mouse_x, mouse_y) {
            surface.fill_rect(
                Rect::new(lrect.x + lrect.w - 1.5, lrect.y, 3.0, lrect.h),
                DaylightTheme::RUST_ORANGE,
            );
        }
        hit_regions.push(HitRegion::splitter(
            split_l,
            UiAction::BeginDrag(DragInteraction::PdfLeftSplitter { pane_left: body.x }),
        ));
    }

    if show_inspector {
        let rrect = Rect::new(body.x + body.w - insp_w, body.y, insp_w, body.h);
        paint_pdf_inspector(surface, fonts, hit_regions, mouse_x, mouse_y, s, rrect, p);
        let split_r = Rect::new(rrect.x - 4.0, rrect.y, 8.0, rrect.h);
        if split_r.contains(mouse_x, mouse_y) {
            surface.fill_rect(
                Rect::new(rrect.x - 1.5, rrect.y, 3.0, rrect.h),
                DaylightTheme::RUST_ORANGE,
            );
        }
        hit_regions.push(HitRegion::splitter(
            split_r,
            UiAction::BeginDrag(DragInteraction::PdfRightSplitter {
                pane_right: body.x + body.w,
            }),
        ));
    }

    let canvas = Rect::new(
        body.x + thumb_w,
        body.y,
        (body.w - thumb_w - insp_w).max(80.0),
        body.h,
    );
    surface.set_clip(canvas);

    let page_idx = p.current_page.min(p.pdf.pages.len().saturating_sub(1));
    if let Some(page) = p.pdf.pages.get(page_idx).cloned() {
        let fit_w = (canvas.w - 48.0 * s) / page.width_pt.max(1.0);
        let fit_h = (canvas.h - 44.0 * s) / page.height_pt.max(1.0);
        let base_scale = fit_w.min(fit_h).clamp(0.45, 1.35);
        let scale = (base_scale * p.zoom).clamp(0.45, 2.35);
        let sheet_w = page.width_pt * scale;
        let sheet_h = page.height_pt * scale;

        let total_h = sheet_h + 52.0 * s;
        p.max_scroll_y = (total_h - canvas.h).max(0.0);
        p.scroll_y = p.scroll_y.clamp(0.0, p.max_scroll_y);

        let sx = canvas.x + ((canvas.w - sheet_w) * 0.5).max(16.0);
        let sy = canvas.y + 24.0 * s - p.scroll_y;
        let sheet = Rect::new(sx, sy, sheet_w, sheet_h);

        surface.draw_paper_shadow(sheet, 2.0, 12.0, 18);
        surface.fill_rounded_rect(sheet, 2.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(sheet, 2.0, 1.0, DaylightTheme::BORDER);

        // Clip inner page stream strictly inside sheet bounds so wide lines never spill outside!
        let sheet_clip = Rect::new(
            sheet.x.max(canvas.x),
            sheet.y.max(canvas.y),
            sheet.w.min(canvas.x + canvas.w - sheet.x.max(canvas.x)),
            sheet.h.min(canvas.y + canvas.h - sheet.y.max(canvas.y)),
        );
        surface.set_clip(sheet_clip);

        if sheet.contains(mouse_x, mouse_y) {
            let pdf_x = ((mouse_x - sheet.x) / scale).clamp(16.0, page.width_pt - 32.0);
            let pdf_y = (page.height_pt - ((mouse_y - sheet.y) / scale)).clamp(16.0, page.height_pt - 24.0);
            hit_regions.push(HitRegion::text(
                sheet,
                UiAction::ClickPdfPageCanvas {
                    tab_idx,
                    page_idx,
                    pdf_x,
                    pdf_y,
                },
            ));
        }

        for rect_op in &page.rects {
            let rx = sheet.x + rect_op.x * scale;
            let ry = sheet.y + (page.height_pt - rect_op.y - rect_op.h) * scale;
            let rw = rect_op.w * scale;
            let rh = (rect_op.h * scale).max(1.0);
            let r = Rect::new(rx, ry, rw, rh);
            if let Some(fill) = rect_op.fill {
                surface.fill_rect(r, Color::rgb(fill.r, fill.g, fill.b));
            }
            if let Some((st, sw)) = rect_op.stroke {
                surface.stroke_rounded_rect(
                    r,
                    0.0,
                    (sw * scale).max(1.0),
                    Color::rgb(st.r, st.g, st.b),
                );
            }
        }

        for ann in &page.annotations {
            if let PdfAnnotation::Highlight { x, y, w, h, color } = ann {
                let hr = Rect::new(
                    sheet.x + x * scale,
                    sheet.y + (page.height_pt - y - h) * scale,
                    w * scale,
                    h * scale,
                );
                surface.fill_rounded_rect(hr, 2.0, Color::rgba(color.r, color.g, color.b, 145));
            }
        }

        for item in &page.text_items {
            let tx = sheet.x + item.x * scale;
            let baseline_y = sheet.y + (page.height_pt - item.y) * scale;
            let px_size = (item.font_size * scale).clamp(8.0, 52.0);
            let slot = if item.bold {
                FontSlot::SansBold
            } else {
                FontSlot::SansRegular
            };
            let tw = fonts
                .measure_text(slot, &item.text, px_size)
                .max(32.0 * scale);
            let item_box = Rect::new(tx - 3.0, baseline_y - px_size * 0.92, tw + 6.0, px_size * 1.24);
            let selected = is_focused && p.selected_text_id == Some(item.id);
            let hovered = item_box.contains(mouse_x, mouse_y);

            if selected {
                surface.fill_rounded_rect(item_box, 3.0, DaylightTheme::WORD_SOFT);
                surface.stroke_rounded_rect(item_box, 3.0, 1.4, DaylightTheme::WORD_ACCENT);
            } else if hovered && p.tool == PdfTool::SelectEdit {
                surface.stroke_rounded_rect(item_box, 3.0, 1.0, DaylightTheme::BORDER_STRONG);
            }

            surface.draw_text(
                fonts,
                slot,
                px_size,
                tx,
                baseline_y,
                Color::rgb(item.color.r, item.color.g, item.color.b),
                &item.text,
            );

            if selected && cursor_blink_on {
                let end_x = tx + fonts.measure_text(slot, &item.text, px_size);
                surface.fill_rect(
                    Rect::new(end_x + 1.0, item_box.y + 2.0, 2.0, item_box.h - 4.0),
                    DaylightTheme::WORD_ACCENT,
                );
            }

            if p.tool == PdfTool::SelectEdit {
                hit_regions.push(HitRegion::text(
                    item_box,
                    UiAction::ClickPdfTextItem {
                        tab_idx,
                        page_idx,
                        item_id: item.id,
                    },
                ));
            }
        }

        for ann in &page.annotations {
            match ann {
                PdfAnnotation::Highlight { .. } => {}
                PdfAnnotation::StickyNote { x, y, author, note } => {
                    let nr = Rect::new(
                        sheet.x + x * scale,
                        sheet.y + (page.height_pt - y - 40.0) * scale,
                        152.0 * scale,
                        44.0 * scale,
                    );
                    surface.fill_rounded_rect(nr, 4.0, Color::rgb(254, 249, 195));
                    surface.stroke_rounded_rect(nr, 4.0, 1.2, Color::rgb(202, 138, 4));
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SansBold,
                        (9.5 * scale).clamp(8.0, 13.0),
                        nr.x + 6.0,
                        nr.y + 13.0 * scale,
                        nr.w - 12.0,
                        Color::rgb(133, 77, 14),
                        author,
                    );
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SansRegular,
                        (9.0 * scale).clamp(7.5, 12.0),
                        nr.x + 6.0,
                        nr.y + 27.0 * scale,
                        nr.w - 12.0,
                        DaylightTheme::INK,
                        note,
                    );
                }
                PdfAnnotation::Stamp { x, y, label, color } => {
                    let sr = Rect::new(
                        sheet.x + x * scale,
                        sheet.y + (page.height_pt - y - 28.0) * scale,
                        108.0 * scale,
                        28.0 * scale,
                    );
                    let c = Color::rgb(color.r, color.g, color.b);
                    surface.fill_rounded_rect(sr, 4.0, Color::rgba(color.r, color.g, color.b, 26));
                    surface.stroke_rounded_rect(sr, 4.0, 2.0, c);
                    let fsz = (11.5 * scale).clamp(9.0, 16.0);
                    let lw = fonts.measure_text(FontSlot::SansBold, label, fsz);
                    surface.draw_text(
                        fonts,
                        FontSlot::SansBold,
                        fsz,
                        sr.x + (sr.w - lw) * 0.5,
                        sr.y + sr.h * 0.68,
                        c,
                        label,
                    );
                }
                PdfAnnotation::Redaction { x, y, w, h } => {
                    let rr = Rect::new(
                        sheet.x + x * scale,
                        sheet.y + (page.height_pt - y - h) * scale,
                        w * scale,
                        h * scale,
                    );
                    surface.fill_rect(rr, Color::rgb(20, 20, 18));
                    surface.draw_text(
                        fonts,
                        FontSlot::MonoRegular,
                        (8.5 * scale).clamp(7.0, 11.0),
                        rr.x + 6.0,
                        rr.y + rr.h * 0.68,
                        DaylightTheme::PAPER,
                        "REDACTED",
                    );
                }
                PdfAnnotation::Signature {
                    strokes,
                    signer,
                    color,
                } => {
                    let sc = Color::rgb(color.r, color.g, color.b);
                    let mut min_x = f32::MAX;
                    let mut max_y = f32::MIN;
                    for stroke in strokes {
                        for pair in stroke.windows(2) {
                            let x0 = sheet.x + pair[0].0 * scale;
                            let y0 = sheet.y + (page.height_pt - pair[0].1) * scale;
                            let x1 = sheet.x + pair[1].0 * scale;
                            let y1 = sheet.y + (page.height_pt - pair[1].1) * scale;
                            min_x = min_x.min(x0).min(x1);
                            max_y = max_y.max(y0).max(y1);
                            surface.draw_line(x0, y0, x1, y1, (2.1 * scale).max(1.5), sc);
                        }
                    }
                    if min_x < f32::MAX {
                        surface.draw_text(
                            fonts,
                            FontSlot::SansRegular,
                            (9.0 * scale).clamp(7.5, 12.5),
                            min_x,
                            max_y + 13.0 * scale,
                            DaylightTheme::INK_SUB,
                            &format!("Signed by {signer}"),
                        );
                    }
                }
            }
        }

        if let Some(wm) = &page.watermark {
            let wm_px = (30.0 * scale).clamp(16.0, 50.0);
            let ww = fonts.measure_text(FontSlot::SansBold, wm, wm_px);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                wm_px,
                sheet.x + (sheet.w - ww) * 0.5,
                sheet.y + sheet.h * 0.52,
                Color::rgba(166, 54, 38, 38),
                wm,
            );
        }

        surface.set_clip(canvas);
        if p.max_scroll_y > 4.0 {
            paint_canvas_scrollbar(
                surface,
                hit_regions,
                tab_idx,
                mouse_x,
                mouse_y,
                canvas,
                p.scroll_y,
                p.max_scroll_y,
                total_h,
            );
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
    s: f32,
    left_open: bool,
    right_open: bool,
    bar: Rect,
    p: &PdfTabState,
) {
    let bh = (30.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut bx = bar.x + 10.0 * s;
    let max_x = bar.x + bar.w - 44.0 * s;

    // Left Pages Sidebar toggle
    let sb_r = Rect::new(bx, by, (30.0 * s).round(), bh);
    if left_open || sb_r.contains(mouse_x, mouse_y) {
        surface.fill_rounded_rect(
            sb_r,
            5.0,
            if left_open {
                DaylightTheme::PDF_SOFT
            } else {
                DaylightTheme::HOVER
            },
        );
    }
    surface.draw_icon(
        UiIcon::Sidebar,
        sb_r.x + sb_r.w * 0.5,
        sb_r.y + bh * 0.5,
        13.5 * s,
        if left_open {
            DaylightTheme::PDF_ACCENT
        } else {
            DaylightTheme::INK_SUB
        },
    );
    hit_regions.push(HitRegion::button(
        sb_r,
        UiAction::ExecCommand(CommandId::ToggleLeftSidebar),
    ));
    bx += sb_r.w + 8.0 * s;

    let tools: &[(PdfTool, &str)] = &[
        (PdfTool::SelectEdit, "Select / Edit"),
        (PdfTool::AddText, "+ Text"),
        (PdfTool::Highlight, "Highlight"),
        (PdfTool::StickyNote, "Note"),
        (PdfTool::Redact, "Redact"),
    ];

    for (tool, label) in tools {
        let tw = fonts.measure_text(FontSlot::SansBold, label, 11.6 * s);
        let bw = (tw + 16.0 * s).round();
        if bx + bw > max_x {
            break;
        }
        let r = Rect::new(bx, by, bw, bh);
        let active = p.tool == *tool;
        let hov = r.contains(mouse_x, mouse_y);
        if active || hov {
            surface.fill_rounded_rect(
                r,
                5.0,
                if active {
                    DaylightTheme::PDF_SOFT
                } else {
                    DaylightTheme::HOVER
                },
            );
        }
        if active {
            surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::PDF_ACCENT);
        }
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.6 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.66,
            if active {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::INK
            },
            label,
        );
        hit_regions.push(HitRegion::button(
            r,
            UiAction::SetPdfTool {
                tab_idx,
                tool: *tool,
            },
        ));
        bx += bw + 4.0 * s;
    }

    surface.fill_rect(
        Rect::new(bx + 3.0 * s, bar.y + 10.0 * s, 1.0, bar.h - 20.0 * s),
        DaylightTheme::BORDER,
    );
    bx += 9.0 * s;

    let ops: &[(&str, CommandId, bool)] = &[
        ("Sign Ink", CommandId::OpenSignaturePad, false),
        ("Stamp", CommandId::StampApproved, false),
        ("Merge", CommandId::PdfMergeFile, false),
        ("Split", CommandId::PdfSplitModal, false),
        ("Rotate", CommandId::PdfRotatePage, false),
        ("+ Page", CommandId::PdfAddBlankPage, false),
        ("Compress", CommandId::PdfCompress, false),
        ("To Word", CommandId::ConvertPdfToWord, true),
    ];

    for (label, cmd, word_accent) in ops {
        let tw = fonts.measure_text(FontSlot::SansBold, label, 11.6 * s);
        let bw = (tw + 16.0 * s).round();
        if bx + bw > max_x {
            break;
        }
        let r = Rect::new(bx, by, bw, bh);
        let hov = r.contains(mouse_x, mouse_y);
        if *word_accent || hov {
            surface.fill_rounded_rect(
                r,
                5.0,
                if *word_accent {
                    DaylightTheme::WORD_SOFT
                } else {
                    DaylightTheme::HOVER
                },
            );
        }
        if *word_accent {
            surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::WORD_ACCENT);
        }
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.6 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.66,
            if *word_accent {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::INK
            },
            label,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(*cmd)));
        bx += bw + 4.0 * s;
    }

    // Right Inspector toggle icon at right edge of PDF toolbar
    let insp_btn = Rect::new(bar.x + bar.w - 38.0 * s, by, (30.0 * s).round(), bh);
    if right_open || insp_btn.contains(mouse_x, mouse_y) {
        surface.fill_rounded_rect(
            insp_btn,
            5.0,
            if right_open {
                DaylightTheme::WORD_SOFT
            } else {
                DaylightTheme::HOVER
            },
        );
    }
    surface.draw_icon(
        UiIcon::SidebarRight,
        insp_btn.x + insp_btn.w * 0.5,
        insp_btn.y + bh * 0.5,
        13.5 * s,
        if right_open {
            DaylightTheme::WORD_ACCENT
        } else {
            DaylightTheme::INK_SUB
        },
    );
    hit_regions.push(HitRegion::button(insp_btn, UiAction::TogglePdfRightInspector));
}

#[allow(clippy::too_many_arguments)]
fn paint_pdf_thumbnails(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    sb: Rect,
    p: &PdfTabState,
) {
    surface.fill_rect(sb, DaylightTheme::SIDEBAR);
    surface.fill_rect(
        Rect::new(sb.x + sb.w - 1.0, sb.y, 1.0, sb.h),
        DaylightTheme::BORDER,
    );
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.5 * s,
        sb.x + 12.0 * s,
        sb.y + 24.0 * s,
        DaylightTheme::INK_MUTED,
        &format!("PAGES ({})", p.pdf.pages.len()),
    );

    let mut y = sb.y + 36.0 * s;
    let total = p.pdf.pages.len();
    for (idx, pg) in p.pdf.pages.iter().enumerate() {
        let card_h = (92.0 * s).round();
        if y + card_h > sb.y + sb.h - 10.0 {
            break;
        }
        let card = Rect::new(sb.x + 10.0 * s, y, sb.w - 20.0 * s, card_h - 6.0 * s);
        let active = idx == p.current_page;
        let hov = card.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            card,
            5.0,
            if active {
                DaylightTheme::PDF_SOFT
            } else if hov {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(
            card,
            5.0,
            if active { 1.5 } else { 1.0 },
            if active {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );

        let mini = Rect::new(card.x + 8.0 * s, card.y + 7.0 * s, 46.0 * s, card.h - 14.0 * s);
        surface.fill_rect(mini, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(mini, 2.0, 1.0, DaylightTheme::BORDER_STRONG);
        for ln in 0..4 {
            surface.fill_rect(
                Rect::new(
                    mini.x + 6.0 * s,
                    mini.y + (9.0 + (ln as f32) * 11.0) * s,
                    mini.w - 12.0 * s,
                    2.0,
                ),
                DaylightTheme::BORDER,
            );
        }

        let tx = mini.x + mini.w + 9.0 * s;
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            12.0 * s,
            tx,
            card.y + 24.0 * s,
            DaylightTheme::INK,
            &format!("Page {}", idx + 1),
        );
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansRegular,
            10.5 * s,
            tx,
            card.y + 42.0 * s,
            (card.w - (tx - card.x) - 6.0).max(24.0),
            DaylightTheme::INK_MUTED,
            &format!("{} blocks", pg.text_items.len()),
        );

        hit_regions.push(HitRegion::button(
            card,
            UiAction::SelectPdfPage {
                tab_idx,
                page_idx: idx,
            },
        ));

        // Up & Down Page Reorder Controls
        let mut btn_x = tx;
        if idx > 0 {
            let up_r = Rect::new(btn_x, card.y + 54.0 * s, 32.0 * s, 20.0 * s);
            surface.fill_rounded_rect(up_r, 3.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(up_r, 3.0, 1.0, DaylightTheme::BORDER);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                9.8 * s,
                up_r.x + 7.0 * s,
                up_r.y + 13.5 * s,
                DaylightTheme::INK_SUB,
                "Up",
            );
            hit_regions.push(HitRegion::button(
                up_r,
                UiAction::MovePdfPage {
                    tab_idx,
                    from: idx,
                    to: idx - 1,
                },
            ));
            btn_x += 36.0 * s;
        }
        if idx + 1 < total {
            let dn_r = Rect::new(btn_x, card.y + 54.0 * s, 36.0 * s, 20.0 * s);
            surface.fill_rounded_rect(dn_r, 3.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(dn_r, 3.0, 1.0, DaylightTheme::BORDER);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                9.8 * s,
                dn_r.x + 5.0 * s,
                dn_r.y + 13.5 * s,
                DaylightTheme::INK_SUB,
                "Down",
            );
            hit_regions.push(HitRegion::button(
                dn_r,
                UiAction::MovePdfPage {
                    tab_idx,
                    from: idx,
                    to: idx + 1,
                },
            ));
        }

        y += card_h;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_pdf_inspector(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    insp: Rect,
    p: &PdfTabState,
) {
    surface.fill_rect(insp, DaylightTheme::SIDEBAR);
    surface.fill_rect(
        Rect::new(insp.x, insp.y, 1.0, insp.h),
        DaylightTheme::BORDER,
    );

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.5 * s,
        insp.x + 14.0 * s,
        insp.y + 24.0 * s,
        DaylightTheme::INK_MUTED,
        "PDF INSPECTOR",
    );

    let pg = p.pdf.pages.get(p.current_page);
    let rows = [
        ("Document", p.pdf.title.clone()),
        ("Pages", format!("{}", p.pdf.pages.len())),
        ("Active Tool", p.tool.label().to_string()),
        (
            "Geometry",
            pg.map(|g| format!("{:.0} × {:.0} pt", g.width_pt, g.height_pt))
                .unwrap_or_default(),
        ),
        (
            "Annotations",
            format!("{}", pg.map(|g| g.annotations.len()).unwrap_or(0)),
        ),
    ];

    let mut y = insp.y + 42.0 * s;
    for (k, val) in rows {
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.2 * s,
            insp.x + 14.0 * s,
            y,
            DaylightTheme::INK_MUTED,
            k,
        );
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansRegular,
            12.0 * s,
            insp.x + 14.0 * s,
            y + 16.0 * s,
            insp.w - 28.0 * s,
            DaylightTheme::INK,
            &val,
        );
        y += 38.0 * s;
    }

    y += 6.0 * s;
    let quick: &[(&str, CommandId)] = &[
        ("Draw & Place Signature", CommandId::OpenSignaturePad),
        ("Stamp [APPROVED]", CommandId::StampApproved),
        ("Set Diagonal Watermark", CommandId::OpenWatermarkModal),
        ("Delete Active Page", CommandId::PdfDeletePage),
        ("Convert PDF -> Word (.docx)", CommandId::ConvertPdfToWord),
    ];

    for (lbl, cmd) in quick {
        let bh = (32.0 * s).round();
        if y + bh > insp.y + insp.h - 12.0 {
            break;
        }
        let r = Rect::new(insp.x + 12.0 * s, y, insp.w - 24.0 * s, bh);
        let hov = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            r,
            5.0,
            if hov {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansBold,
            11.4 * s,
            r.x + 10.0 * s,
            r.y + bh * 0.65,
            r.w - 18.0 * s,
            DaylightTheme::INK,
            lbl,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(*cmd)));
        y += bh + 6.0 * s;
    }
}
