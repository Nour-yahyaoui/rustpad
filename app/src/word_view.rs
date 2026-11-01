//! Word Studio (`.docx`) Paginated Canvas, Vector Formatting Ribbon, Contextual Table Bar,
//! Multi-Line Resizable Tables, Character-Accurate Hit-Testing & Resizable Outline Rail.

use crate::commands::CommandId;
use crate::state::{
    DragInteraction, HitRegion, ToolbarDropdown, UiAction, WordLineHitBox, WordTabState,
};
use doc_model::{
    probe_media_dimensions, Alignment, Block, FontFamily, ListKind, Paragraph, ParagraphStyle,
    RevisionKind, SharedBytes,
};
use power_tools::SpellChecker;
use rustpad_render::{
    decode_any_image_thumbnail_rgba, hash_image_bytes, Color, FontBook, FontSlot, Rect,
    StudioTheme, Surface, UiIcon,
};

#[allow(clippy::too_many_arguments)]
pub fn paint_word_tab(
    surface: &mut Surface,
    fonts: &mut FontBook,
    spellchecker: &SpellChecker,
    hit_regions: &mut Vec<HitRegion>,
    word_line_boxes: &mut Vec<WordLineHitBox>,
    tab_idx: usize,
    is_focused: bool,
    focus_mode: bool,
    active_drag: DragInteraction,
    cursor_blink_on: bool,
    mouse_x: f32,
    mouse_y: f32,
    ui_scale: f32,
    sidebar_open: bool,
    sidebar_w: f32,
    area: Rect,
    w: &mut WordTabState,
) {
    let content_key = compute_content_key(w);
    if w.cached_metrics_key != content_key {
        w.cached_stats = w.doc.stats();
        w.cached_outline = w.doc.outline();
        w.cached_rev_counts = w.doc.revision_counts();
        w.cached_metrics_key = content_key;
    }

    let s = ui_scale.clamp(0.85, 1.6);
    let ribbon_h = if focus_mode { 0.0 } else { (44.0 * s).round() };
    let has_active_table = w.active_table_cell.is_some();
    let has_active_image = w
        .selected_image_block
        .is_some_and(|b| matches!(w.doc.blocks.get(b), Some(Block::Image(_))));
    let (rev_ins, rev_del) = w.cached_rev_counts;
    let show_review_bar = !focus_mode
        && is_focused
        && !has_active_table
        && !has_active_image
        && (w.doc.track_changes || rev_ins > 0 || rev_del > 0 || !w.doc.comments.is_empty());
    let table_bar_h = if !focus_mode
        && (((has_active_table || has_active_image) && is_focused) || show_review_bar)
    {
        (36.0 * s).round()
    } else {
        0.0
    };

    // 1. Primary Formatting Ribbon (hidden in Distraction-Free Focus Mode)
    if ribbon_h > 0.0 {
        let ribbon_rect = Rect::new(area.x, area.y, area.w, ribbon_h);
        surface.fill_rect(ribbon_rect, StudioTheme::chrome());
        surface.fill_rect(
            Rect::new(
                ribbon_rect.x,
                ribbon_rect.y + ribbon_rect.h - 1.0,
                ribbon_rect.w,
                1.0,
            ),
            StudioTheme::border(),
        );
        if is_focused {
            paint_word_ribbon(
                surface,
                fonts,
                hit_regions,
                mouse_x,
                mouse_y,
                s,
                sidebar_open,
                ribbon_rect,
                w,
            );
        } else {
            let lbl = format!("WORD STUDIO · {}", w.doc.title);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                12.0 * s,
                ribbon_rect.x + 16.0 * s,
                ribbon_rect.y + ribbon_h * 0.63,
                StudioTheme::word_accent(),
                &lbl,
            );
            hit_regions.push(HitRegion::button(ribbon_rect, UiAction::SwitchTab(tab_idx)));
        }
    }

    // 2. Contextual Table Studio, Image Studio, OR Legal Redline Review Sub-Bar
    if table_bar_h > 0.0 {
        let tbar = Rect::new(area.x, area.y + ribbon_h, area.w, table_bar_h);
        if has_active_table {
            paint_contextual_table_bar(surface, fonts, hit_regions, mouse_x, mouse_y, s, tbar, w);
        } else if has_active_image {
            paint_contextual_image_bar(surface, fonts, hit_regions, mouse_x, mouse_y, s, tbar, w);
        } else {
            paint_contextual_review_bar(
                surface,
                fonts,
                hit_regions,
                tab_idx,
                mouse_x,
                mouse_y,
                s,
                tbar,
                w,
                rev_ins,
                rev_del,
            );
        }
    }

    let top_used = ribbon_h + table_bar_h;
    let body_rect = Rect::new(
        area.x,
        area.y + top_used,
        area.w,
        (area.h - top_used).max(40.0),
    );
    surface.fill_rect(body_rect, StudioTheme::desk_bg());

    // 3. Resizable + Collapsible Left Document Outline Rail (hidden in Focus Mode)
    let can_fit_sidebar = !focus_mode && sidebar_open && body_rect.w >= 640.0;
    let eff_sb_w = if can_fit_sidebar {
        (sidebar_w * s).clamp(160.0, body_rect.w * 0.42).round()
    } else {
        0.0
    };

    let canvas_rect = Rect::new(
        body_rect.x + eff_sb_w,
        body_rect.y,
        (body_rect.w - eff_sb_w).max(80.0),
        body_rect.h,
    );

    // 4. Layout & Paginate Document Blocks Across Physical Sheets
    let (pt_w, pt_h) = w
        .doc
        .page_setup
        .paper_size
        .dimensions_pt(w.doc.page_setup.landscape);
    let scale = (1.16 * w.zoom * (s * 0.96).clamp(0.92, 1.35)).clamp(0.65, 2.45);
    let page_w = pt_w * scale;
    let page_h = pt_h * scale;
    let margin_x = (w.doc.page_setup.margin_left_pt * scale).min(page_w * 0.22);
    let margin_top = (w.doc.page_setup.margin_top_pt * scale).min(page_h * 0.18);
    let margin_bot = (w.doc.page_setup.margin_bottom_pt * scale).min(page_h * 0.18);
    let content_w = (page_w - margin_x * 2.0).max(120.0);
    let usable_page_h = (page_h - margin_top - margin_bot).max(160.0);

    // Zero-allocation header probe on first frame: populate `intrinsic_px` for all figures in
    // microseconds without decoding gigabytes of off-screen screenshots upfront!
    if !w.media_dimensions_probed {
        for blk in &mut w.doc.blocks {
            if let Block::Image(img) = blk {
                if img.intrinsic_px.is_none() && !img.png_bytes.is_empty() {
                    img.intrinsic_px = probe_media_dimensions(&img.png_bytes);
                }
            }
        }
        w.media_dimensions_probed = true;
    }

    let pages = ensure_cached_layout(fonts, w, content_key, scale, content_w, usable_page_h);
    let page_gap = 28.0 * s;
    let top_pad = 26.0 * s;
    let total_pages = pages.len().max(1);
    let total_doc_h =
        top_pad * 2.0 + (total_pages as f32) * page_h + (total_pages.saturating_sub(1) as f32) * page_gap;
    w.max_scroll_y = (total_doc_h - canvas_rect.h + 40.0).max(0.0);

    // Record exact canvas Y of every block for accurate Outline jump & auto-scroll-to-caret
    let mut block_canvas_y = vec![0.0f32; w.doc.blocks.len().max(1)];
    let mut caret_doc_y: Option<f32> = None;
    for (p_idx, page_items) in pages.iter().enumerate() {
        let p_top = top_pad + (p_idx as f32) * (page_h + page_gap);
        for item in page_items {
            let item_doc_y = p_top + margin_top + item.rel_y;
            if item.table_row_start == 0 && item.block_idx < block_canvas_y.len() {
                block_canvas_y[item.block_idx] = (item_doc_y - 36.0).max(0.0);
            }
            if let Some((tb, tr, _)) = w.active_table_cell {
                if tb == item.block_idx
                    && tr >= item.table_row_start
                    && tr < item.table_row_start + item.table_row_heights.len().max(1)
                {
                    let local_r = tr - item.table_row_start;
                    let row_off: f32 = item.table_row_heights.iter().take(local_r).sum();
                    caret_doc_y = Some(item_doc_y + row_off + 18.0);
                }
            } else if w.cursor_block == item.block_idx && item.table_row_start == 0 {
                caret_doc_y = Some(item_doc_y + item.height * 0.5);
            }
        }
    }

    if w.scroll_to_caret {
        w.scroll_to_caret = false;
        if let Some(cy) = caret_doc_y {
            let view_top = w.scroll_y + 56.0;
            let view_bot = w.scroll_y + canvas_rect.h - 72.0;
            if cy < view_top {
                let next = (cy - 56.0).max(0.0);
                w.target_scroll_y = next;
                w.scroll_y = next;
            } else if cy > view_bot {
                let next = (cy - canvas_rect.h + 88.0).clamp(0.0, w.max_scroll_y);
                w.target_scroll_y = next;
                w.scroll_y = next;
            }
        }
    }
    w.scroll_y = w.scroll_y.clamp(0.0, w.max_scroll_y);
    w.target_scroll_y = w.target_scroll_y.clamp(0.0, w.max_scroll_y);

    // Paint Outline Sidebar now that exact `block_canvas_y` offsets are known!
    if can_fit_sidebar {
        let sb_rect = Rect::new(body_rect.x, body_rect.y, eff_sb_w, body_rect.h);
        paint_outline_sidebar(
            surface,
            fonts,
            hit_regions,
            tab_idx,
            mouse_x,
            mouse_y,
            s,
            sb_rect,
            &block_canvas_y,
            w,
        );
        // Interactive Resize Splitter on right edge of Outline Sidebar
        let split_hit = Rect::new(sb_rect.x + sb_rect.w - 4.0, sb_rect.y, 8.0, sb_rect.h);
        let split_hov = split_hit.contains(mouse_x, mouse_y);
        if split_hov {
            surface.fill_rect(
                Rect::new(sb_rect.x + sb_rect.w - 1.5, sb_rect.y, 3.0, sb_rect.h),
                StudioTheme::rust_orange(),
            );
        }
        hit_regions.push(HitRegion::splitter(
            split_hit,
            UiAction::BeginDrag(DragInteraction::WordOutlineSplitter {
                pane_left: body_rect.x,
            }),
        ));
    }

    // 5. Render Physical Paper Sheets & Blocks
    surface.set_clip(canvas_rect);
    let scrollbar_reserve = if w.max_scroll_y > 4.0 { 14.0 } else { 0.0 };
    let sheet_x = canvas_rect.x
        + ((canvas_rect.w - scrollbar_reserve - page_w) * 0.5).max(18.0 * s);
    let mut sel_bar_anchor: Option<Rect> = None;
    let mut decoded_images_this_frame = 0usize;

    for (page_idx, page_items) in pages.iter().enumerate() {
        let sheet_y =
            canvas_rect.y + top_pad - w.scroll_y + (page_idx as f32) * (page_h + page_gap);
        let sheet = Rect::new(sheet_x, sheet_y, page_w, page_h);

        if sheet.y + sheet.h < canvas_rect.y - 24.0
            || sheet.y > canvas_rect.y + canvas_rect.h + 24.0
        {
            continue;
        }

        // Clicking blank paper space jumps cursor cleanly to the last block on that page
        let fallback_block = page_items
            .last()
            .map(|it| it.block_idx)
            .unwrap_or_else(|| w.doc.blocks.len().saturating_sub(1));
        let fallback_char = match w.doc.blocks.get(fallback_block) {
            Some(Block::Paragraph(p)) => p.plain_text().chars().count(),
            _ => 0,
        };
        hit_regions.push(HitRegion::text(
            sheet,
            UiAction::ClickWordParagraph {
                tab_idx,
                block_idx: fallback_block,
                char_idx: fallback_char,
            },
        ));

        surface.draw_paper_shadow(sheet, 2.0, 12.0, 18);
        surface.fill_rounded_rect(sheet, 2.0, StudioTheme::sheet_paper());
        surface.stroke_rounded_rect(sheet, 2.0, 1.0, StudioTheme::sheet_border());

        // Running Header (respects `different_first_page` when `first_page_header` is set or blank)
        let active_hdr = if page_idx == 0 && w.doc.page_setup.different_first_page {
            w.doc.page_setup.first_page_header.as_str()
        } else {
            w.doc.page_setup.header_text.as_str()
        };
        if !active_hdr.is_empty() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                (9.5 * scale).clamp(8.5, 14.0),
                sheet.x + margin_x,
                sheet.y + (margin_top * 0.56).max(20.0),
                content_w,
                StudioTheme::sheet_ink_muted(),
                active_hdr,
            );
            surface.fill_rect(
                Rect::new(
                    sheet.x + margin_x,
                    sheet.y + (margin_top * 0.68).max(26.0),
                    content_w,
                    1.0,
                ),
                StudioTheme::sheet_border(),
            );
        }

        // Running Footer + Page Counter + Footnotes preview on final page
        let footer_y = sheet.y + sheet.h - (margin_bot * 0.42).max(16.0);
        let active_ftr = if page_idx == 0 && w.doc.page_setup.different_first_page {
            w.doc.page_setup.first_page_footer.as_str()
        } else {
            w.doc.page_setup.footer_text.as_str()
        };
        if !active_ftr.is_empty() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                (9.2 * scale).clamp(8.0, 13.5),
                sheet.x + margin_x,
                footer_y,
                content_w * 0.72,
                StudioTheme::sheet_ink_muted(),
                active_ftr,
            );
        }
        if w.doc.page_setup.show_page_numbers {
            let p_str = format!("Page {} of {}", page_idx + 1, total_pages);
            let pw = fonts.measure_text(
                FontSlot::SansRegular,
                &p_str,
                (9.2 * scale).clamp(8.0, 13.5),
            );
            surface.draw_text(
                fonts,
                FontSlot::SansRegular,
                (9.2 * scale).clamp(8.0, 13.5),
                sheet.x + sheet.w - margin_x - pw,
                footer_y,
                StudioTheme::sheet_ink_muted(),
                &p_str,
            );
        }
        if page_idx + 1 == total_pages && (!w.doc.footnotes.is_empty() || !w.doc.endnotes.is_empty()) {
            let note_count = w.doc.footnotes.len() + w.doc.endnotes.len();
            let max_show = note_count.min(3);
            let base_ny = footer_y - (16.0 + max_show as f32 * 12.5) * scale;
            surface.fill_rect(
                Rect::new(sheet.x + margin_x, base_ny, content_w * 0.34, 1.0),
                StudioTheme::sheet_border(),
            );
            let mut ny = base_ny + 11.5 * scale;
            for n in w.doc.footnotes.iter().take(max_show) {
                surface.draw_text_clipped(
                    fonts,
                    FontSlot::SerifRegular,
                    (8.8 * scale).clamp(8.0, 12.5),
                    sheet.x + margin_x,
                    ny,
                    content_w,
                    StudioTheme::sheet_ink_sub(),
                    &format!("[{}] {}", n.id, n.text),
                );
                ny += 12.0 * scale;
            }
            if w.doc.footnotes.len() < max_show {
                for n in w.doc.endnotes.iter().take(max_show - w.doc.footnotes.len()) {
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SerifRegular,
                        (8.8 * scale).clamp(8.0, 12.5),
                        sheet.x + margin_x,
                        ny,
                        content_w,
                        StudioTheme::sheet_ink_sub(),
                        &format!("[e{}] {}", n.id, n.text),
                    );
                    ny += 12.0 * scale;
                }
            }
        }

        // Optional Watermark
        if let Some(wm) = &w.doc.page_setup.watermark {
            let wm_px = (32.0 * scale).clamp(18.0, 56.0);
            let ww = fonts.measure_text(FontSlot::SansBold, wm, wm_px);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                wm_px,
                sheet.x + (sheet.w - ww) * 0.5,
                sheet.y + sheet.h * 0.52,
                Color::rgba(122, 118, 110, 38),
                wm,
            );
        }

        let content_origin_x = sheet.x + margin_x;
        let content_origin_y = sheet.y + margin_top;

        // Progressive viewport-only HiDPI thumbnail decoder (64 MiB LRU budget, max 3 decodes/frame)
        warm_visible_page_images(
            w,
            page_items,
            content_origin_y,
            canvas_rect,
            &mut decoded_images_this_frame,
        );

        for item in page_items {
            let blk_top = content_origin_y + item.rel_y;
            let blk_bot = blk_top + item.height;
            if blk_bot < canvas_rect.y - 36.0 || blk_top > canvas_rect.y + canvas_rect.h + 36.0 {
                continue;
            }
            paint_laid_out_block(
                surface,
                fonts,
                spellchecker,
                hit_regions,
                word_line_boxes,
                &mut sel_bar_anchor,
                tab_idx,
                is_focused,
                cursor_blink_on,
                mouse_x,
                mouse_y,
                w,
                item,
                content_origin_x,
                content_origin_y + item.rel_y,
                content_w,
                scale,
            );
        }
    }

    // 5b. Floating Contextual Selection Mini-Toolbar (Warm Ivory Paper Popover)
    if is_focused
        && !matches!(active_drag, DragInteraction::WordTextSelection { .. })
        && w.has_selection()
    {
        if let Some(sel_r) = sel_bar_anchor {
            paint_floating_selection_bar(
                surface,
                fonts,
                hit_regions,
                mouse_x,
                mouse_y,
                s,
                canvas_rect,
                sel_r,
                w,
            );
        }
    }

    // 5c. Distraction-Free Focus Mode subtle exit badge
    if focus_mode {
        let pill_w = (188.0 * s).round();
        let pill_h = (26.0 * s).round();
        let pill = Rect::new(
            (canvas_rect.x + canvas_rect.w - pill_w - 18.0 * s).max(canvas_rect.x + 8.0),
            canvas_rect.y + 12.0 * s,
            pill_w,
            pill_h,
        );
        let hov = pill.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            pill,
            13.0,
            if hov {
                StudioTheme::paper()
            } else {
                Color::rgba(255, 255, 255, 228)
            },
        );
        surface.stroke_rounded_rect(pill, 13.0, 1.0, StudioTheme::border_strong());
        let lbl = "Exit Focus · Ctrl+Shift+F";
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 10.6 * s);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.6 * s,
            pill.x + (pill.w - tw) * 0.5,
            pill.y + pill_h * 0.67,
            StudioTheme::word_accent(),
            lbl,
        );
        hit_regions.push(HitRegion::button(
            pill,
            UiAction::ExecCommand(CommandId::ToggleFocusMode),
        ));
    }

    // 6. Interactive Vertical Scrollbar Thumb
    if w.max_scroll_y > 4.0 {
        paint_canvas_scrollbar(
            surface,
            hit_regions,
            tab_idx,
            mouse_x,
            mouse_y,
            canvas_rect,
            w.scroll_y,
            w.max_scroll_y,
            total_doc_h,
        );
    }

    surface.reset_clip();
}

