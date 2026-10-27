//! Word Studio (`.docx`) Paginated Canvas, Vector Formatting Ribbon, Contextual Table Bar,
//! Multi-Line Resizable Tables, Character-Accurate Hit-Testing & Resizable Outline Rail.

use crate::commands::CommandId;
use crate::state::{
    DragInteraction, HitRegion, ToolbarDropdown, UiAction, WordLineHitBox, WordTabState,
};
use doc_model::{
    Alignment, Block, FontFamily, ListKind, Paragraph, ParagraphStyle, RevisionKind,
};
use power_tools::SpellChecker;
use rustpad_render::{
    decode_png_rgba, Color, DaylightTheme, FontBook, FontSlot, Rect, Surface, UiIcon,
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
    let s = ui_scale.clamp(0.85, 1.6);
    let ribbon_h = if focus_mode { 0.0 } else { (44.0 * s).round() };
    let has_active_table = w.active_table_cell.is_some();
    let (rev_ins, rev_del) = w.doc.revision_counts();
    let show_review_bar = !focus_mode
        && is_focused
        && !has_active_table
        && (w.doc.track_changes || rev_ins > 0 || rev_del > 0 || !w.doc.comments.is_empty());
    let table_bar_h = if !focus_mode && ((has_active_table && is_focused) || show_review_bar) {
        (36.0 * s).round()
    } else {
        0.0
    };

    // 1. Primary Formatting Ribbon (hidden in Distraction-Free Focus Mode)
    if ribbon_h > 0.0 {
        let ribbon_rect = Rect::new(area.x, area.y, area.w, ribbon_h);
        surface.fill_rect(ribbon_rect, DaylightTheme::CHROME);
        surface.fill_rect(
            Rect::new(
                ribbon_rect.x,
                ribbon_rect.y + ribbon_rect.h - 1.0,
                ribbon_rect.w,
                1.0,
            ),
            DaylightTheme::BORDER,
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
                DaylightTheme::WORD_ACCENT,
                &lbl,
            );
            hit_regions.push(HitRegion::button(ribbon_rect, UiAction::SwitchTab(tab_idx)));
        }
    }

    // 2. Contextual Table Studio OR Legal Redline Review Sub-Bar
    if table_bar_h > 0.0 {
        let tbar = Rect::new(area.x, area.y + ribbon_h, area.w, table_bar_h);
        if has_active_table {
            paint_contextual_table_bar(surface, fonts, hit_regions, mouse_x, mouse_y, s, tbar, w);
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
    surface.fill_rect(body_rect, DaylightTheme::DESK_BG);

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

    let pages = layout_document_pages(fonts, w, scale, content_w, usable_page_h);
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
            if item.block_idx < block_canvas_y.len() {
                block_canvas_y[item.block_idx] = (item_doc_y - 36.0).max(0.0);
            }
            if w.active_table_cell.map(|(b, _, _)| b) == Some(item.block_idx)
                || (w.active_table_cell.is_none() && w.cursor_block == item.block_idx)
            {
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

    // Lazily decode any embedded PNG figures once into `w.image_rgba_cache` for 0-alloc 60Hz scrolling
    for (bi, blk) in w.doc.blocks.iter().enumerate() {
        if let Block::Image(img) = blk {
            if img.rgba_preview.is_none() && !img.png_bytes.is_empty() {
                let already = w
                    .image_rgba_cache
                    .iter()
                    .any(|(b, plen, _, _, _)| *b == bi && *plen == img.png_bytes.len());
                if !already {
                    if let Ok((iw, ih, rgba)) = decode_png_rgba(&img.png_bytes) {
                        w.image_rgba_cache.retain(|(b, _, _, _, _)| *b != bi);
                        w.image_rgba_cache
                            .push((bi, img.png_bytes.len(), iw, ih, rgba));
                    }
                }
            }
        }
    }

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
                DaylightTheme::RUST_ORANGE,
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
        surface.fill_rounded_rect(sheet, 2.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(sheet, 2.0, 1.0, DaylightTheme::BORDER);

        // Running Header
        if !w.doc.page_setup.header_text.is_empty() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                (9.5 * scale).clamp(8.5, 14.0),
                sheet.x + margin_x,
                sheet.y + (margin_top * 0.56).max(20.0),
                content_w,
                DaylightTheme::INK_MUTED,
                &w.doc.page_setup.header_text,
            );
            surface.fill_rect(
                Rect::new(
                    sheet.x + margin_x,
                    sheet.y + (margin_top * 0.68).max(26.0),
                    content_w,
                    1.0,
                ),
                DaylightTheme::BORDER,
            );
        }

        // Running Footer + Page Counter
        let footer_y = sheet.y + sheet.h - (margin_bot * 0.42).max(16.0);
        if !w.doc.page_setup.footer_text.is_empty() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                (9.2 * scale).clamp(8.0, 13.5),
                sheet.x + margin_x,
                footer_y,
                content_w * 0.72,
                DaylightTheme::INK_MUTED,
                &w.doc.page_setup.footer_text,
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
                DaylightTheme::INK_MUTED,
                &p_str,
            );
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
        for item in page_items {
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
                DaylightTheme::PAPER
            } else {
                Color::rgba(255, 255, 255, 228)
            },
        );
        surface.stroke_rounded_rect(pill, 13.0, 1.0, DaylightTheme::BORDER_STRONG);
        let lbl = "Exit Focus · Ctrl+Shift+F";
        let tw = fonts.measure_text(FontSlot::SansBold, lbl, 10.6 * s);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.6 * s,
            pill.x + (pill.w - tw) * 0.5,
            pill.y + pill_h * 0.67,
            DaylightTheme::WORD_ACCENT,
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
    surface.fill_rounded_rect(pop, 7.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(pop, 7.0, 1.0, DaylightTheme::BORDER_STRONG);

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
                    DaylightTheme::WORD_ACCENT
                } else {
                    DaylightTheme::WORD_SOFT
                },
            );
            surface.stroke_rounded_rect(br, 4.5, 1.0, DaylightTheme::WORD_ACCENT);
        } else if *active || hov {
            surface.fill_rounded_rect(
                br,
                4.5,
                if *active {
                    DaylightTheme::WORD_SOFT
                } else {
                    DaylightTheme::HOVER
                },
            );
            if *active {
                surface.stroke_rounded_rect(br, 4.5, 1.0, DaylightTheme::WORD_ACCENT);
            }
        }
        let fg = if *ai_accent && hov {
            DaylightTheme::PAPER
        } else if *ai_accent || *active {
            DaylightTheme::WORD_ACCENT
        } else {
            DaylightTheme::INK
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
            DaylightTheme::INK_SUB
        } else {
            DaylightTheme::BORDER_STRONG
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

struct LaidOutBlock {
    block_idx: usize,
    rel_y: f32,
    height: f32,
    lines: Vec<LaidOutLine>,
    table_row_heights: Vec<f32>,
    table_wrapped_cells: Vec<Vec<Vec<String>>>,
}

struct LaidOutLine {
    text: String,
    char_start: usize,
    slot: FontSlot,
    px_size: f32,
    line_h: f32,
    color: Color,
    highlight: Option<Color>,
    underline: bool,
    strike: bool,
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
                    table_row_heights: Vec::new(),
                    table_wrapped_cells: Vec::new(),
                });
                cur_y += h;
            }
            Block::Image(img) => {
                let h = (img.height_pt * scale).clamp(48.0, 220.0) + 22.0 * scale;
                if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = 0.0;
                }
                pages.last_mut().unwrap().push(LaidOutBlock {
                    block_idx: b_idx,
                    rel_y: cur_y,
                    height: h,
                    lines: Vec::new(),
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

                // Reserve space for on-canvas `+ Add Row` helper rail below table
                let table_body_h: f32 = row_heights.iter().sum();
                let h = table_body_h + 26.0 * scale;
                if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = 0.0;
                }
                pages.last_mut().unwrap().push(LaidOutBlock {
                    block_idx: b_idx,
                    rel_y: cur_y,
                    height: h,
                    lines: Vec::new(),
                    table_row_heights: row_heights,
                    table_wrapped_cells: wrapped_grid,
                });
                cur_y += h;
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
                if cur_y + h > usable_page_h && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = 0.0;
                }
                pages.last_mut().unwrap().push(LaidOutBlock {
                    block_idx: b_idx,
                    rel_y: cur_y + spacing_before,
                    height: h,
                    lines,
                    table_row_heights: Vec::new(),
                    table_wrapped_cells: Vec::new(),
                });
                cur_y += h;
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
    for word in text.split_inclusive(' ') {
        let candidate = format!("{cur}{word}");
        if !cur.is_empty() && fonts.measure_text(slot, &candidate, px_size) > max_w {
            lines.push(cur.trim_end().to_string());
            cur = word.to_string();
        } else {
            // Handle single extra-long token wider than cell width
            if cur.is_empty() && fonts.measure_text(slot, word, px_size) > max_w {
                let mut chunk = String::new();
                for ch in word.chars() {
                    let test = format!("{chunk}{ch}");
                    if !chunk.is_empty() && fonts.measure_text(slot, &test, px_size) > max_w {
                        lines.push(chunk);
                        chunk = ch.to_string();
                    } else {
                        chunk.push(ch);
                    }
                }
                cur = chunk;
            } else {
                cur = candidate;
            }
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
        .map(|s| Color::rgb(s.style.color.r, s.style.color.g, s.style.color.b))
        .unwrap_or(DaylightTheme::INK);
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
    let mut cur_start = 0usize;
    let mut char_cursor = 0usize;

    for token in full_text.split_inclusive(' ') {
        let candidate = format!("{cur_line}{token}");
        let cand_w = fonts.measure_text(slot, &candidate, px_size);
        if !cur_line.is_empty() && cand_w > wrap_w {
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
            cur_start = char_cursor;
        } else {
            cur_line = candidate;
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
        Block::HorizontalRule => {
            surface.fill_rect(
                Rect::new(x, y + item.height * 0.45, content_w, 1.5),
                DaylightTheme::BORDER_STRONG,
            );
        }
        Block::Image(img) => {
            let box_h = (item.height - 16.0 * scale).max(36.0);
            let r = Rect::new(x, y, (img.width_pt * scale).min(content_w), box_h);
            surface.fill_rounded_rect(r, 4.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(r, 4.0, 1.0, DaylightTheme::BORDER_STRONG);
            let inner = Rect::new(
                r.x + 3.0,
                r.y + 3.0,
                (r.w - 6.0).max(2.0),
                (r.h - 6.0).max(2.0),
            );
            if let Some((iw, ih, rgba)) = &img.rgba_preview {
                surface.blit_rgba_scaled(inner, *iw, *ih, rgba);
            } else if let Some((_, _, iw, ih, rgba)) = w
                .image_rgba_cache
                .iter()
                .find(|(b, _, _, _, _)| *b == item.block_idx)
            {
                surface.blit_rgba_scaled(inner, *iw, *ih, rgba);
            } else {
                surface.draw_text(
                    fonts,
                    FontSlot::SansBold,
                    12.0 * scale,
                    r.x + 14.0,
                    r.y + 24.0,
                    DaylightTheme::INK,
                    &format!("[Embedded Figure: {}]", img.name),
                );
            }
        }
        Block::Table(t) => {
            let cols = t.cols.max(1);
            let weights = w.normalized_col_weights(item.block_idx, cols);
            let txt_px = (11.5 * scale).clamp(10.0, 20.0);
            let line_h = (txt_px * 1.36).max(14.0);
            let cell_pad_x = 9.0 * scale;
            let cell_pad_y = 7.5 * scale;
            let border_col = Color::rgb(t.border_color.r, t.border_color.g, t.border_color.b);

            let table_body_h: f32 = item.table_row_heights.iter().sum();
            let table_outer = Rect::new(x - 4.0, y - 4.0, content_w + 28.0 * scale, table_body_h + 26.0 * scale);
            let table_active_or_hovered = w
                .active_table_cell
                .map(|(b, _, _)| b == item.block_idx)
                .unwrap_or(false)
                || table_outer.contains(mouse_x, mouse_y);

            let mut cur_row_y = y;
            for (r_idx, row) in t.rows.iter().enumerate() {
                let rh = item
                    .table_row_heights
                    .get(r_idx)
                    .copied()
                    .unwrap_or(28.0 * scale);
                let mut cx = x;

                for c_idx in 0..cols {
                    let cw = content_w * weights[c_idx];
                    let cell_r = Rect::new(cx, cur_row_y, cw, rh);
                    let cell_opt = row.get(c_idx);
                    if let Some(bg) = cell_opt.and_then(|c| c.bg_color) {
                        surface.fill_rect(cell_r, Color::rgb(bg.r, bg.g, bg.b));
                    } else if r_idx == 0 {
                        surface.fill_rect(cell_r, Color::rgb(244, 241, 235));
                    }

                    let is_active_cell = is_focused
                        && w.active_table_cell == Some((item.block_idx, r_idx, c_idx));

                    surface.stroke_rounded_rect(
                        cell_r,
                        0.0,
                        if is_active_cell { 1.8 } else { 1.0 },
                        if is_active_cell {
                            DaylightTheme::WORD_ACCENT
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
                        .get(r_idx)
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
                            if !ln.is_empty() {
                                surface.draw_text(
                                    fonts,
                                    slot,
                                    txt_px,
                                    tx,
                                    baseline,
                                    DaylightTheme::INK,
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
                                        DaylightTheme::WORD_ACCENT,
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
                        DaylightTheme::RUST_ORANGE,
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
            if table_active_or_hovered {
                let row_btn = Rect::new(x, y + table_body_h + 2.0, content_w, (19.0 * scale).clamp(17.0, 26.0));
                let row_hov = row_btn.contains(mouse_x, mouse_y);
                surface.fill_rounded_rect(
                    row_btn,
                    3.0,
                    if row_hov {
                        DaylightTheme::WORD_SOFT
                    } else {
                        DaylightTheme::SIDEBAR
                    },
                );
                surface.stroke_rounded_rect(
                    row_btn,
                    3.0,
                    1.0,
                    if row_hov {
                        DaylightTheme::WORD_ACCENT
                    } else {
                        DaylightTheme::BORDER
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
                        DaylightTheme::WORD_ACCENT
                    } else {
                        DaylightTheme::INK_SUB
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
                        DaylightTheme::WORD_SOFT
                    } else {
                        DaylightTheme::SIDEBAR
                    },
                );
                surface.stroke_rounded_rect(
                    col_btn,
                    3.0,
                    1.0,
                    if col_hov {
                        DaylightTheme::WORD_ACCENT
                    } else {
                        DaylightTheme::BORDER
                    },
                );
                surface.draw_icon(
                    UiIcon::Plus,
                    col_btn.x + col_btn.w * 0.5,
                    col_btn.y + col_btn.h * 0.5,
                    (11.0 * scale).clamp(9.0, 14.0),
                    if col_hov {
                        DaylightTheme::WORD_ACCENT
                    } else {
                        DaylightTheme::INK_SUB
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
            DaylightTheme::PDF_ACCENT,
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
            DaylightTheme::RUST_ORANGE,
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
                DaylightTheme::INK_SUB,
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
                DaylightTheme::INK_SUB,
                &format!("{n}."),
            );
            text_x += 22.0 * scale;
        }
        ListKind::Checklist(done) => {
            let box_sz = (14.5 * scale).clamp(12.0, 21.0);
            let cb = Rect::new(text_x, y + (first_line_h - box_sz) * 0.36, box_sz, box_sz);
            if done {
                surface.fill_rounded_rect(cb, 3.0, DaylightTheme::SUCCESS);
                surface.draw_line(
                    cb.x + box_sz * 0.22,
                    cb.y + box_sz * 0.52,
                    cb.x + box_sz * 0.44,
                    cb.y + box_sz * 0.76,
                    1.8,
                    DaylightTheme::PAPER,
                );
                surface.draw_line(
                    cb.x + box_sz * 0.44,
                    cb.y + box_sz * 0.76,
                    cb.x + box_sz * 0.80,
                    cb.y + box_sz * 0.26,
                    1.8,
                    DaylightTheme::PAPER,
                );
            } else {
                surface.fill_rounded_rect(cb, 3.0, DaylightTheme::PAPER);
                surface.stroke_rounded_rect(cb, 3.0, 1.4, DaylightTheme::BORDER_STRONG);
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
                DaylightTheme::INK_MUTED
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
                    DaylightTheme::INK_MUTED,
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
                    DaylightTheme::PDF_ACCENT,
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
                    DaylightTheme::WORD_ACCENT,
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
                if let Some(hl) = sp.style.highlight {
                    surface.fill_rounded_rect(
                        Rect::new(seg_x - 1.5, cur_y + 1.0, sw + 3.0, ln.line_h - 2.0),
                        2.5,
                        Color::rgb(hl.r, hl.g, hl.b),
                    );
                }
                let col = if is_checked_done {
                    DaylightTheme::INK_MUTED
                } else {
                    Color::rgb(sp.style.color.r, sp.style.color.g, sp.style.color.b)
                };
                surface.draw_text(fonts, sp_slot, sp_px, seg_x, baseline_y, col, &slice_str);
                if sp.style.underline {
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
        surface.fill_rounded_rect(r, 5.0, if hov { DaylightTheme::HOVER } else { DaylightTheme::PAPER });
        surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansBold,
            11.8 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.66,
            style_w - 22.0 * s,
            DaylightTheme::INK,
            style_lbl,
        );
        surface.draw_icon(
            UiIcon::ChevronDown,
            r.x + r.w - 11.0 * s,
            r.y + bh * 0.52,
            9.0 * s,
            DaylightTheme::INK_MUTED,
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
        surface.fill_rounded_rect(r, 5.0, if hov { DaylightTheme::HOVER } else { DaylightTheme::PAPER });
        surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansRegular,
            11.8 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.66,
            DaylightTheme::INK,
            fam_lbl,
        );
        surface.draw_icon(
            UiIcon::ChevronDown,
            r.x + r.w - 11.0 * s,
            r.y + bh * 0.52,
            9.0 * s,
            DaylightTheme::INK_MUTED,
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
        surface.fill_rounded_rect(r, 5.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::BORDER);
        let btn_seg = (22.0 * s).round();
        let down_r = Rect::new(r.x, r.y, btn_seg, bh);
        let up_r = Rect::new(r.x + r.w - btn_seg, r.y, btn_seg, bh);
        surface.draw_icon(UiIcon::Minus, down_r.x + btn_seg * 0.5, r.y + bh * 0.5, 9.0 * s, DaylightTheme::INK_SUB);
        surface.draw_icon(UiIcon::Plus, up_r.x + btn_seg * 0.5, r.y + bh * 0.5, 9.0 * s, DaylightTheme::INK_SUB);
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
            DaylightTheme::INK,
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

    // 5. Insert Table + Page Tools + Export PDF
    bx = draw_icon_text_btn(
        surface,
        fonts,
        hit_regions,
        mouse_x,
        mouse_y,
        bx,
        by,
        bh,
        s,
        max_x,
        UiIcon::Table,
        "Table",
        w.active_table_cell.is_some(),
        false,
        CommandId::InsertTable,
    );
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
        DaylightTheme::BORDER,
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
        DaylightTheme::PDF_ACCENT,
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
                        DaylightTheme::SUCCESS,
                        DaylightTheme::SUCCESS,
                    )
                } else {
                    (
                        if hov {
                            DaylightTheme::PDF_SOFT
                        } else {
                            Color::rgba(185, 48, 42, 18)
                        },
                        DaylightTheme::PDF_ACCENT,
                        DaylightTheme::PDF_ACCENT,
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
                DaylightTheme::WORD_SOFT
            } else if hov {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(
            r,
            4.0,
            1.0,
            if active {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::BORDER_STRONG
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.8 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.68,
            if active {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::INK
            },
            lbl,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
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
    surface.fill_rect(bar, DaylightTheme::RUST_SOFT);
    surface.fill_rect(
        Rect::new(bar.x, bar.y + bar.h - 1.0, bar.w, 1.0),
        DaylightTheme::BORDER,
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
        DaylightTheme::RUST_ORANGE,
        &cell_info,
    );

    let bh = (25.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut bx = bar.x + 98.0 * s;
    let max_x = bar.x + bar.w - 10.0 * s;

    let actions: &[(&str, CommandId, bool)] = &[
        ("+ Row Below", CommandId::TableAddRow, false),
        ("+ Row Above", CommandId::TableInsertRowAbove, false),
        ("- Row", CommandId::TableDeleteRow, false),
        ("+ Col Right", CommandId::TableAddCol, false),
        ("+ Col Left", CommandId::TableInsertColLeft, false),
        ("- Col", CommandId::TableDeleteCol, false),
        ("Cell Bold", CommandId::TableToggleCellBold, false),
        ("Shade Cell", CommandId::TableCycleCellFill, false),
        ("Delete Table", CommandId::TableDelete, true),
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
                    DaylightTheme::PDF_SOFT
                } else {
                    DaylightTheme::PAPER
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
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::BORDER_STRONG
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.0 * s,
            r.x + 8.0 * s,
            r.y + bh * 0.68,
            if *danger {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::INK
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
                DaylightTheme::WORD_SOFT
            } else {
                DaylightTheme::HOVER
            },
        );
    }
    if active {
        surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::WORD_ACCENT);
    }
    surface.draw_icon(
        icon,
        r.x + bw * 0.5,
        r.y + bh * 0.5,
        13.5 * s,
        if active {
            DaylightTheme::WORD_ACCENT
        } else {
            DaylightTheme::INK_SUB
        },
    );
    hit_regions.push(HitRegion::button(r, action));
    bx + bw + 3.0 * s
}

#[allow(clippy::too_many_arguments)]
fn draw_icon_text_btn(
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
    icon: UiIcon,
    label: &str,
    active: bool,
    accent: bool,
    cmd: CommandId,
) -> f32 {
    let tw = fonts.measure_text(FontSlot::SansBold, label, 11.5 * s);
    let bw = (tw + 30.0 * s).round();
    if bx + bw > max_x {
        return bx;
    }
    let r = Rect::new(bx, by, bw, bh);
    let hov = r.contains(mouse_x, mouse_y);
    if active || hov || accent {
        surface.fill_rounded_rect(
            r,
            5.0,
            if accent {
                DaylightTheme::PDF_SOFT
            } else if active {
                DaylightTheme::WORD_SOFT
            } else {
                DaylightTheme::HOVER
            },
        );
    }
    if active || accent {
        surface.stroke_rounded_rect(
            r,
            5.0,
            1.0,
            if accent {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::WORD_ACCENT
            },
        );
    }
    let fg = if accent {
        DaylightTheme::PDF_ACCENT
    } else if active {
        DaylightTheme::WORD_ACCENT
    } else {
        DaylightTheme::INK
    };
    surface.draw_icon(icon, r.x + 12.0 * s, r.y + bh * 0.5, 12.5 * s, fg);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        11.5 * s,
        r.x + 22.0 * s,
        r.y + bh * 0.66,
        fg,
        label,
    );
    hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
    bx + bw + 4.0 * s
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
                DaylightTheme::PDF_SOFT
            } else if active {
                DaylightTheme::WORD_SOFT
            } else {
                DaylightTheme::HOVER
            },
        );
    }
    if pdf_accent || active {
        surface.stroke_rounded_rect(
            r,
            5.0,
            1.0,
            if pdf_accent {
                DaylightTheme::PDF_ACCENT
            } else {
                DaylightTheme::WORD_ACCENT
            },
        );
    }
    let fg = if pdf_accent {
        DaylightTheme::PDF_ACCENT
    } else if active {
        DaylightTheme::WORD_ACCENT
    } else {
        DaylightTheme::INK
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
        DaylightTheme::BORDER,
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
    surface.fill_rect(sb, DaylightTheme::SIDEBAR);
    surface.fill_rect(
        Rect::new(sb.x + sb.w - 1.0, sb.y, 1.0, sb.h),
        DaylightTheme::BORDER,
    );

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.5 * s,
        sb.x + 14.0 * s,
        sb.y + 24.0 * s,
        DaylightTheme::INK_MUTED,
        "OUTLINE",
    );

    let outline = w.doc.outline();
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
            DaylightTheme::INK_MUTED,
            "Apply Heading 1–3 to build a live outline.",
        );
    } else {
        for item in &outline {
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
                        DaylightTheme::WORD_SOFT
                    } else {
                        DaylightTheme::HOVER
                    },
                );
            }
            if active {
                surface.fill_rect(
                    Rect::new(row.x, row.y + 5.0 * s, 2.5, row.h - 10.0 * s),
                    DaylightTheme::WORD_ACCENT,
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
                    DaylightTheme::WORD_ACCENT
                } else {
                    DaylightTheme::INK_SUB
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
            DaylightTheme::BORDER,
        );
        y += 16.0 * s;
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.2 * s,
            sb.x + 14.0 * s,
            y,
            DaylightTheme::INK_MUTED,
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
                    DaylightTheme::HOVER
                } else {
                    DaylightTheme::PAPER
                },
            );
            surface.stroke_rounded_rect(crow, 4.0, 1.0, DaylightTheme::BORDER);
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansBold,
                10.6 * s,
                crow.x + 8.0 * s,
                crow.y + 15.0 * s,
                crow.w - 16.0 * s,
                DaylightTheme::WORD_ACCENT,
                &format!("{} · ¶{}", cmt.author, cmt.block_index + 1),
            );
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                10.6 * s,
                crow.x + 8.0 * s,
                crow.y + 31.0 * s,
                crow.w - 16.0 * s,
                DaylightTheme::INK_SUB,
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
    let stats = w.doc.stats();
    let m_top = (sb.y + sb.h - footer_h + 8.0 * s).max(y + 12.0 * s);
    if m_top + 96.0 * s <= sb.y + sb.h {
        surface.fill_rect(
            Rect::new(sb.x + 14.0 * s, m_top, sb.w - 28.0 * s, 1.0),
            DaylightTheme::BORDER,
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            10.2 * s,
            sb.x + 14.0 * s,
            m_top + 18.0 * s,
            DaylightTheme::INK_MUTED,
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
                DaylightTheme::INK_MUTED,
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
                DaylightTheme::INK,
                val,
            );
        }
    }
}