#[allow(clippy::too_many_arguments)]
fn paint_floating_selection_bar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    canvas_rect: Rect,
    sel_r: Rect,
    w: &WordTabState,
) {
    let bar_h = (34.0 * s).round();
    let pad_x = (6.0 * s).round();
    let gap = (3.0 * s).round();

    let probe_style = match w.ordered_selection() {
        Some(((b0, c0), _)) => match w.doc.blocks.get(b0) {
            Some(Block::Paragraph(p)) => p.style_at_char(c0 + 1),
            _ => doc_model::SpanStyle::default(),
        },
        None => doc_model::SpanStyle::default(),
    };

    let items: [(&str, FontSlot, bool, bool, CommandId); 7] = [
        ("B", FontSlot::SansBold, probe_style.bold, false, CommandId::ToggleBold),
        ("I", FontSlot::SerifRegular, probe_style.italic, false, CommandId::ToggleItalic),
        ("U", FontSlot::SansBold, probe_style.underline, false, CommandId::ToggleUnderline),
        (
            "Highlight",
            FontSlot::SansRegular,
            probe_style.highlight.is_some(),
            false,
            CommandId::ToggleHighlight,
        ),
        ("Ink", FontSlot::SansRegular, false, false, CommandId::CycleTextColor),
        ("+Note", FontSlot::SansRegular, false, false, CommandId::AddReviewComment),
        ("AI Polish", FontSlot::SansBold, false, true, CommandId::AiRunFixGrammar),
    ];

    let mut total_w = pad_x * 2.0;
    let mut btn_ws = [0.0f32; 7];
    for (i, (lbl, slot, _, _, _)) in items.iter().enumerate() {
        let tw = fonts.measure_text(*slot, lbl, 11.2 * s);
        let bw = (tw + 14.0 * s).max(26.0 * s).round();
        btn_ws[i] = bw;
        total_w += bw;
        if i + 1 < items.len() {
            total_w += gap;
        }
    }

    let min_x = canvas_rect.x + 10.0 * s;
    let max_x = (canvas_rect.x + canvas_rect.w - total_w - 16.0 * s).max(min_x);
    let pop_x = (sel_r.x + (sel_r.w - total_w) * 0.5).clamp(min_x, max_x).round();
    let above_y = sel_r.y - bar_h - 8.0 * s;
    let pop_y = if above_y >= canvas_rect.y + 8.0 * s {
        above_y.round()
    } else {
        (sel_r.y + sel_r.h + 8.0 * s)
            .min(canvas_rect.y + canvas_rect.h - bar_h - 8.0 * s)
            .round()
    };

    let pop = Rect::new(pop_x, pop_y, total_w, bar_h);
    surface.draw_paper_shadow(pop, 7.0, 12.0, 26);
    surface.fill_rounded_rect(pop, 7.0, StudioTheme::paper());
    surface.stroke_rounded_rect(pop, 7.0, 1.0, StudioTheme::border_strong());

    let bh = (24.0 * s).round();
    let by = (pop.y + (bar_h - bh) * 0.5).round();
    let mut bx = pop.x + pad_x;

    for (i, (lbl, slot, active, ai_accent, cmd)) in items.iter().enumerate() {
        let bw = btn_ws[i];
        let br = Rect::new(bx, by, bw, bh);
        let hov = br.contains(mouse_x, mouse_y);
        if *ai_accent {
            surface.fill_rounded_rect(
                br,
                4.5,
                if hov {
                    StudioTheme::word_accent()
                } else {
                    StudioTheme::word_soft()
                },
            );
            surface.stroke_rounded_rect(br, 4.5, 1.0, StudioTheme::word_accent());
        } else if *active || hov {
            surface.fill_rounded_rect(
                br,
                4.5,
                if *active {
                    StudioTheme::word_soft()
                } else {
                    StudioTheme::hover()
                },
            );
            if *active {
                surface.stroke_rounded_rect(br, 4.5, 1.0, StudioTheme::word_accent());
            }
        }
        let fg = if *ai_accent && hov {
            StudioTheme::paper()
        } else if *ai_accent || *active {
            StudioTheme::word_accent()
        } else {
            StudioTheme::ink()
        };
        let tw = fonts.measure_text(*slot, lbl, 11.2 * s);
        surface.draw_text(
            fonts,
            *slot,
            11.2 * s,
            br.x + (bw - tw) * 0.5,
            br.y + bh * 0.70,
            fg,
            lbl,
        );
        hit_regions.push(HitRegion::button(br, UiAction::ExecCommand(*cmd)));
        bx += bw + gap;
    }
}

pub fn paint_canvas_scrollbar(
    surface: &mut Surface,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    canvas_rect: Rect,
    scroll_y: f32,
    max_scroll_y: f32,
    total_content_h: f32,
) {
    let track = Rect::new(
        canvas_rect.x + canvas_rect.w - 11.0,
        canvas_rect.y + 6.0,
        7.0,
        (canvas_rect.h - 12.0).max(24.0),
    );
    let visible_ratio = (canvas_rect.h / total_content_h.max(1.0)).clamp(0.08, 0.92);
    let thumb_h = (track.h * visible_ratio).max(28.0).min(track.h);
    let travel = (track.h - thumb_h).max(0.0);
    let progress = (scroll_y / max_scroll_y.max(1.0)).clamp(0.0, 1.0);
    let thumb = Rect::new(track.x, track.y + progress * travel, track.w, thumb_h);

    let hit_track = Rect::new(track.x - 4.0, track.y, 14.0, track.h);
    let hov = hit_track.contains(mouse_x, mouse_y);
    if hov {
        surface.fill_rounded_rect(track, 3.5, Color::rgba(200, 194, 182, 90));
    }
    surface.fill_rounded_rect(
        thumb,
        3.5,
        if hov {
            StudioTheme::ink_sub()
        } else {
            StudioTheme::border_strong()
        },
    );
    hit_regions.push(HitRegion::button(
        hit_track,
        UiAction::BeginDrag(DragInteraction::Scrollbar {
            tab_idx,
            track_y: track.y + thumb_h * 0.5,
            track_h: travel.max(1.0),
            max_scroll: max_scroll_y,
        }),
    ));
}

const INLINE_ICON_MAX_BYTES: usize = 24 * 1024;
const THUMB_MAX_DIM: u32 = 1280;
const IMAGE_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct LaidOutBlock {
    pub block_idx: usize,
    pub rel_y: f32,
    pub height: f32,
    pub lines: Vec<LaidOutLine>,
    pub table_row_start: usize,
    pub table_is_last_slice: bool,
    pub table_row_heights: Vec<f32>,
    pub table_wrapped_cells: Vec<Vec<Vec<String>>>,
}

#[derive(Clone)]
pub struct LaidOutLine {
    pub text: String,
    pub char_start: usize,
    pub slot: FontSlot,
    pub px_size: f32,
    pub line_h: f32,
    pub color: Color,
    pub highlight: Option<Color>,
    pub underline: bool,
    pub strike: bool,
}

fn fnv_mix_u64(mut h: u64, val: u64) -> u64 {
    h ^= val;
    h.wrapping_mul(0x0100_0000_01b3)
}

fn fnv_mix_bytes(mut h: u64, bytes: &[u8]) -> u64 {
    h ^= bytes.len() as u64;
    h = h.wrapping_mul(0x0100_0000_01b3);
    // Stride very long cell/paragraph strings (> 512 B) so layout key checks stay < 2 µs
    if bytes.len() <= 512 {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    } else {
        for &b in &bytes[..128] {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let step = (bytes.len() / 128).max(1);
        let mut i = 128;
        while i < bytes.len() {
            h ^= bytes[i] as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
            i += step;
        }
        for &b in &bytes[bytes.len() - 64..] {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

fn mix_single_block(mut h: u64, blk: &Block) -> u64 {
    match blk {
        Block::PageBreak => fnv_mix_u64(h, 0xA1),
        Block::SectionBreak(k) => fnv_mix_u64(h, 0xA6 + (*k as u64)),
        Block::HorizontalRule => fnv_mix_u64(h, 0xA2),
        Block::Image(img) => {
            h = fnv_mix_u64(h, 0xA3);
            h = fnv_mix_u64(h, img.width_pt.to_bits() as u64);
            h = fnv_mix_u64(h, img.height_pt.to_bits() as u64);
            h = fnv_mix_u64(h, img.aspect_ratio().to_bits() as u64);
            fnv_mix_bytes(h, img.caption.as_bytes())
        }
        Block::Table(t) => {
            h = fnv_mix_u64(h, 0xA4);
            h = fnv_mix_u64(h, t.cols as u64);
            h = fnv_mix_u64(h, t.rows.len() as u64);
            let r_step = (t.rows.len() / 32).max(1);
            let mut ri = 0usize;
            while ri < t.rows.len() {
                for cell in &t.rows[ri] {
                    h = fnv_mix_u64(h, cell.bold as u64);
                    h = fnv_mix_bytes(h, cell.text.as_bytes());
                }
                ri += r_step;
            }
            if let Some(last_row) = t.rows.last() {
                for cell in last_row {
                    h = fnv_mix_bytes(h, cell.text.as_bytes());
                }
            }
            h
        }
        Block::Paragraph(p) => {
            h = fnv_mix_u64(h, 0xA5);
            h = fnv_mix_u64(h, p.style as u64);
            h = fnv_mix_u64(h, p.alignment as u64);
            h = fnv_mix_u64(h, p.indent_level as u64);
            h = fnv_mix_u64(h, p.line_spacing.to_bits() as u64);
            let lk_tag = match p.list_kind {
                ListKind::None => 0u64,
                ListKind::Bullet => 1,
                ListKind::Numbered(n) => 2 + (n as u64) * 8,
                ListKind::Checklist(b) => 3 + (b as u64),
            };
            h = fnv_mix_u64(h, lk_tag);
            for sp in &p.spans {
                h = fnv_mix_bytes(h, sp.text.as_bytes());
                h = fnv_mix_u64(
                    h,
                    (sp.style.bold as u64)
                        | ((sp.style.italic as u64) << 1)
                        | ((sp.style.underline as u64) << 2)
                        | ((sp.style.strikethrough as u64) << 3)
                        | ((sp.style.font_family as u64) << 4)
                        | ((sp.style.hidden as u64) << 8)
                        | ((sp.style.superscript as u64) << 9)
                        | ((sp.style.subscript as u64) << 10),
                );
                h = fnv_mix_u64(h, sp.style.font_size_pt.to_bits() as u64);
                if let Some(rev) = &sp.revision {
                    h = fnv_mix_u64(h, 0x9000 + (rev.id as u64) * 4 + (rev.kind as u64));
                }
            }
            h
        }
    }
}

/// Fast `< 5 µs` document content fingerprint: hashes all blocks on compact docs (`<= 192` blocks)
/// and switches to `O(1)` neighborhood + strided sampling on massive 30+ MB multi-hundred-page docs!
fn compute_content_key(w: &WordTabState) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let n = w.doc.blocks.len();
    h = fnv_mix_u64(h, n as u64);
    h = fnv_mix_u64(h, w.doc.track_changes as u64);
    h = fnv_mix_u64(h, w.dirty as u64);
    h = fnv_mix_u64(h, w.doc.comments.len() as u64);
    h = fnv_mix_u64(h, w.doc.page_setup.paper_size as u64);
    h = fnv_mix_u64(h, w.doc.page_setup.landscape as u64);
    h = fnv_mix_u64(
        h,
        w.selected_image_block.map(|i| (i as u64) + 1).unwrap_or(0),
    );
    for (tb_idx, weights) in &w.table_col_weights {
        h = fnv_mix_u64(h, *tb_idx as u64);
        for wt in weights {
            h = fnv_mix_u64(h, wt.to_bits() as u64);
        }
    }

    if n <= 192 {
        for blk in &w.doc.blocks {
            h = mix_single_block(h, blk);
        }
    } else {
        // Always hash exact neighborhood around active caret, active table, and selected image
        let lo = w.cursor_block.saturating_sub(3);
        let hi = (w.cursor_block + 3).min(n.saturating_sub(1));
        for bi in lo..=hi {
            h = fnv_mix_u64(h, bi as u64);
            h = mix_single_block(h, &w.doc.blocks[bi]);
        }
        if let Some((tb, tr, tc)) = w.active_table_cell {
            if let Some(Block::Table(t)) = w.doc.blocks.get(tb) {
                h = mix_single_block(h, &w.doc.blocks[tb]);
                if let Some(cell) = t.rows.get(tr).and_then(|rw| rw.get(tc)) {
                    h = fnv_mix_bytes(h, cell.text.as_bytes());
                }
            }
        }
        if let Some(ib) = w.selected_image_block {
            if let Some(blk) = w.doc.blocks.get(ib) {
                h = mix_single_block(h, blk);
            }
        }
        // Plus 64 strided global samples so bulk ops (`AcceptAll`, `ReplaceAll`) invalidate too
        let step = (n / 64).max(1);
        let mut bi = 0usize;
        while bi < n {
            h = mix_single_block(h, &w.doc.blocks[bi]);
            bi += step;
        }
    }
    h.max(1)
}

fn ensure_cached_layout(
    fonts: &mut FontBook,
    w: &mut WordTabState,
    content_key: u64,
    scale: f32,
    content_w: f32,
    usable_page_h: f32,
) -> std::rc::Rc<Vec<Vec<LaidOutBlock>>> {
    let mut key = content_key;
    key = fnv_mix_u64(key, rustpad_render::active_theme_mode() as u64);
    key = fnv_mix_u64(key, scale.to_bits() as u64);
    key = fnv_mix_u64(key, content_w.to_bits() as u64);
    key = fnv_mix_u64(key, usable_page_h.to_bits() as u64);
    let key = key.max(1);

    if w.cached_layout_key == key && !w.cached_pages.is_empty() {
        return w.cached_pages.clone();
    }
    let built = std::rc::Rc::new(layout_document_pages(
        fonts,
        w,
        scale,
        content_w,
        usable_page_h,
    ));
    w.cached_layout_key = key;
    w.cached_pages = built.clone();
    built
}

fn warm_visible_page_images(
    w: &mut WordTabState,
    page_items: &[LaidOutBlock],
    content_origin_y: f32,
    canvas_rect: Rect,
    decoded_this_frame: &mut usize,
) {
    let prefetch_top = canvas_rect.y - 220.0;
    let prefetch_bot = canvas_rect.y + canvas_rect.h + 220.0;

    for item in page_items {
        let blk_top = content_origin_y + item.rel_y;
        let blk_bot = blk_top + item.height;
        if blk_bot < prefetch_top || blk_top > prefetch_bot {
            continue;
        }
        let payload = {
            let Some(Block::Image(img)) = w.doc.blocks.get(item.block_idx) else {
                continue;
            };
            if img.rgba_preview.is_some() || img.png_bytes.is_empty() {
                continue;
            }
            img.png_bytes.clone()
        };
        let hkey = hash_image_bytes(&payload);
        if let Some(pos) = w.image_rgba_cache.iter().position(|(k, _, _, _)| *k == hkey) {
            let last = w.image_rgba_cache.len() - 1;
            if pos != last {
                let entry = w.image_rgba_cache.remove(pos);
                w.image_rgba_cache.push(entry);
            }
            continue;
        }
        if w.failed_image_hashes.contains(&hkey) || w.in_flight_image_hashes.contains(&hkey) {
            continue;
        }

        // Tiny inline icons (`<= 24 KiB`) decode in `< 0.3 ms`; anything larger (> 24 KiB,
        // e.g. Retina/4K/8K screenshots) decodes asynchronously on `rustpad-image-decoder`
        // with 0.0 ms blocking on the Win32 UI thread!
        if payload.len() > INLINE_ICON_MAX_BYTES || *decoded_this_frame >= 1 {
            w.enqueue_image_decode(hkey, item.block_idx, payload, THUMB_MAX_DIM);
            continue;
        }

        *decoded_this_frame += 1;
        match decode_any_image_thumbnail_rgba(&payload, THUMB_MAX_DIM) {
            Ok((orig_w, orig_h, tw, th, rgba)) => {
                if let Some(Block::Image(img_mut)) = w.doc.blocks.get_mut(item.block_idx) {
                    if img_mut.intrinsic_px.is_none() {
                        img_mut.intrinsic_px = Some((orig_w, orig_h));
                    }
                }
                let added_bytes = rgba.len();
                let shared_rgba = SharedBytes::from_vec(rgba);
                while !w.image_rgba_cache.is_empty()
                    && w.image_rgba_cache_bytes.saturating_add(added_bytes) > IMAGE_CACHE_MAX_BYTES
                {
                    let removed = w.image_rgba_cache.remove(0);
                    w.image_rgba_cache_bytes =
                        w.image_rgba_cache_bytes.saturating_sub(removed.3.len());
                }
                w.image_rgba_cache_bytes = w.image_rgba_cache_bytes.saturating_add(added_bytes);
                w.image_rgba_cache.push((hkey, tw, th, shared_rgba));
            }
            Err(_) => {
                if w.failed_image_hashes.len() >= 4096 {
                    w.failed_image_hashes.remove(0);
                }
                w.failed_image_hashes.push(hkey);
            }
        }
    }
}

fn layout_document_pages(
    fonts: &mut FontBook,
    w: &WordTabState,
    scale: f32,
    content_w: f32,
    usable_page_h: f32,
) -> Vec<Vec<LaidOutBlock>> {
    let mut pages: Vec<Vec<LaidOutBlock>> = vec![Vec::new()];
    let mut cur_y = 0.0f32;

    for (b_idx, block) in w.doc.blocks.iter().enumerate() {
        match block {
            Block::PageBreak => {
                pages.push(Vec::new());
                cur_y = 0.0;
            }
            Block::SectionBreak(kind) => {
                if *kind == doc_model::SectionBreakKind::Continuous {
                    let h = 20.0 * scale;
                    if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                        pages.push(Vec::new());
                        cur_y = 0.0;
                    }
                    pages.last_mut().unwrap().push(LaidOutBlock {
                        block_idx: b_idx,
                        rel_y: cur_y,
                        height: h,
                        lines: Vec::new(),
                        table_row_start: 0,
                        table_is_last_slice: true,
                        table_row_heights: Vec::new(),
                        table_wrapped_cells: Vec::new(),
                    });
                    cur_y += h;
                } else {
                    pages.push(Vec::new());
                    cur_y = 0.0;
                }
            }
            Block::HorizontalRule => {
                let h = 18.0 * scale;
                if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = 0.0;
                }
                pages.last_mut().unwrap().push(LaidOutBlock {
                    block_idx: b_idx,
                    rel_y: cur_y,
                    height: h,
                    lines: Vec::new(),
                    table_row_start: 0,
                    table_is_last_slice: true,
                    table_row_heights: Vec::new(),
                    table_wrapped_cells: Vec::new(),
                });
                cur_y += h;
            }
            Block::Image(img) => {
                let disp_w = (img.width_pt * scale).clamp(48.0 * scale, content_w);
                let disp_h =
                    (disp_w / img.aspect_ratio()).clamp(36.0 * scale, usable_page_h * 0.82);
                let show_cap_row =
                    !img.caption.trim().is_empty() || w.selected_image_block == Some(b_idx);
                let cap_extra = if show_cap_row { 20.0 * scale } else { 0.0 };
                let h = disp_h + cap_extra + 14.0 * scale;
                if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = 0.0;
                }
                pages.last_mut().unwrap().push(LaidOutBlock {
                    block_idx: b_idx,
                    rel_y: cur_y,
                    height: h,
                    lines: Vec::new(),
                    table_row_start: 0,
                    table_is_last_slice: true,
                    table_row_heights: Vec::new(),
                    table_wrapped_cells: Vec::new(),
                });
                cur_y += h;
            }
            Block::Table(t) => {
                let cols = t.cols.max(1);
                let weights = w.normalized_col_weights(b_idx, cols);
                let txt_px = (11.5 * scale).clamp(10.0, 20.0);
                let line_h = (txt_px * 1.36).max(14.0);
                let cell_pad_x = 9.0 * scale;
                let cell_pad_y = 7.5 * scale;

                let mut row_heights = Vec::with_capacity(t.rows.len());
                let mut wrapped_grid = Vec::with_capacity(t.rows.len());

                for (r_idx, row) in t.rows.iter().enumerate() {
                    let mut max_lines = 1usize;
                    let mut wrapped_row = Vec::with_capacity(cols);
                    for c_idx in 0..cols {
                        let cw = (content_w * weights[c_idx] - cell_pad_x * 2.0).max(28.0);
                        let cell_txt = row.get(c_idx).map(|c| c.text.as_str()).unwrap_or("");
                        let slot = if r_idx == 0 || row.get(c_idx).map(|c| c.bold).unwrap_or(false)
                        {
                            FontSlot::SansBold
                        } else {
                            FontSlot::SansRegular
                        };
                        let wlines = wrap_plain_text(fonts, slot, txt_px, cell_txt, cw);
                        max_lines = max_lines.max(wlines.len().max(1));
                        wrapped_row.push(wlines);
                    }
                    row_heights.push((max_lines as f32) * line_h + cell_pad_y * 2.0);
                    wrapped_grid.push(wrapped_row);
                }

                // Multi-page table row-range pagination so 50..5000-row enterprise tables
                // flow cleanly across physical sheets without overflowing a single page!
                let total_rows = row_heights.len();
                if total_rows == 0 {
                    continue;
                }
                let mut r_cursor = 0usize;
                while r_cursor < total_rows {
                    let first_rh = row_heights[r_cursor];
                    if cur_y + first_rh > usable_page_h && !pages.last().unwrap().is_empty() {
                        pages.push(Vec::new());
                        cur_y = 0.0;
                    }
                    let budget = (usable_page_h - cur_y).max(first_rh);
                    let mut acc_h = 0.0f32;
                    let mut r_end = r_cursor;
                    while r_end < total_rows {
                        let rh = row_heights[r_end];
                        let tail_pad = if r_end + 1 == total_rows {
                            26.0 * scale
                        } else {
                            6.0 * scale
                        };
                        if r_end > r_cursor && acc_h + rh + tail_pad > budget {
                            break;
                        }
                        acc_h += rh;
                        r_end += 1;
                    }
                    let is_last = r_end >= total_rows;
                    let slice_h = acc_h + if is_last { 26.0 * scale } else { 8.0 * scale };
                    pages.last_mut().unwrap().push(LaidOutBlock {
                        block_idx: b_idx,
                        rel_y: cur_y,
                        height: slice_h,
                        lines: Vec::new(),
                        table_row_start: r_cursor,
                        table_is_last_slice: is_last,
                        table_row_heights: row_heights[r_cursor..r_end].to_vec(),
                        table_wrapped_cells: wrapped_grid[r_cursor..r_end].to_vec(),
                    });
                    cur_y += slice_h;
                    r_cursor = r_end;
                    if !is_last {
                        pages.push(Vec::new());
                        cur_y = 0.0;
                    }
                }
            }
            Block::Paragraph(p) => {
                let indent = (p.indent_level as f32) * 20.0 * scale
                    + if p.list_kind != ListKind::None || p.style == ParagraphStyle::Quote {
                        20.0 * scale
                    } else {
                        0.0
                    };
                let wrap_w = (content_w - indent).max(60.0);
                let lines = wrap_paragraph_lines(fonts, p, scale, wrap_w);
                let spacing_after = match p.style {
                    ParagraphStyle::Title => 12.0 * scale,
                    ParagraphStyle::Subtitle => 10.0 * scale,
                    ParagraphStyle::Heading1 | ParagraphStyle::Heading2 => 9.0 * scale,
                    _ => 7.0 * scale,
                };
                let spacing_before = match p.style {
                    ParagraphStyle::Heading1 => 10.0 * scale,
                    ParagraphStyle::Heading2 => 7.0 * scale,
                    ParagraphStyle::Heading3 => 5.0 * scale,
                    _ => 0.0,
                };
                let text_h: f32 = lines.iter().map(|l| l.line_h).sum();
                let h = spacing_before + text_h + spacing_after;
                if h <= usable_page_h {
                    if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                        pages.push(Vec::new());
                        cur_y = 0.0;
                    }
                    pages.last_mut().unwrap().push(LaidOutBlock {
                        block_idx: b_idx,
                        rel_y: cur_y + spacing_before,
                        height: h,
                        lines,
                        table_row_start: 0,
                        table_is_last_slice: true,
                        table_row_heights: Vec::new(),
                        table_wrapped_cells: Vec::new(),
                    });
                    cur_y += h;
                } else {
                    // Multi-page paragraph line slicing for huge pasted logs/clauses
                    let mut l_cur = 0usize;
                    while l_cur < lines.len() {
                        let sb = if l_cur == 0 { spacing_before } else { 0.0 };
                        let first_lh = lines[l_cur].line_h;
                        if cur_y + sb + first_lh > usable_page_h
                            && !pages.last().unwrap().is_empty()
                        {
                            pages.push(Vec::new());
                            cur_y = 0.0;
                        }
                        let budget = (usable_page_h - cur_y - sb).max(first_lh);
                        let mut acc = 0.0f32;
                        let mut l_end = l_cur;
                        while l_end < lines.len() {
                            let lh = lines[l_end].line_h;
                            if l_end > l_cur && acc + lh > budget {
                                break;
                            }
                            acc += lh;
                            l_end += 1;
                        }
                        let sa = if l_end == lines.len() {
                            spacing_after
                        } else {
                            0.0
                        };
                        let sh = sb + acc + sa;
                        pages.last_mut().unwrap().push(LaidOutBlock {
                            block_idx: b_idx,
                            rel_y: cur_y + sb,
                            height: sh,
                            lines: lines[l_cur..l_end].to_vec(),
                            table_row_start: if l_cur == 0 { 0 } else { 1 },
                            table_is_last_slice: l_end == lines.len(),
                            table_row_heights: Vec::new(),
                            table_wrapped_cells: Vec::new(),
                        });
                        cur_y += sh;
                        l_cur = l_end;
                        if l_cur < lines.len() {
                            pages.push(Vec::new());
                            cur_y = 0.0;
                        }
                    }
                }
            }
        }
    }
    pages
}

fn wrap_plain_text(
    fonts: &mut FontBook,
    slot: FontSlot,
    px_size: f32,
    text: &str,
    max_w: f32,
) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0.0f32;
    for word in text.split_inclusive(' ') {
        let word_w = fonts.measure_text(slot, word, px_size);
        if !cur.is_empty() && cur_w + word_w > max_w {
            lines.push(cur.trim_end().to_string());
            cur.clear();
            cur_w = 0.0;
        }
        if cur.is_empty() && word_w > max_w {
            let mut chunk = String::new();
            let mut chunk_w = 0.0f32;
            let mut ch_buf = [0u8; 4];
            for ch in word.chars() {
                let s_ch = ch.encode_utf8(&mut ch_buf);
                let cw = fonts.measure_text(slot, s_ch, px_size);
                if !chunk.is_empty() && chunk_w + cw > max_w {
                    lines.push(chunk);
                    chunk = s_ch.to_string();
                    chunk_w = cw;
                } else {
                    chunk.push(ch);
                    chunk_w += cw;
                }
            }
            cur = chunk;
            cur_w = chunk_w;
        } else {
            cur.push_str(word);
            cur_w += word_w;
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

fn wrap_paragraph_lines(
    fonts: &mut FontBook,
    p: &Paragraph,
    scale: f32,
    wrap_w: f32,
) -> Vec<LaidOutLine> {
    let first_span = p.spans.first();
    let base_pt = first_span
        .map(|s| s.style.font_size_pt)
        .unwrap_or_else(|| p.style.default_font_size());
    let px_size = (base_pt * scale).clamp(9.5, 58.0);
    let bold = first_span
        .map(|s| s.style.bold)
        .unwrap_or_else(|| p.style.is_bold_by_default());
    let fam = first_span
        .map(|s| s.style.font_family)
        .unwrap_or(FontFamily::Sans);
    let slot = match (fam, bold) {
        (FontFamily::Mono, _) => FontSlot::MonoRegular,
        (FontFamily::Serif, _) => FontSlot::SerifRegular,
        (FontFamily::Sans, true) => FontSlot::SansBold,
        (FontFamily::Sans, false) => FontSlot::SansRegular,
    };
    let col = first_span
        .map(|s| StudioTheme::adapt_doc_ink(s.style.color, s.style.highlight.is_some()))
        .unwrap_or_else(StudioTheme::sheet_ink);
    let hl = first_span
        .and_then(|s| s.style.highlight)
        .map(|h| Color::rgb(h.r, h.g, h.b));
    let underline = first_span.map(|s| s.style.underline).unwrap_or(false);
    let strike = first_span.map(|s| s.style.strikethrough).unwrap_or(false);
    let line_h = px_size * p.line_spacing.max(1.32);

    let full_text = p.plain_text();
    if full_text.is_empty() {
        return vec![LaidOutLine {
            text: String::new(),
            char_start: 0,
            slot,
            px_size,
            line_h,
            color: col,
            highlight: hl,
            underline,
            strike,
        }];
    }

    let mut out = Vec::new();
    let mut cur_line = String::new();
    let mut cur_w = 0.0f32;
    let mut cur_start = 0usize;
    let mut char_cursor = 0usize;

    for token in full_text.split_inclusive(' ') {
        let tok_w = fonts.measure_text(slot, token, px_size);
        if !cur_line.is_empty() && cur_w + tok_w > wrap_w {
            out.push(LaidOutLine {
                text: cur_line,
                char_start: cur_start,
                slot,
                px_size,
                line_h,
                color: col,
                highlight: hl,
                underline,
                strike,
            });
            cur_line = token.to_string();
            cur_w = tok_w;
            cur_start = char_cursor;
        } else {
            cur_line.push_str(token);
            cur_w += tok_w;
        }
        char_cursor += token.chars().count();
    }

    if !cur_line.is_empty() {
        out.push(LaidOutLine {
            text: cur_line,
            char_start: cur_start,
            slot,
            px_size,
            line_h,
            color: col,
            highlight: hl,
            underline,
            strike,
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn paint_laid_out_block(
    surface: &mut Surface,
    fonts: &mut FontBook,
    spellchecker: &SpellChecker,
    hit_regions: &mut Vec<HitRegion>,
    word_line_boxes: &mut Vec<WordLineHitBox>,
    sel_bar_anchor: &mut Option<Rect>,
    tab_idx: usize,
    is_focused: bool,
    cursor_blink_on: bool,
    mouse_x: f32,
    mouse_y: f32,
    w: &WordTabState,
    item: &LaidOutBlock,
    x: f32,
    y: f32,
    content_w: f32,
    scale: f32,
) {
    let Some(block) = w.doc.blocks.get(item.block_idx) else {
        return;
    };

    match block {
        Block::PageBreak => {}
        Block::SectionBreak(kind) => {
            if *kind == doc_model::SectionBreakKind::Continuous {
                surface.fill_rect(
                    Rect::new(x, y + item.height * 0.50, content_w, 1.0),
                    StudioTheme::sheet_border(),
                );
                surface.draw_text(
                    fonts,
                    FontSlot::SansRegular,
                    (8.8 * scale).clamp(8.0, 11.5),
                    x + content_w * 0.38,
                    y + item.height * 0.42,
                    StudioTheme::sheet_ink_muted(),
                    "··· SECTION BREAK ···",
                );
            }
        }
        Block::HorizontalRule => {
            surface.fill_rect(
                Rect::new(x, y + item.height * 0.45, content_w, 1.5),
                StudioTheme::sheet_border(),
            );
        }
        Block::Image(img) => {
            let disp_w = (img.width_pt * scale).clamp(48.0 * scale, content_w);
            let show_cap_row =
                !img.caption.trim().is_empty() || w.selected_image_block == Some(item.block_idx);
            let cap_extra = if show_cap_row { 20.0 * scale } else { 0.0 };
            let box_h = (item.height - cap_extra - 14.0 * scale).max(36.0);
            let img_x = match img.alignment {
                Alignment::Center => x + (content_w - disp_w) * 0.5,
                Alignment::Right => x + (content_w - disp_w),
                _ => x,
            };
            let r = Rect::new(img_x, y + 3.0 * scale, disp_w, box_h);
            let is_selected = is_focused && w.selected_image_block == Some(item.block_idx);
            let is_hov = r.contains(mouse_x, mouse_y);

            surface.draw_paper_shadow(r, 4.0, 8.0, 14);
            surface.fill_rounded_rect(r, 4.0, StudioTheme::paper());
            surface.stroke_rounded_rect(
                r,
                4.0,
                if is_selected { 2.0 } else { 1.0 },
                if is_selected {
                    StudioTheme::word_accent()
                } else if is_hov {
                    StudioTheme::rust_orange()
                } else {
                    StudioTheme::border_strong()
                },
            );

            let inner = Rect::new(
                r.x + 2.0,
                r.y + 2.0,
                (r.w - 4.0).max(2.0),
                (r.h - 4.0).max(2.0),
            );
            if let Some((iw, ih, rgba)) = &img.rgba_preview {
                surface.blit_rgba_scaled(inner, *iw, *ih, rgba);
            } else {
                let hkey = hash_image_bytes(&img.png_bytes);
                if let Some((_, iw, ih, rgba)) =
                    w.image_rgba_cache.iter().find(|(k, _, _, _)| *k == hkey)
                {
                    surface.blit_rgba_scaled(inner, *iw, *ih, rgba);
                } else {
                    let is_failed = w.failed_image_hashes.contains(&hkey);
                    let status_lbl = if is_failed {
                        format!("Figure: {} (Unsupported Legacy Media)", img.name)
                    } else {
                        format!("Streaming HiDPI Figure · {}", img.name)
                    };
                    surface.fill_rounded_rect(inner, 3.0, StudioTheme::sidebar());
                    surface.draw_icon(
                        UiIcon::Image,
                        r.x + 22.0 * scale,
                        r.y + r.h * 0.5,
                        18.0 * scale,
                        StudioTheme::ink_muted(),
                    );
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SansBold,
                        11.5 * scale,
                        r.x + 38.0 * scale,
                        r.y + r.h * 0.5 + 4.0 * scale,
                        (r.w - 46.0 * scale).max(24.0),
                        StudioTheme::ink_sub(),
                        &status_lbl,
                    );
                }
            }

            hit_regions.push(HitRegion::button(
                r,
                UiAction::SelectWordImage {
                    tab_idx,
                    block_idx: item.block_idx,
                },
            ));

            // Figure Caption (or `+ Add figure caption…` quick pill when selected)
            if !img.caption.trim().is_empty() {
                let cap_px = (10.2 * scale).clamp(9.0, 14.0);
                let cap_txt = img.caption.trim();
                let cap_w = fonts.measure_text(FontSlot::SerifRegular, cap_txt, cap_px);
                let cap_x = match img.alignment {
                    Alignment::Center => r.x + ((r.w - cap_w) * 0.5).max(0.0),
                    Alignment::Right => r.x + (r.w - cap_w).max(0.0),
                    _ => r.x,
                };
                let cap_hit = Rect::new(r.x, r.y + r.h + 2.0 * scale, r.w, 18.0 * scale);
                surface.draw_text_clipped(
                    fonts,
                    FontSlot::SerifRegular,
                    cap_px,
                    cap_x,
                    r.y + r.h + 14.0 * scale,
                    r.w,
                    StudioTheme::ink_sub(),
                    cap_txt,
                );
                hit_regions.push(HitRegion::button(
                    cap_hit,
                    UiAction::ExecCommand(CommandId::ImageEditCaption),
                ));
            } else if is_selected {
                let pill_lbl = "+ Add figure caption…";
                let pill_px = (9.8 * scale).clamp(9.0, 12.5);
                let ptw = fonts.measure_text(FontSlot::SansRegular, pill_lbl, pill_px);
                let pw = (ptw + 14.0 * scale).min(content_w);
                let px = r.x + ((r.w - pw) * 0.5).max(0.0);
                let pill_r = Rect::new(px, r.y + r.h + 3.0 * scale, pw, 16.5 * scale);
                let phov = pill_r.contains(mouse_x, mouse_y);
                surface.fill_rounded_rect(
                    pill_r,
                    3.5,
                    if phov {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::sidebar()
                    },
                );
                surface.stroke_rounded_rect(pill_r, 3.5, 1.0, StudioTheme::border());
                surface.draw_text(
                    fonts,
                    FontSlot::SansRegular,
                    pill_px,
                    pill_r.x + (pw - ptw) * 0.5,
                    pill_r.y + pill_r.h * 0.72,
                    if phov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::ink_muted()
                    },
                    pill_lbl,
                );
                hit_regions.push(HitRegion::button(
                    pill_r,
                    UiAction::ExecCommand(CommandId::ImageEditCaption),
                ));
            }

            // Live Selection Overlay: Dimension Badge + 60–120Hz Aspect-Locked Corner Resize Handles!
            if is_selected || is_hov {
                let max_w_pt = (content_w / scale.max(0.4)).max(120.0);
                let pct = ((img.width_pt / max_w_pt) * 100.0).round().clamp(10.0, 100.0) as i32;
                let badge_txt =
                    format!("{:.0}×{:.0} pt · {pct}%", img.width_pt, img.height_pt);
                let bpx = (9.5 * scale).clamp(8.5, 12.0);
                let btw = fonts.measure_text(FontSlot::MonoRegular, &badge_txt, bpx);
                let bw = btw + 12.0 * scale;
                let bh = 17.0 * scale;
                if bw + 12.0 < r.w {
                    let badge_r =
                        Rect::new(r.x + r.w - bw - 6.0 * scale, r.y + 6.0 * scale, bw, bh);
                    surface.fill_rounded_rect(badge_r, 3.5, Color::rgba(24, 28, 34, 205));
                    surface.draw_text(
                        fonts,
                        FontSlot::MonoRegular,
                        bpx,
                        badge_r.x + 6.0 * scale,
                        badge_r.y + bh * 0.73,
                        StudioTheme::paper(),
                        &badge_txt,
                    );
                }

                let hs = (9.0 * scale).clamp(7.5, 12.0);
                let hit_pad = (6.0 * scale).clamp(5.0, 9.0);
                let right_factor = match img.alignment {
                    Alignment::Center => 2.0,
                    Alignment::Right => -1.0,
                    _ => 1.0,
                };
                let left_factor = match img.alignment {
                    Alignment::Center => -2.0,
                    Alignment::Right => -1.0,
                    _ => -1.0,
                };

                for (hx, hy, factor) in [
                    (r.x + r.w, r.y + r.h, right_factor),
                    (r.x + r.w, r.y + r.h * 0.5, right_factor),
                    (r.x, r.y + r.h, left_factor),
                ] {
                    let vis_h = Rect::new(hx - hs * 0.5, hy - hs * 0.5, hs, hs);
                    let hit_h = Rect::new(
                        hx - hs * 0.5 - hit_pad,
                        hy - hs * 0.5 - hit_pad,
                        hs + hit_pad * 2.0,
                        hs + hit_pad * 2.0,
                    );
                    surface.fill_rounded_rect(vis_h, 2.0, StudioTheme::paper());
                    surface.stroke_rounded_rect(vis_h, 2.0, 1.8, StudioTheme::word_accent());
                    hit_regions.push(HitRegion::splitter(
                        hit_h,
                        UiAction::BeginDrag(DragInteraction::ImageResize {
                            tab_idx,
                            block_idx: item.block_idx,
                            start_x: mouse_x,
                            start_width_pt: img.width_pt,
                            scale,
                            anchor_factor: factor,
                            max_width_pt: max_w_pt,
                        }),
                    ));
                }
            }
        }
        Block::Table(t) => {
            let cols = t.cols.max(1);
            let weights = w.normalized_col_weights(item.block_idx, cols);
            let txt_px = (11.5 * scale).clamp(10.0, 20.0);
            let line_h = (txt_px * 1.36).max(14.0);
            let cell_pad_x = 9.0 * scale;
            let cell_pad_y = 7.5 * scale;
            let border_col = StudioTheme::adapt_table_border(t.border_color);

            let table_body_h: f32 = item.table_row_heights.iter().sum();
            let table_outer = Rect::new(x - 4.0, y - 4.0, content_w + 28.0 * scale, table_body_h + 26.0 * scale);
            let table_active_or_hovered = w
                .active_table_cell
                .map(|(b, _, _)| b == item.block_idx)
                .unwrap_or(false)
                || table_outer.contains(mouse_x, mouse_y);

            let mut cur_row_y = y;
            for (local_r, &rh) in item.table_row_heights.iter().enumerate() {
                let r_idx = item.table_row_start + local_r;
                let Some(row) = t.rows.get(r_idx) else {
                    break;
                };
                let mut cx = x;

                for c_idx in 0..cols {
                    let cw = content_w * weights[c_idx];
                    let cell_r = Rect::new(cx, cur_row_y, cw, rh);
                    let cell_opt = row.get(c_idx);
                    if let Some(bg) = cell_opt.and_then(|c| c.bg_color) {
                        surface.fill_rect(cell_r, StudioTheme::adapt_table_cell_bg(bg));
                    }

                    let is_active_cell = is_focused
                        && w.active_table_cell == Some((item.block_idx, r_idx, c_idx));

                    surface.stroke_rounded_rect(
                        cell_r,
                        0.0,
                        if is_active_cell { 1.8 } else { 1.0 },
                        if is_active_cell {
                            StudioTheme::word_accent()
                        } else {
                            border_col
                        },
                    );

                    let slot = if r_idx == 0 || cell_opt.map(|c| c.bold).unwrap_or(false) {
                        FontSlot::SansBold
                    } else {
                        FontSlot::SansRegular
                    };
                    let align = cell_opt.map(|c| c.alignment).unwrap_or(Alignment::Left);
                    let cell_lines = item
                        .table_wrapped_cells
                        .get(local_r)
                        .and_then(|rw| rw.get(c_idx));

                    let mut char_running = 0usize;
                    let mut clicked_char_est = cell_opt.map(|c| c.text.chars().count()).unwrap_or(0);
                    let mut caret_drawn = false;

                    if let Some(wlines) = cell_lines {
                        for (li, ln) in wlines.iter().enumerate() {
                            let lw = fonts.measure_text(slot, ln, txt_px);
                            let tx = match align {
                                Alignment::Center => cell_r.x + ((cw - lw) * 0.5).max(cell_pad_x),
                                Alignment::Right => (cell_r.x + cw - cell_pad_x - lw).max(cell_r.x + cell_pad_x),
                                _ => cell_r.x + cell_pad_x,
                            };
                            let baseline =
                                cell_r.y + cell_pad_y + txt_px * 0.88 + (li as f32) * line_h;
                            let is_untouched_header_placeholder = r_idx == 0
                                && cell_opt
                                    .map(|c| c.text == format!("Column {}", c_idx + 1))
                                    .unwrap_or(false);
                            if !ln.is_empty() {
                                surface.draw_text(
                                    fonts,
                                    slot,
                                    txt_px,
                                    tx,
                                    baseline,
                                    if is_untouched_header_placeholder {
                                        StudioTheme::sheet_ink_muted()
                                    } else {
                                        StudioTheme::sheet_ink()
                                    },
                                    ln,
                                );
                            }

                            let ln_chars = ln.chars().count();
                            let line_band_top = cell_r.y + cell_pad_y + (li as f32) * line_h - 3.0;
                            let line_band_bot = line_band_top + line_h + 4.0;
                            if cell_r.contains(mouse_x, mouse_y)
                                && mouse_y >= line_band_top
                                && mouse_y <= line_band_bot
                            {
                                clicked_char_est = char_running
                                    + char_offset_from_x(fonts, slot, txt_px, ln, mouse_x - tx);
                            }

                            // Draw blinking caret inside active table cell!
                            if is_active_cell && cursor_blink_on && !caret_drawn {
                                let caret_pos = w
                                    .table_cursor_char
                                    .min(cell_opt.map(|c| c.text.chars().count()).unwrap_or(0));
                                let is_last_line = li + 1 == wlines.len();
                                if (caret_pos >= char_running && caret_pos <= char_running + ln_chars)
                                    || is_last_line
                                {
                                    let local_off = caret_pos.saturating_sub(char_running).min(ln_chars);
                                    let prefix: String = ln.chars().take(local_off).collect();
                                    let cx_caret = tx + fonts.measure_text(slot, &prefix, txt_px);
                                    surface.fill_rect(
                                        Rect::new(
                                            cx_caret,
                                            baseline - txt_px * 0.86,
                                            2.0,
                                            txt_px * 1.15,
                                        ),
                                        StudioTheme::word_accent(),
                                    );
                                    caret_drawn = true;
                                }
                            }
                            // Account for space consumed between wrapped lines
                            char_running += ln_chars;
                            if let Some(cell) = cell_opt {
                                if cell.text.chars().nth(char_running) == Some(' ') {
                                    char_running += 1;
                                }
                            }
                        }
                    }

                    hit_regions.push(HitRegion::text(
                        cell_r,
                        UiAction::ClickTableCell {
                            tab_idx,
                            block_idx: item.block_idx,
                            row: r_idx,
                            col: c_idx,
                            char_idx: clicked_char_est,
                        },
                    ));

                    cx += cw;
                }
                cur_row_y += rh;
            }

            // Column Border Drag-Resize Handles across the full height of the table!
            let mut div_x = x;
            for c_idx in 0..cols.saturating_sub(1) {
                div_x += content_w * weights[c_idx];
                let grip = Rect::new(div_x - 3.5, y, 7.0, table_body_h);
                if grip.contains(mouse_x, mouse_y) {
                    surface.fill_rect(
                        Rect::new(div_x - 1.0, y, 2.0, table_body_h),
                        StudioTheme::rust_orange(),
                    );
                }
                hit_regions.push(HitRegion::splitter(
                    grip,
                    UiAction::BeginDrag(DragInteraction::TableColumn {
                        tab_idx,
                        block_idx: item.block_idx,
                        cols,
                        col_idx: c_idx,
                        last_x: mouse_x,
                        table_px_w: content_w,
                    }),
                ));
            }

            // On-Canvas `+ Add Row` bottom bar & `+ Col` right edge button!
            if table_active_or_hovered && item.table_is_last_slice {
                let row_btn = Rect::new(x, y + table_body_h + 2.0, content_w, (19.0 * scale).clamp(17.0, 26.0));
                let row_hov = row_btn.contains(mouse_x, mouse_y);
                surface.fill_rounded_rect(
                    row_btn,
                    3.0,
                    if row_hov {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::sidebar()
                    },
                );
                surface.stroke_rounded_rect(
                    row_btn,
                    3.0,
                    1.0,
                    if row_hov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::border()
                    },
                );
                let r_lbl = "+ Add Row";
                let r_px = (10.2 * scale).clamp(9.5, 13.5);
                let rw = fonts.measure_text(FontSlot::SansBold, r_lbl, r_px);
                surface.draw_text(
                    fonts,
                    FontSlot::SansBold,
                    r_px,
                    row_btn.x + (row_btn.w - rw) * 0.5,
                    row_btn.y + row_btn.h * 0.72,
                    if row_hov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::ink_sub()
                    },
                    r_lbl,
                );
                hit_regions.push(HitRegion::button(
                    row_btn,
                    UiAction::TableQuickAddRow {
                        tab_idx,
                        block_idx: item.block_idx,
                    },
                ));

                // Right-edge `+ Col` rail
                let col_btn = Rect::new(
                    x + content_w + 3.0,
                    y,
                    (21.0 * scale).clamp(18.0, 28.0),
                    table_body_h.max(24.0),
                );
                let col_hov = col_btn.contains(mouse_x, mouse_y);
                surface.fill_rounded_rect(
                    col_btn,
                    3.0,
                    if col_hov {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::sidebar()
                    },
                );
                surface.stroke_rounded_rect(
                    col_btn,
                    3.0,
                    1.0,
                    if col_hov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::border()
                    },
                );
                surface.draw_icon(
                    UiIcon::Plus,
                    col_btn.x + col_btn.w * 0.5,
                    col_btn.y + col_btn.h * 0.5,
                    (11.0 * scale).clamp(9.0, 14.0),
                    if col_hov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::ink_sub()
                    },
                );
                hit_regions.push(HitRegion::button(
                    col_btn,
                    UiAction::TableQuickAddCol {
                        tab_idx,
                        block_idx: item.block_idx,
                    },
                ));
            }
        }
        Block::Paragraph(p) => {
            paint_paragraph_block(
                surface,
                fonts,
                spellchecker,
                hit_regions,
                word_line_boxes,
                sel_bar_anchor,
                tab_idx,
                is_focused,
                cursor_blink_on,
                mouse_x,
                mouse_y,
                w,
                p,
                item,
                x,
                y,
                content_w,
                scale,
            );
        }
    }
}

fn compute_line_char_xs(
    fonts: &mut FontBook,
    slot: FontSlot,
    px_size: f32,
    draw_x: f32,
    text: &str,
) -> Vec<f32> {
    let mut xs = Vec::with_capacity(text.chars().count() + 1);
    let mut pen = draw_x;
    xs.push(pen);
    for ch in text.chars() {
        pen += fonts.glyph(slot, ch, px_size).advance_px;
        xs.push(pen);
    }
    xs
}

fn char_offset_from_x(
    fonts: &mut FontBook,
    slot: FontSlot,
    px_size: f32,
    line: &str,
    rel_x: f32,
) -> usize {
    if rel_x <= 0.0 || line.is_empty() {
        return 0;
    }
    let mut acc = 0.0f32;
    for (idx, ch) in line.chars().enumerate() {
        let adv = fonts.glyph(slot, ch, px_size).advance_px;
        if rel_x < acc + adv * 0.52 {
            return idx;
        }
        acc += adv;
    }
    line.chars().count()
}

#[allow(clippy::too_many_arguments)]
fn paint_paragraph_block(
    surface: &mut Surface,
    fonts: &mut FontBook,
    spellchecker: &SpellChecker,
    hit_regions: &mut Vec<HitRegion>,
    word_line_boxes: &mut Vec<WordLineHitBox>,
    sel_bar_anchor: &mut Option<Rect>,
    tab_idx: usize,
    is_focused: bool,
    cursor_blink_on: bool,
    mouse_x: f32,
    mouse_y: f32,
    w: &WordTabState,
    p: &Paragraph,
    item: &LaidOutBlock,
    x: f32,
    y: f32,
    content_w: f32,
    scale: f32,
) {
    let indent_px = (p.indent_level as f32) * 20.0 * scale;
    let mut text_x = x + indent_px;

    // Legal Redline Change Bar in Left Sheet Gutter when paragraph carries tracked revisions
    if p.has_revisions() {
        surface.fill_rounded_rect(
            Rect::new(
                (x - 10.5 * scale).max(4.0),
                y + 1.5,
                (2.8 * scale).max(2.2),
                (item.height - 6.0 * scale).max(15.0),
            ),
            1.2,
            StudioTheme::pdf_accent(),
        );
    }

    if p.style == ParagraphStyle::Quote {
        surface.fill_rounded_rect(
            Rect::new(
                text_x,
                y + 2.0,
                3.0 * scale,
                (item.height - 8.0 * scale).max(16.0),
            ),
            1.5,
            StudioTheme::rust_orange(),
        );
        text_x += 14.0 * scale;
    }

    let first_line_h = item.lines.first().map(|l| l.line_h).unwrap_or(20.0);
    let first_px = item.lines.first().map(|l| l.px_size).unwrap_or(14.0);
    match p.list_kind {
        ListKind::None => {}
        ListKind::Bullet => {
            let dot_r = (2.8 * scale).clamp(2.2, 4.5);
            surface.fill_rounded_rect(
                Rect::new(
                    text_x + 4.0 * scale,
                    y + first_px * 0.56 - dot_r,
                    dot_r * 2.0,
                    dot_r * 2.0,
                ),
                dot_r,
                StudioTheme::sheet_ink_sub(),
            );
            text_x += 20.0 * scale;
        }
        ListKind::Numbered(n) => {
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                first_px,
                text_x,
                y + first_px * 0.88,
                StudioTheme::sheet_ink_sub(),
                &format!("{n}."),
            );
            text_x += 22.0 * scale;
        }
        ListKind::Checklist(done) => {
            let box_sz = (14.5 * scale).clamp(12.0, 21.0);
            let cb = Rect::new(text_x, y + (first_line_h - box_sz) * 0.36, box_sz, box_sz);
            if done {
                surface.fill_rounded_rect(cb, 3.0, StudioTheme::success());
                surface.draw_line(
                    cb.x + box_sz * 0.22,
                    cb.y + box_sz * 0.52,
                    cb.x + box_sz * 0.44,
                    cb.y + box_sz * 0.76,
                    1.8,
                    Color::rgb(255, 255, 255),
                );
                surface.draw_line(
                    cb.x + box_sz * 0.44,
                    cb.y + box_sz * 0.76,
                    cb.x + box_sz * 0.80,
                    cb.y + box_sz * 0.26,
                    1.8,
                    Color::rgb(255, 255, 255),
                );
            } else {
                surface.fill_rounded_rect(cb, 3.0, StudioTheme::sheet_paper());
                surface.stroke_rounded_rect(cb, 3.0, 1.4, StudioTheme::sheet_border());
            }
            hit_regions.push(HitRegion::button(
                Rect::new(cb.x - 3.0, cb.y - 3.0, cb.w + 6.0, cb.h + 6.0),
                UiAction::ToggleChecklistItem {
                    tab_idx,
                    block_idx: item.block_idx,
                },
            ));
            text_x += 23.0 * scale;
        }
    }

    let is_checked_done = matches!(p.list_kind, ListKind::Checklist(true));
    let avail_w = (content_w - (text_x - x)).max(40.0);
    let para_total_chars = p.plain_text().chars().count();
    let block_sel = if is_focused {
        w.block_selection_char_range(item.block_idx, para_total_chars)
    } else {
        None
    };
    let mut cur_y = y;

    for (line_idx, ln) in item.lines.iter().enumerate() {
        let lw = fonts.measure_text(ln.slot, &ln.text, ln.px_size);
        let eff_align = if p.alignment == Alignment::Left && p.is_rtl() {
            Alignment::Right
        } else {
            p.alignment
        };
        let draw_x = match eff_align {
            Alignment::Center => text_x + ((avail_w - lw) * 0.5).max(0.0),
            Alignment::Right => text_x + (avail_w - lw).max(0.0),
            Alignment::Left | Alignment::Justify => text_x,
        };
        let baseline_y = cur_y + ln.px_size * 0.90;
        let line_chars = ln.text.chars().count();
        let use_rich_spans = !ln.text.is_empty() && (p.spans.len() > 1 || p.has_revisions());

        let char_xs = if use_rich_spans {
            measure_rich_spans_char_xs(fonts, p, ln, draw_x, scale)
        } else {
            compute_line_char_xs(fonts, ln.slot, ln.px_size, draw_x, &ln.text)
        };

        // Paint silky Cobalt-Linen range selection pill behind selected glyphs!
        if let Some((sel_s, sel_e)) = block_sel {
            let line_end = ln.char_start + line_chars;
            if sel_e >= ln.char_start && sel_s <= line_end {
                let rel_s = sel_s.saturating_sub(ln.char_start).min(line_chars);
                let rel_e = sel_e.saturating_sub(ln.char_start).min(line_chars);
                let sx0 = char_xs.get(rel_s).copied().unwrap_or(draw_x);
                let mut sx1 = char_xs.get(rel_e).copied().unwrap_or(sx0);
                if sel_s < sel_e && rel_s == rel_e && line_chars == 0 {
                    sx1 = sx0 + 6.0 * scale;
                }
                if sx1 > sx0 + 0.5 {
                    let sel_rect =
                        Rect::new(sx0 - 1.0, cur_y + 0.5, (sx1 - sx0) + 2.0, ln.line_h - 1.0);
                    surface.fill_rounded_rect(sel_rect, 2.5, Color::rgba(35, 92, 184, 44));
                    if sel_bar_anchor.is_none() {
                        *sel_bar_anchor = Some(sel_rect);
                    }
                }
            }
        }

        if use_rich_spans {
            paint_rich_spans_line(
                surface,
                fonts,
                p,
                ln,
                draw_x,
                cur_y,
                baseline_y,
                scale,
                is_checked_done,
            );
        } else {
            if let Some(hl_col) = ln.highlight {
                if lw > 0.5 {
                    surface.fill_rounded_rect(
                        Rect::new(draw_x - 2.0, cur_y + 1.0, lw + 4.0, ln.line_h - 2.0),
                        2.5,
                        hl_col,
                    );
                }
            }

            let eff_color = if is_checked_done {
                StudioTheme::sheet_ink_muted()
            } else {
                ln.color
            };

            if !ln.text.is_empty() {
                surface.draw_text(
                    fonts,
                    ln.slot,
                    ln.px_size,
                    draw_x,
                    baseline_y,
                    eff_color,
                    &ln.text,
                );
            } else if item.block_idx == 0 && w.doc.blocks.len() == 1 {
                surface.draw_text(
                    fonts,
                    FontSlot::SansRegular,
                    ln.px_size,
                    draw_x,
                    baseline_y,
                    StudioTheme::sheet_ink_muted(),
                    "Start writing on this A4 sheet, double-click/drag to style words, or press Ctrl+J for AI…",
                );
            }

            if ln.underline && lw > 0.5 {
                surface.fill_rect(
                    Rect::new(draw_x, baseline_y + 2.0, lw, (1.1 * scale).max(1.0)),
                    eff_color,
                );
            }
            if (ln.strike || is_checked_done) && lw > 0.5 {
                surface.fill_rect(
                    Rect::new(
                        draw_x,
                        baseline_y - ln.px_size * 0.30,
                        lw,
                        (1.1 * scale).max(1.0),
                    ),
                    eff_color,
                );
            }
        }

        if w.spellcheck && !ln.text.is_empty() {
            for (c0, c1, _word) in spellchecker.check_paragraph(&ln.text) {
                let wx = char_xs.get(c0).copied().unwrap_or(draw_x);
                let wx1 = char_xs.get(c1).copied().unwrap_or(wx);
                surface.fill_rect(
                    Rect::new(wx, baseline_y + 2.5, (wx1 - wx).max(2.0), 1.5),
                    StudioTheme::pdf_accent(),
                );
            }
        }

        // Blinking Caret (using exact `char_xs` so mixed bold/font spans align to the sub-pixel!)
        let is_last_line = line_idx + 1 == item.lines.len();
        let cursor_in_block =
            is_focused && w.active_table_cell.is_none() && w.cursor_block == item.block_idx;
        if cursor_in_block && cursor_blink_on && !w.has_selection() {
            let in_this_line = if is_last_line {
                w.cursor_char >= ln.char_start && w.cursor_char <= ln.char_start + line_chars
            } else {
                w.cursor_char >= ln.char_start && w.cursor_char < ln.char_start + line_chars
            };
            if in_this_line {
                let offset = w.cursor_char.saturating_sub(ln.char_start).min(line_chars);
                let cx = char_xs.get(offset).copied().unwrap_or(draw_x);
                surface.fill_rect(
                    Rect::new(cx, cur_y + 1.0, 2.0, ln.line_h - 2.0),
                    StudioTheme::word_accent(),
                );
            }
        }

        // Character-accurate hit region + cached `WordLineHitBox` for 60-120Hz zero-lag drag selection!
        let line_hit = Rect::new(x, cur_y, content_w, ln.line_h.max(16.0));
        let line_box = WordLineHitBox {
            tab_idx,
            block_idx: item.block_idx,
            rect: line_hit,
            start_char: ln.char_start,
            char_xs,
        };
        let click_char = if line_hit.contains(mouse_x, mouse_y) {
            line_box.hit_test_char(mouse_x)
        } else {
            ln.char_start + line_chars
        };
        word_line_boxes.push(line_box);
        hit_regions.push(HitRegion::text(
            line_hit,
            UiAction::ClickWordParagraph {
                tab_idx,
                block_idx: item.block_idx,
                char_idx: click_char,
            },
        ));

        // Calm Empty-Paragraph Affordance: ghost hint + 1-click inline `+ Image` & `+ Table` launchers!
        if cursor_in_block
            && item.lines.len() == 1
            && ln.text.is_empty()
            && p.list_kind == ListKind::None
            && !w.has_selection()
            && content_w >= 260.0
        {
            let hint_px = (10.6 * scale).clamp(9.5, 13.0);
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                hint_px,
                draw_x + 8.0,
                baseline_y,
                (content_w - 168.0).max(80.0),
                StudioTheme::ink_muted(),
                "Type prose, /table 4x3, /image, or | Col A | Col B |…",
            );

            let chip_px = (10.0 * scale).clamp(9.2, 12.2);
            let chip_h = (ln.line_h - 3.0).clamp(18.0, 24.0);
            let chip_y = cur_y + (ln.line_h - chip_h) * 0.5;

            let tbl_lbl = "+ Table";
            let tbl_tw = fonts.measure_text(FontSlot::SansBold, tbl_lbl, chip_px);
            let tbl_w = (tbl_tw + 15.0 * scale).round();
            let tbl_r = Rect::new(x + content_w - tbl_w, chip_y, tbl_w, chip_h);

            let img_lbl = "+ Image";
            let img_tw = fonts.measure_text(FontSlot::SansBold, img_lbl, chip_px);
            let img_w = (img_tw + 15.0 * scale).round();
            let img_r = Rect::new(tbl_r.x - img_w - 5.0 * scale, chip_y, img_w, chip_h);

            for (cr, clbl, ctw, action) in [
                (
                    img_r,
                    img_lbl,
                    img_tw,
                    UiAction::ExecCommand(CommandId::InsertImage),
                ),
                (
                    tbl_r,
                    tbl_lbl,
                    tbl_tw,
                    UiAction::ToggleDropdown(ToolbarDropdown::TableCreator {
                        anchor_x: (tbl_r.x - 170.0 * scale).max(16.0) as i32,
                        anchor_y: (tbl_r.y + chip_h + 4.0) as i32,
                        rows: 3,
                        cols: 3,
                    }),
                ),
            ] {
                if cr.x <= draw_x + 70.0 {
                    continue;
                }
                let chov = cr.contains(mouse_x, mouse_y);
                surface.fill_rounded_rect(
                    cr,
                    4.0,
                    if chov {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::sidebar()
                    },
                );
                surface.stroke_rounded_rect(
                    cr,
                    4.0,
                    1.0,
                    if chov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::border()
                    },
                );
                surface.draw_text(
                    fonts,
                    FontSlot::SansBold,
                    chip_px,
                    cr.x + (cr.w - ctw) * 0.5,
                    cr.y + chip_h * 0.72,
                    if chov {
                        StudioTheme::word_accent()
                    } else {
                        StudioTheme::ink_sub()
                    },
                    clbl,
                );
                hit_regions.push(HitRegion::button(cr, action));
            }
        }

        cur_y += ln.line_h;
    }
}

fn span_slot_and_px(sp: &doc_model::TextSpan, p: &Paragraph, scale: f32) -> (FontSlot, f32) {
    let slot = match (
        sp.style.font_family,
        sp.style.bold || p.style.is_bold_by_default(),
    ) {
        (FontFamily::Mono, _) => FontSlot::MonoRegular,
        (FontFamily::Serif, _) => FontSlot::SerifRegular,
        (FontFamily::Sans, true) => FontSlot::SansBold,
        (FontFamily::Sans, false) => FontSlot::SansRegular,
    };
    let px = (sp.style.font_size_pt * scale).clamp(9.5, 58.0);
    (slot, px)
}

fn measure_rich_spans_char_xs(
    fonts: &mut FontBook,
    p: &Paragraph,
    ln: &LaidOutLine,
    draw_x: f32,
    scale: f32,
) -> Vec<f32> {
    let line_len = ln.text.chars().count();
    let line_end = ln.char_start + line_len;
    let mut xs = Vec::with_capacity(line_len + 1);
    let mut pen = draw_x;
    xs.push(pen);
    let mut span_cursor = 0usize;

    for sp in &p.spans {
        let sp_len = sp.text.chars().count();
        let sp_start = span_cursor;
        let sp_end = span_cursor + sp_len;
        span_cursor = sp_end;
        if sp_end <= ln.char_start || sp_start >= line_end {
            continue;
        }
        let ov_start = sp_start.max(ln.char_start) - sp_start;
        let ov_end = sp_end.min(line_end) - sp_start;
        let (slot, px) = span_slot_and_px(sp, p, scale);
        for ch in sp.text.chars().skip(ov_start).take(ov_end - ov_start) {
            pen += fonts.glyph(slot, ch, px).advance_px;
            xs.push(pen);
        }
    }
    xs
}

#[allow(clippy::too_many_arguments)]
fn paint_rich_spans_line(
    surface: &mut Surface,
    fonts: &mut FontBook,
    p: &Paragraph,
    ln: &LaidOutLine,
    draw_x: f32,
    cur_y: f32,
    baseline_y: f32,
    scale: f32,
    is_checked_done: bool,
) {
    let line_len = ln.text.chars().count();
    let line_end = ln.char_start + line_len;
    let mut span_cursor = 0usize;
    let mut seg_x = draw_x;

    for sp in &p.spans {
        let sp_len = sp.text.chars().count();
        let sp_start = span_cursor;
        let sp_end = span_cursor + sp_len;
        span_cursor = sp_end;

        if sp_end <= ln.char_start || sp_start >= line_end {
            continue;
        }
        let ov_start = sp_start.max(ln.char_start) - sp_start;
        let ov_end = sp_end.min(line_end) - sp_start;
        if ov_end <= ov_start {
            continue;
        }
        let slice_str: String = sp
            .text
            .chars()
            .skip(ov_start)
            .take(ov_end - ov_start)
            .collect();
        if slice_str.is_empty() {
            continue;
        }

        let (sp_slot, sp_px) = span_slot_and_px(sp, p, scale);
        let sw = fonts.measure_text(sp_slot, &slice_str, sp_px);
        let rev_kind = sp.revision.as_ref().map(|r| r.kind);

        match rev_kind {
            Some(RevisionKind::Insertion) => {
                surface.fill_rounded_rect(
                    Rect::new(seg_x - 1.0, cur_y + 1.5, sw + 2.0, ln.line_h - 3.0),
                    2.0,
                    Color::rgba(26, 127, 75, 28),
                );
                let ins_col = Color::rgb(21, 114, 67);
                surface.draw_text(
                    fonts,
                    sp_slot,
                    sp_px,
                    seg_x,
                    baseline_y,
                    ins_col,
                    &slice_str,
                );
                surface.fill_rect(
                    Rect::new(seg_x, baseline_y + 2.0, sw, (1.3 * scale).max(1.1)),
                    ins_col,
                );
            }
            Some(RevisionKind::Deletion) => {
                surface.fill_rounded_rect(
                    Rect::new(seg_x - 1.0, cur_y + 1.5, sw + 2.0, ln.line_h - 3.0),
                    2.0,
                    Color::rgba(185, 48, 42, 24),
                );
                let del_col = Color::rgb(178, 42, 38);
                surface.draw_text(
                    fonts,
                    sp_slot,
                    sp_px,
                    seg_x,
                    baseline_y,
                    del_col,
                    &slice_str,
                );
                surface.fill_rect(
                    Rect::new(
                        seg_x,
                        baseline_y - sp_px * 0.30,
                        sw,
                        (1.3 * scale).max(1.1),
                    ),
                    del_col,
                );
            }
            None => {
                if sp.style.hidden && !p.has_revisions() {
                    // Render subtle dotted inspector underline when hidden text is present
                    surface.fill_rounded_rect(
                        Rect::new(seg_x - 1.0, cur_y + 2.0, sw + 2.0, ln.line_h - 4.0),
                        2.0,
                        Color::rgba(148, 163, 184, 34),
                    );
                } else if let Some(hl) = sp.style.highlight {
                    surface.fill_rounded_rect(
                        Rect::new(seg_x - 1.5, cur_y + 1.0, sw + 3.0, ln.line_h - 2.0),
                        2.5,
                        Color::rgb(hl.r, hl.g, hl.b),
                    );
                }
                let col = if is_checked_done || sp.style.hidden {
                    StudioTheme::sheet_ink_muted()
                } else {
                    StudioTheme::adapt_doc_ink(sp.style.color, sp.style.highlight.is_some())
                };
                let adj_y = if sp.style.superscript {
                    baseline_y - sp_px * 0.24
                } else if sp.style.subscript {
                    baseline_y + sp_px * 0.16
                } else {
                    baseline_y
                };
                let adj_px = if sp.style.superscript || sp.style.subscript {
                    (sp_px * 0.78).max(8.0)
                } else {
                    sp_px
                };
                surface.draw_text(fonts, sp_slot, adj_px, seg_x, adj_y, col, &slice_str);
                if sp.style.underline || sp.style.hyperlink.is_some() {
                    surface.fill_rect(
                        Rect::new(seg_x, baseline_y + 2.0, sw, (1.1 * scale).max(1.0)),
                        col,
                    );
                }
                if sp.style.strikethrough || is_checked_done {
                    surface.fill_rect(
                        Rect::new(
                            seg_x,
                            baseline_y - sp_px * 0.30,
                            sw,
                            (1.1 * scale).max(1.0),
                        ),
                        col,
                    );
                }
            }
        }
        seg_x += sw;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_word_ribbon(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    sidebar_open: bool,
    bar: Rect,
    w: &WordTabState,
) {
    let cur_para = w.doc.blocks.get(w.cursor_block).and_then(|b| match b {
        Block::Paragraph(p) => Some(p),
        _ => None,
    });
    let active_sp_style = cur_para
        .map(|p| {
            let probe = w
                .ordered_selection()
                .map(|((_, c0), _)| c0 + 1)
                .unwrap_or(w.cursor_char);
            p.style_at_char(probe)
        })
        .unwrap_or_default();
    let is_bold = active_sp_style.bold;
    let is_italic = active_sp_style.italic;
    let is_under = active_sp_style.underline;
    let has_hl = active_sp_style.highlight.is_some();
    let cur_pt = active_sp_style.font_size_pt;
    let style_lbl = cur_para.map(|p| p.style.label()).unwrap_or("Body");
    let fam_lbl = match active_sp_style.font_family {
        FontFamily::Sans => "Sans",
        FontFamily::Serif => "Serif",
        FontFamily::Mono => "Mono",
    };
    let cur_align = cur_para.map(|p| p.alignment).unwrap_or(Alignment::Left);
    let is_bullet = cur_para
        .map(|p| p.list_kind == ListKind::Bullet)
        .unwrap_or(false);
    let is_check = cur_para
        .map(|p| matches!(p.list_kind, ListKind::Checklist(_)))
        .unwrap_or(false);

    let bh = (30.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut bx = bar.x + 10.0 * s;
    let max_x = bar.x + bar.w - 10.0 * s;

    // Helper closures via compact functions
    // 1. Sidebar Toggle + Undo / Redo
    bx = draw_icon_btn(
        surface,
        hit_regions,
        mouse_x,
        mouse_y,
        bx,
        by,
        bh,
        s,
        UiIcon::Sidebar,
        sidebar_open,
        UiAction::ExecCommand(CommandId::ToggleLeftSidebar),
    );
    bx = draw_divider(surface, bx, bar.y, bar.h, s);

    bx = draw_icon_btn(
        surface,
        hit_regions,
        mouse_x,
        mouse_y,
        bx,
        by,
        bh,
        s,
        UiIcon::Undo,
        false,
        UiAction::ExecCommand(CommandId::Undo),
    );
    bx = draw_icon_btn(
        surface,
        hit_regions,
        mouse_x,
        mouse_y,
        bx,
        by,
        bh,
        s,
        UiIcon::Redo,
        false,
        UiAction::ExecCommand(CommandId::Redo),
    );
    bx = draw_divider(surface, bx, bar.y, bar.h, s);

    // 2. Paragraph Style Dropdown + Font Family Dropdown + Font Size Stepper
    let style_w = (96.0 * s).round();
    if bx + style_w < max_x {
        let r = Rect::new(bx, by, style_w, bh);
        let hov = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(r, 5.0, if hov { StudioTheme::hover() } else { StudioTheme::paper() });
        surface.stroke_rounded_rect(r, 5.0, 1.0, StudioTheme::border());
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansBold,
            11.8 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.66,
            style_w - 22.0 * s,
            StudioTheme::ink(),
            style_lbl,
        );
        surface.draw_icon(
            UiIcon::ChevronDown,
            r.x + r.w - 11.0 * s,
            r.y + bh * 0.52,
            9.0 * s,
            StudioTheme::ink_muted(),
        );
        hit_regions.push(HitRegion::button(
            r,
            UiAction::ToggleDropdown(ToolbarDropdown::ParagraphStyle {
                anchor_x: r.x as i32,
                anchor_y: (bar.y + bar.h) as i32,
            }),
        ));
        bx += style_w + 5.0 * s;
    }

    let font_w = (68.0 * s).round();
    if bx + font_w < max_x {
        let r = Rect::new(bx, by, font_w, bh);
        let hov = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(r, 5.0, if hov { StudioTheme::hover() } else { StudioTheme::paper() });
        surface.stroke_rounded_rect(r, 5.0, 1.0, StudioTheme::border());
        surface.draw_text(
            fonts,
            FontSlot::SansRegular,
            11.8 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.66,
            StudioTheme::ink(),
            fam_lbl,
        );
        surface.draw_icon(
            UiIcon::ChevronDown,
            r.x + r.w - 11.0 * s,
            r.y + bh * 0.52,
            9.0 * s,
            StudioTheme::ink_muted(),
        );
        hit_regions.push(HitRegion::button(
            r,
            UiAction::ToggleDropdown(ToolbarDropdown::FontFamily {
                anchor_x: r.x as i32,
                anchor_y: (bar.y + bar.h) as i32,
            }),
        ));
        bx += font_w + 5.0 * s;
    }

    // Joined [- 12.5 +] size control
    let step_w = (82.0 * s).round();
    if bx + step_w < max_x {
        let r = Rect::new(bx, by, step_w, bh);
        surface.fill_rounded_rect(r, 5.0, StudioTheme::paper());
        surface.stroke_rounded_rect(r, 5.0, 1.0, StudioTheme::border());
        let btn_seg = (22.0 * s).round();
        let down_r = Rect::new(r.x, r.y, btn_seg, bh);
        let up_r = Rect::new(r.x + r.w - btn_seg, r.y, btn_seg, bh);
        surface.draw_icon(UiIcon::Minus, down_r.x + btn_seg * 0.5, r.y + bh * 0.5, 9.0 * s, StudioTheme::ink_sub());
        surface.draw_icon(UiIcon::Plus, up_r.x + btn_seg * 0.5, r.y + bh * 0.5, 9.0 * s, StudioTheme::ink_sub());
        let sz_txt = if (cur_pt.fract()).abs() < 0.1 {
            format!("{:.0}", cur_pt)
        } else {
            format!("{:.1}", cur_pt)
        };
        let tw = fonts.measure_text(FontSlot::MonoRegular, &sz_txt, 11.2 * s);
        surface.draw_text(
            fonts,
            FontSlot::MonoRegular,
            11.2 * s,
            r.x + (r.w - tw) * 0.5,
            r.y + bh * 0.66,
            StudioTheme::ink(),
            &sz_txt,
        );
        hit_regions.push(HitRegion::button(down_r, UiAction::ExecCommand(CommandId::FontSizeDown)));
        hit_regions.push(HitRegion::button(up_r, UiAction::ExecCommand(CommandId::FontSizeUp)));
        bx += step_w + 4.0 * s;
    }

    bx = draw_divider(surface, bx, bar.y, bar.h, s);

    // 3. Inline Marks (`B`, `I`, `U`, `Ink`, `Mark`)
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "B", FontSlot::SansBold, is_bold, false, CommandId::ToggleBold);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "I", FontSlot::SerifRegular, is_italic, false, CommandId::ToggleItalic);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "U", FontSlot::SansRegular, is_under, false, CommandId::ToggleUnderline);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "Ink", FontSlot::SansRegular, false, false, CommandId::CycleTextColor);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "Mark", FontSlot::SansRegular, has_hl, false, CommandId::ToggleHighlight);

    bx = draw_divider(surface, bx, bar.y, bar.h, s);

    // 4. Alignment & Lists (Crisp Vector Icons!)
    for (ic, active, cmd) in [
        (UiIcon::AlignLeft, cur_align == Alignment::Left, CommandId::AlignLeft),
        (UiIcon::AlignCenter, cur_align == Alignment::Center, CommandId::AlignCenter),
        (UiIcon::AlignRight, cur_align == Alignment::Right, CommandId::AlignRight),
        (UiIcon::BulletList, is_bullet, CommandId::ToggleBulletList),
        (UiIcon::Checklist, is_check, CommandId::ToggleChecklist),
    ] {
        if bx + bh > max_x {
            break;
        }
        bx = draw_icon_btn(
            surface,
            hit_regions,
            mouse_x,
            mouse_y,
            bx,
            by,
            bh,
            s,
            ic,
            active,
            UiAction::ExecCommand(cmd),
        );
    }

    bx = draw_divider(surface, bx, bar.y, bar.h, s);

    // 5. Interactive Table Creator (`Table ▾`) + Page Tools + Export PDF
    {
        let lbl = "Table";
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 11.5 * s);
        let bw = (tw + 40.0 * s).round();
        if bx + bw <= max_x {
            let r = Rect::new(bx, by, bw, bh);
            let hov = r.contains(mouse_x, mouse_y);
            let active = w.active_table_cell.is_some();
            if active || hov {
                surface.fill_rounded_rect(
                    r,
                    5.0,
                    if active {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::hover()
                    },
                );
            }
            if active {
                surface.stroke_rounded_rect(r, 5.0, 1.0, StudioTheme::word_accent());
            }
            let fg = if active {
                StudioTheme::word_accent()
            } else {
                StudioTheme::ink()
            };
            surface.draw_icon(UiIcon::Table, r.x + 11.0 * s, r.y + bh * 0.5, 13.0 * s, fg);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                11.5 * s,
                r.x + 21.0 * s,
                r.y + bh * 0.67,
                fg,
                lbl,
            );
            surface.draw_icon(
                UiIcon::ChevronDown,
                r.x + bw - 9.5 * s,
                r.y + bh * 0.52,
                8.5 * s,
                StudioTheme::ink_sub(),
            );
            hit_regions.push(HitRegion::button(
                r,
                UiAction::ToggleDropdown(ToolbarDropdown::TableCreator {
                    anchor_x: r.x as i32,
                    anchor_y: (r.y + bh) as i32,
                    rows: 3,
                    cols: 3,
                }),
            ));
            bx += bw + 4.0 * s;
        }
    }
    // 1-Click `Image` Ribbon Button (`Ctrl+Shift+I`)
    {
        let lbl = "Image";
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 11.5 * s);
        let bw = (tw + 30.0 * s).round();
        if bx + bw <= max_x {
            let r = Rect::new(bx, by, bw, bh);
            let hov = r.contains(mouse_x, mouse_y);
            let active = w.selected_image_block.is_some();
            if active || hov {
                surface.fill_rounded_rect(
                    r,
                    5.0,
                    if active {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::hover()
                    },
                );
            }
            if active {
                surface.stroke_rounded_rect(r, 5.0, 1.0, StudioTheme::word_accent());
            }
            let fg = if active {
                StudioTheme::word_accent()
            } else {
                StudioTheme::ink()
            };
            surface.draw_icon(UiIcon::Image, r.x + 11.0 * s, r.y + bh * 0.5, 13.0 * s, fg);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                11.5 * s,
                r.x + 21.0 * s,
                r.y + bh * 0.67,
                fg,
                lbl,
            );
            hit_regions.push(HitRegion::button(
                r,
                UiAction::ExecCommand(CommandId::InsertImage),
            ));
            bx += bw + 4.0 * s;
        }
    }
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "Break", FontSlot::SansRegular, false, false, CommandId::InsertPageBreak);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "Track", FontSlot::SansBold, w.doc.track_changes, false, CommandId::ToggleTrackChanges);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "+Comment", FontSlot::SansRegular, !w.doc.comments.is_empty(), false, CommandId::AddReviewComment);
    bx = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "Spell", FontSlot::SansRegular, w.spellcheck, false, CommandId::ToggleSpellcheck);
    let _ = draw_label_btn(surface, fonts, hit_regions, mouse_x, mouse_y, bx, by, bh, s, max_x, "Export PDF", FontSlot::SansBold, false, true, CommandId::ExportWordToPdf);
}

#[allow(clippy::too_many_arguments)]
fn paint_contextual_review_bar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    bar: Rect,
    w: &WordTabState,
    rev_ins: usize,
    rev_del: usize,
) {
    surface.fill_rect(bar, Color::rgb(245, 242, 234));
    surface.fill_rect(
        Rect::new(bar.x, bar.y + bar.h - 1.0, bar.w, 1.0),
        StudioTheme::border(),
    );

    let summary = format!(
        "LEGAL REDLINE · +{} ins / -{} del · {} comment{}",
        rev_ins,
        rev_del,
        w.doc.comments.len(),
        if w.doc.comments.len() == 1 { "" } else { "s" }
    );
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.8 * s,
        bar.x + 14.0 * s,
        bar.y + bar.h * 0.64,
        StudioTheme::pdf_accent(),
        &summary,
    );

    let summary_w = fonts.measure_text(FontSlot::SansBold, &summary, 10.8 * s);
    let bh = (25.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut bx = bar.x + summary_w + 22.0 * s;
    let max_x = bar.x + bar.w - 10.0 * s;

    // 1-Click Per-Clause Accept / Reject when cursor or paragraph sits on a tracked revision!
    let cur_clause = w
        .doc
        .blocks
        .get(w.cursor_block)
        .and_then(|b| match b {
            Block::Paragraph(p) => p.revision_at_char(w.cursor_char),
            _ => None,
        });
    if let Some(rev) = cur_clause {
        let acc_lbl = format!("Accept #{}", rev.id);
        let rej_lbl = format!("Reject #{}", rev.id);
        for (lbl, is_accept, action) in [
            (
                acc_lbl.as_str(),
                true,
                UiAction::AcceptRevisionById {
                    tab_idx,
                    rev_id: rev.id,
                },
            ),
            (
                rej_lbl.as_str(),
                false,
                UiAction::RejectRevisionById {
                    tab_idx,
                    rev_id: rev.id,
                },
            ),
        ] {
            let tw = fonts.measure_text(FontSlot::SansBold, lbl, 10.8 * s);
            let bw = (tw + 16.0 * s).round();
            if bx + bw <= max_x {
                let r = Rect::new(bx, by, bw, bh);
                let hov = r.contains(mouse_x, mouse_y);
                let (bg, border, fg) = if is_accept {
                    (
                        if hov {
                            Color::rgba(26, 127, 75, 42)
                        } else {
                            Color::rgba(26, 127, 75, 22)
                        },
                        StudioTheme::success(),
                        StudioTheme::success(),
                    )
                } else {
                    (
                        if hov {
                            StudioTheme::pdf_soft()
                        } else {
                            Color::rgba(185, 48, 42, 18)
                        },
                        StudioTheme::pdf_accent(),
                        StudioTheme::pdf_accent(),
                    )
                };
                surface.fill_rounded_rect(r, 4.0, bg);
                surface.stroke_rounded_rect(r, 4.0, 1.0, border);
                surface.draw_text(
                    fonts,
                    FontSlot::SansBold,
                    10.8 * s,
                    r.x + 8.0 * s,
                    r.y + bh * 0.68,
                    fg,
                    lbl,
                );
                hit_regions.push(HitRegion::button(r, action));
                bx += bw + 5.0 * s;
            }
        }
    }

    let track_lbl = if w.doc.track_changes {
        "Track: ON"
    } else {
        "Track: OFF"
    };
    let cls_lbl = format!("Class: {}", w.doc.classification.label());
    let actions: [(&str, CommandId, bool); 5] = [
        (track_lbl, CommandId::ToggleTrackChanges, w.doc.track_changes),
        ("Accept All", CommandId::AcceptAllRevisions, false),
        ("Reject All", CommandId::RejectAllRevisions, false),
        ("+ Comment", CommandId::AddReviewComment, false),
        (&cls_lbl, CommandId::CycleClassification, false),
    ];

    for (lbl, cmd, active) in actions {
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 10.8 * s);
        let bw = (tw + 16.0 * s).round();
        if bx + bw > max_x {
            break;
        }
        let r = Rect::new(bx, by, bw, bh);
        let hov = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            r,
            4.0,
            if active {
                StudioTheme::word_soft()
            } else if hov {
                StudioTheme::hover()
            } else {
                StudioTheme::paper()
            },
        );
        surface.stroke_rounded_rect(
            r,
            4.0,
            1.0,
            if active {
                StudioTheme::word_accent()
            } else {
                StudioTheme::border_strong()
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.8 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.68,
            if active {
                StudioTheme::word_accent()
            } else {
                StudioTheme::ink()
            },
            lbl,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
        bx += bw + 5.0 * s;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_contextual_image_bar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    bar: Rect,
    w: &WordTabState,
) {
    surface.fill_rect(bar, StudioTheme::word_soft());
    surface.fill_rect(
        Rect::new(bar.x, bar.y + bar.h - 1.0, bar.w, 1.0),
        StudioTheme::border(),
    );

    let (img_info, cur_align) = w
        .selected_image_block
        .and_then(|b| match w.doc.blocks.get(b) {
            Some(Block::Image(img)) => Some((
                format!("IMAGE · {:.0}×{:.0} pt", img.width_pt, img.height_pt),
                img.alignment,
            )),
            _ => None,
        })
        .unwrap_or_else(|| ("IMAGE".into(), Alignment::Center));

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.8 * s,
        bar.x + 12.0 * s,
        bar.y + bar.h * 0.64,
        StudioTheme::word_accent(),
        &img_info,
    );

    let info_w = fonts.measure_text(FontSlot::SansBold, &img_info, 10.8 * s);
    let bh = (25.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut bx = bar.x + info_w + 22.0 * s;
    let max_x = bar.x + bar.w - 10.0 * s;

    let actions: &[(&str, CommandId, bool, bool)] = &[
        ("25%", CommandId::ImageScale25, false, false),
        ("50%", CommandId::ImageScale50, false, false),
        ("75%", CommandId::ImageScale75, false, false),
        ("100%", CommandId::ImageScale100, false, false),
        ("Left", CommandId::AlignLeft, cur_align == Alignment::Left, false),
        ("Center", CommandId::AlignCenter, cur_align == Alignment::Center, false),
        ("Right", CommandId::AlignRight, cur_align == Alignment::Right, false),
        ("Caption", CommandId::ImageEditCaption, false, false),
        ("Replace", CommandId::ImageReplace, false, false),
        ("Delete", CommandId::ImageDelete, false, true),
    ];

    for (lbl, cmd, active, danger) in actions {
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 11.0 * s);
        let bw = (tw + 16.0 * s).round();
        if bx + bw > max_x {
            break;
        }
        let r = Rect::new(bx, by, bw, bh);
        let hov = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            r,
            4.0,
            if *danger && hov {
                StudioTheme::pdf_soft()
            } else if *active || hov {
                StudioTheme::paper()
            } else {
                Color::rgba(255, 255, 255, 210)
            },
        );
        surface.stroke_rounded_rect(
            r,
            4.0,
            1.0,
            if *danger && hov {
                StudioTheme::pdf_accent()
            } else if *active {
                StudioTheme::word_accent()
            } else {
                StudioTheme::border_strong()
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.0 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.68,
            if *danger {
                StudioTheme::pdf_accent()
            } else if *active {
                StudioTheme::word_accent()
            } else {
                StudioTheme::ink()
            },
            lbl,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(*cmd)));
        bx += bw + 5.0 * s;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_contextual_table_bar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    bar: Rect,
    w: &WordTabState,
) {
    surface.fill_rect(bar, StudioTheme::rust_soft());
    surface.fill_rect(
        Rect::new(bar.x, bar.y + bar.h - 1.0, bar.w, 1.0),
        StudioTheme::border(),
    );

    let cell_info = w
        .active_table_cell
        .map(|(_, r, c)| format!("TABLE R{}·C{}", r + 1, c + 1))
        .unwrap_or_else(|| "TABLE".into());
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.8 * s,
        bar.x + 12.0 * s,
        bar.y + bar.h * 0.64,
        StudioTheme::rust_orange(),
        &cell_info,
    );

    let bh = (25.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut bx = bar.x + 98.0 * s;
    let max_x = bar.x + bar.w - 10.0 * s;

    let actions: &[(&str, CommandId, bool)] = &[
        ("+ Row", CommandId::TableAddRow, false),
        ("+ Row ↑", CommandId::TableInsertRowAbove, false),
        ("- Row", CommandId::TableDeleteRow, false),
        ("+ Col", CommandId::TableAddCol, false),
        ("- Col", CommandId::TableDeleteCol, false),
        ("Bold", CommandId::TableToggleCellBold, false),
        ("Shade", CommandId::TableCycleCellFill, false),
        ("Zebra", CommandId::TableToggleZebra, false),
        ("Theme", CommandId::TableCycleTheme, false),
        ("Equal", CommandId::TableEqualizeCols, false),
        ("Delete", CommandId::TableDelete, true),
    ];

    for (lbl, cmd, danger) in actions {
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 11.0 * s);
        let bw = (tw + 16.0 * s).round();
        if bx + bw > max_x {
            break;
        }
        let r = Rect::new(bx, by, bw, bh);
        let hov = r.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            r,
            4.0,
            if hov {
                if *danger {
                    StudioTheme::pdf_soft()
                } else {
                    StudioTheme::paper()
                }
            } else {
                Color::rgba(255, 255, 255, 210)
            },
        );
        surface.stroke_rounded_rect(
            r,
            4.0,
            1.0,
            if *danger && hov {
                StudioTheme::pdf_accent()
            } else {
                StudioTheme::border_strong()
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.0 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.68,
            if *danger {
                StudioTheme::pdf_accent()
            } else {
                StudioTheme::ink()
            },
            lbl,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(*cmd)));
        bx += bw + 5.0 * s;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_icon_btn(
    surface: &mut Surface,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    bx: f32,
    by: f32,
    bh: f32,
    s: f32,
    icon: UiIcon,
    active: bool,
    action: UiAction,
) -> f32 {
    let bw = (30.0 * s).round();
    let r = Rect::new(bx, by, bw, bh);
    let hov = r.contains(mouse_x, mouse_y);
    if active || hov {
        surface.fill_rounded_rect(
            r,
            5.0,
            if active {
                StudioTheme::word_soft()
            } else {
                StudioTheme::hover()
            },
        );
    }
    if active {
        surface.stroke_rounded_rect(r, 5.0, 1.0, StudioTheme::word_accent());
    }
    surface.draw_icon(
        icon,
        r.x + bw * 0.5,
        r.y + bh * 0.5,
        13.5 * s,
        if active {
            StudioTheme::word_accent()
        } else {
            StudioTheme::ink_sub()
        },
    );
    hit_regions.push(HitRegion::button(r, action));
    bx + bw + 3.0 * s
}

#[allow(clippy::too_many_arguments)]
fn draw_label_btn(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    bx: f32,
    by: f32,
    bh: f32,
    s: f32,
    max_x: f32,
    label: &str,
    slot: FontSlot,
    active: bool,
    pdf_accent: bool,
    cmd: CommandId,
) -> f32 {
    let tw = fonts.measure_text(slot, label, 12.0 * s);
    let bw = (tw + 15.0 * s).max(28.0 * s).round();
    if bx + bw > max_x {
        return bx;
    }
    let r = Rect::new(bx, by, bw, bh);
    let hov = r.contains(mouse_x, mouse_y);
    if pdf_accent || active || hov {
        surface.fill_rounded_rect(
            r,
            5.0,
            if pdf_accent {
                StudioTheme::pdf_soft()
            } else if active {
                StudioTheme::word_soft()
            } else {
                StudioTheme::hover()
            },
        );
    }
    if pdf_accent || active {
        surface.stroke_rounded_rect(
            r,
            5.0,
            1.0,
            if pdf_accent {
                StudioTheme::pdf_accent()
            } else {
                StudioTheme::word_accent()
            },
        );
    }
    let fg = if pdf_accent {
        StudioTheme::pdf_accent()
    } else if active {
        StudioTheme::word_accent()
    } else {
        StudioTheme::ink()
    };
    surface.draw_text(
        fonts,
        slot,
        12.0 * s,
        r.x + (bw - tw) * 0.5,
        r.y + bh * 0.67,
        fg,
        label,
    );
    hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
    bx + bw + 3.0 * s
}

fn draw_divider(surface: &mut Surface, bx: f32, bar_y: f32, bar_h: f32, s: f32) -> f32 {
    surface.fill_rect(
        Rect::new((bx + 3.0 * s).round(), bar_y + 10.0 * s, 1.0, bar_h - 20.0 * s),
        StudioTheme::border(),
    );
    bx + 8.0 * s
}

#[allow(clippy::too_many_arguments)]
fn paint_outline_sidebar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    sb: Rect,
    block_canvas_y: &[f32],
    w: &WordTabState,
) {
    surface.fill_rect(sb, StudioTheme::sidebar());
    surface.fill_rect(
        Rect::new(sb.x + sb.w - 1.0, sb.y, 1.0, sb.h),
        StudioTheme::border(),
    );

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.5 * s,
        sb.x + 14.0 * s,
        sb.y + 24.0 * s,
        StudioTheme::ink_muted(),
        "OUTLINE",
    );

    let outline = &w.cached_outline;
    let mut y = sb.y + 36.0 * s;
    let footer_h = 132.0 * s;

    if outline.is_empty() {
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansRegular,
            11.8 * s,
            sb.x + 14.0 * s,
            y + 16.0 * s,
            sb.w - 28.0 * s,
            StudioTheme::ink_muted(),
            "Apply Heading 1–3 to build a live outline.",
        );
    } else {
        for item in outline {
            let row_h = (28.0 * s).round();
            if y + row_h > sb.y + sb.h - footer_h {
                break;
            }
            let row = Rect::new(sb.x + 8.0 * s, y, sb.w - 16.0 * s, row_h);
            let active = w.cursor_block == item.block_index;
            let hov = row.contains(mouse_x, mouse_y);
            if active || hov {
                surface.fill_rounded_rect(
                    row,
                    4.0,
                    if active {
                        StudioTheme::word_soft()
                    } else {
                        StudioTheme::hover()
                    },
                );
            }
            if active {
                surface.fill_rect(
                    Rect::new(row.x, row.y + 5.0 * s, 2.5, row.h - 10.0 * s),
                    StudioTheme::word_accent(),
                );
            }
            let indent = (item.level as f32) * 9.0 * s;
            surface.draw_text_clipped(
                fonts,
                if item.level <= 1 {
                    FontSlot::SansBold
                } else {
                    FontSlot::SansRegular
                },
                12.0 * s,
                row.x + 10.0 * s + indent,
                row.y + row_h * 0.67,
                (row.w - 16.0 * s - indent).max(32.0),
                if active {
                    StudioTheme::word_accent()
                } else {
                    StudioTheme::ink_sub()
                },
                &item.title,
            );
            let target_y = block_canvas_y.get(item.block_index).copied().unwrap_or(0.0);
            hit_regions.push(HitRegion::button(
                row,
                UiAction::JumpOutline {
                    tab_idx,
                    block_idx: item.block_index,
                    target_scroll_y: target_y,
                },
            ));
            y += row_h + 2.0 * s;
        }
    }

    // Threaded Review Comments List (jump directly to commented clause!)
    if !w.doc.comments.is_empty() && y + 66.0 * s < sb.y + sb.h - footer_h {
        y += 10.0 * s;
        surface.fill_rect(
            Rect::new(sb.x + 14.0 * s, y, sb.w - 28.0 * s, 1.0),
            StudioTheme::border(),
        );
        y += 16.0 * s;
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.2 * s,
            sb.x + 14.0 * s,
            y,
            StudioTheme::ink_muted(),
            &format!("REVIEW COMMENTS ({})", w.doc.comments.len()),
        );
        y += 8.0 * s;

        for cmt in &w.doc.comments {
            let ch = (42.0 * s).round();
            if y + ch > sb.y + sb.h - footer_h {
                break;
            }
            let crow = Rect::new(sb.x + 8.0 * s, y, sb.w - 16.0 * s, ch);
            let hov = crow.contains(mouse_x, mouse_y);
            surface.fill_rounded_rect(
                crow,
                4.0,
                if hov {
                    StudioTheme::hover()
                } else {
                    StudioTheme::paper()
                },
            );
            surface.stroke_rounded_rect(crow, 4.0, 1.0, StudioTheme::border());
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansBold,
                10.6 * s,
                crow.x + 8.0 * s,
                crow.y + 15.0 * s,
                crow.w - 16.0 * s,
                StudioTheme::word_accent(),
                &format!("{} · ¶{}", cmt.author, cmt.block_index + 1),
            );
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                10.6 * s,
                crow.x + 8.0 * s,
                crow.y + 31.0 * s,
                crow.w - 16.0 * s,
                StudioTheme::ink_sub(),
                &cmt.text,
            );
            let target_y = block_canvas_y.get(cmt.block_index).copied().unwrap_or(0.0);
            hit_regions.push(HitRegion::button(
                crow,
                UiAction::JumpOutline {
                    tab_idx,
                    block_idx: cmt.block_index,
                    target_scroll_y: target_y,
                },
            ));
            y += ch + 4.0 * s;
        }
    }

    // Hairline-separated Document Metrics Ledger (zero floating AI-slop nested card!)
    let stats = w.cached_stats;
    let m_top = (sb.y + sb.h - footer_h + 8.0 * s).max(y + 12.0 * s);
    if m_top + 96.0 * s <= sb.y + sb.h {
        surface.fill_rect(
            Rect::new(sb.x + 14.0 * s, m_top, sb.w - 28.0 * s, 1.0),
            StudioTheme::border(),
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.2 * s,
            sb.x + 14.0 * s,
            m_top + 18.0 * s,
            StudioTheme::ink_muted(),
            "DOCUMENT METRICS",
        );
        let rows = [
            ("Words", format!("{}", stats.words)),
            ("Characters", format!("{}", stats.chars)),
            ("Reading", format!("~{} min", stats.reading_time_minutes)),
            ("Paper", w.doc.page_setup.paper_size.label().to_string()),
        ];
        for (i, (k, val)) in rows.iter().enumerate() {
            let ry = m_top + (36.0 + (i as f32) * 19.0) * s;
            surface.draw_text(
                fonts,
                FontSlot::SansRegular,
                11.4 * s,
                sb.x + 14.0 * s,
                ry,
                StudioTheme::ink_muted(),
                k,
            );
            let vw = fonts.measure_text(FontSlot::MonoRegular, val, 11.2 * s);
            surface.draw_text_clipped(
                fonts,
                FontSlot::MonoRegular,
                11.2 * s,
                (sb.x + sb.w - 14.0 * s - vw).max(sb.x + 76.0 * s),
                ry,
                sb.w - 86.0 * s,
                StudioTheme::ink(),
                val,
            );
        }
    }
}
