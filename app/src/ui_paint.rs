//! Master "Paper & Daylight" Native GUI Painter:
//! Unified Top Header, Home Hub Studio, Side-by-Side Split Workspace,
//! Modals (`Ctrl+K` Command Palette, Signature Pad, Find/Replace, Snapshots, About), & Status Bar.

use crate::commands::{filter_commands, CommandId};
use crate::pdf_view::paint_pdf_tab;
use crate::state::{
    ActiveModal, FileModalKind, HitRegion, RustPadApp, StudioTab, UiAction,
};
use crate::word_view::paint_word_tab;
use doc_model::TemplateKind;
use rustpad_render::{Color, DaylightTheme, FontBook, FontSlot, Rect, Surface};

pub fn render_app(app: &mut RustPadApp, surface: &mut Surface) {
    app.hit_regions.clear();
    surface.reset_clip();
    surface.clear(DaylightTheme::DESK_BG);

    let win_w = surface.width as f32;
    let win_h = surface.height as f32;
    let header_h = 50.0;
    let status_h = 28.0;

    // 1. Unified Header Bar (Logo + Tabs + Split + Command Palette + Quick Save)
    paint_header_bar(app, surface, Rect::new(0.0, 0.0, win_w, header_h));

    // 2. Main Workspace Area (Single Tab or Side-by-Side 50/50 Split Workspace)
    let work = Rect::new(
        0.0,
        header_h,
        win_w,
        (win_h - header_h - status_h).max(40.0),
    );

    if let Some(sec_idx) = app.split_secondary_tab.filter(|&s| s < app.tabs.len() && s != app.active_tab) {
        let left_w = ((work.w - 4.0) * 0.5).floor();
        let left_rect = Rect::new(work.x, work.y, left_w, work.h);
        let right_rect = Rect::new(work.x + left_w + 4.0, work.y, work.w - left_w - 4.0, work.h);

        paint_tab_pane(app, surface, app.active_tab, true, left_rect);
        surface.fill_rect(
            Rect::new(work.x + left_w, work.y, 4.0, work.h),
            DaylightTheme::BORDER_STRONG,
        );
        paint_tab_pane(app, surface, sec_idx, false, right_rect);
    } else {
        paint_tab_pane(app, surface, app.active_tab, true, work);
    }

    // 3. Bottom Daylight Status Bar
    paint_status_bar(
        app,
        surface,
        Rect::new(0.0, win_h - status_h, win_w, status_h),
    );

    // 4. Active Modal Overlay (Command Palette, Signature Pad, Find/Replace, File Prompt, etc.)
    if app.modal.is_some() {
        paint_active_modal(app, surface, Rect::new(0.0, 0.0, win_w, win_h));
    }
}

fn paint_tab_pane(
    app: &mut RustPadApp,
    surface: &mut Surface,
    tab_idx: usize,
    is_focused_pane: bool,
    area: Rect,
) {
    let blink = app.cursor_blink_on;
    let mx = app.mouse_x;
    let my = app.mouse_y;

    // Borrow fields cleanly so `fonts`, `spellchecker`, `hit_regions`, and `tabs[tab_idx]` don't alias
    let Some(tab_ref) = app.tabs.get(tab_idx) else {
        return;
    };
    match tab_ref {
        StudioTab::HomeHub => {
            let recent = app.recent_files.clone();
            paint_home_hub(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                mx,
                my,
                area,
                &recent,
            );
        }
        StudioTab::Word(w) => {
            paint_word_tab(
                surface,
                &mut app.fonts,
                &app.spellchecker,
                &mut app.hit_regions,
                tab_idx,
                is_focused_pane,
                blink,
                mx,
                my,
                area,
                w,
            );
        }
        StudioTab::Pdf(p) => {
            paint_pdf_tab(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                tab_idx,
                is_focused_pane,
                blink,
                mx,
                my,
                area,
                p,
            );
        }
    }
}

fn paint_header_bar(app: &mut RustPadApp, surface: &mut Surface, bar: Rect) {
    surface.fill_rect(bar, DaylightTheme::PAPER);
    surface.fill_rect(
        Rect::new(bar.x, bar.y + bar.h - 1.0, bar.w, 1.0),
        DaylightTheme::BORDER,
    );

    // Brand Logo + Name (clicks jump to Home Hub)
    surface.draw_logo_36(12, 7);
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        15.0,
        56.0,
        30.5,
        DaylightTheme::INK,
        "RustPad",
    );
    app.hit_regions.push(HitRegion {
        rect: Rect::new(8.0, 4.0, 114.0, 42.0),
        action: UiAction::OpenHomeTab,
    });

    // Right-side Quick Controls (`Ctrl+K`, `Split`, `Save`, `Open`)
    let right_reserve = 352.0;
    let tabs_max_x = (bar.w - right_reserve).max(160.0);

    // Multi-Document Tab Strip
    let mut tx = 132.0;
    for idx in 0..app.tabs.len() {
        let title = app.tabs[idx].title();
        let (badge, is_pdf) = app.tabs[idx].badge();
        let active = idx == app.active_tab;
        let tw = app
            .fonts
            .measure_text(FontSlot::SansBold, &title, 12.0)
            .min(148.0);
        let tab_w = if idx == 0 {
            74.0
        } else {
            (tw + 68.0).clamp(112.0, 210.0)
        };
        if tx + tab_w > tabs_max_x {
            break;
        }
        let tab_r = Rect::new(tx, 9.0, tab_w, 33.0);
        let hov = tab_r.contains(app.mouse_x, app.mouse_y);

        surface.fill_rounded_rect(
            tab_r,
            7.0,
            if active {
                if is_pdf {
                    DaylightTheme::PDF_SOFT
                } else {
                    DaylightTheme::WORD_SOFT
                }
            } else if hov {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::SIDEBAR
            },
        );
        surface.stroke_rounded_rect(
            tab_r,
            7.0,
            1.0,
            if active {
                if is_pdf {
                    DaylightTheme::PDF_ACCENT
                } else {
                    DaylightTheme::WORD_ACCENT
                }
            } else {
                DaylightTheme::BORDER
            },
        );

        if idx == 0 {
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                12.0,
                tab_r.x + 18.0,
                tab_r.y + 21.0,
                if active {
                    DaylightTheme::WORD_ACCENT
                } else {
                    DaylightTheme::INK_SUB
                },
                "Home",
            );
        } else {
            let pill = Rect::new(tab_r.x + 8.0, tab_r.y + 8.0, 36.0, 17.0);
            surface.fill_rounded_rect(
                pill,
                4.0,
                if is_pdf {
                    DaylightTheme::PDF_ACCENT
                } else {
                    DaylightTheme::WORD_ACCENT
                },
            );
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                9.5,
                pill.x + 5.0,
                pill.y + 12.2,
                DaylightTheme::PAPER,
                badge,
            );
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::SansBold,
                12.0,
                pill.x + pill.w + 7.0,
                tab_r.y + 21.0,
                tab_r.w - 72.0,
                DaylightTheme::INK,
                &title,
            );

            // Close tab button
            let close_r = Rect::new(tab_r.x + tab_r.w - 22.0, tab_r.y + 8.0, 16.0, 16.0);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                12.5,
                close_r.x + 4.0,
                close_r.y + 12.5,
                DaylightTheme::INK_MUTED,
                "×",
            );
            app.hit_regions.push(HitRegion {
                rect: tab_r,
                action: UiAction::SwitchTab(idx),
            });
            app.hit_regions.push(HitRegion {
                rect: close_r,
                action: UiAction::CloseTab(idx),
            });
            tx += tab_w + 6.0;
            continue;
        }

        app.hit_regions.push(HitRegion {
            rect: tab_r,
            action: UiAction::SwitchTab(idx),
        });
        tx += tab_w + 6.0;
    }

    // '+' New Word Tab quick button
    if tx + 32.0 <= tabs_max_x {
        let plus_r = Rect::new(tx, 10.0, 30.0, 30.0);
        surface.fill_rounded_rect(plus_r, 6.0, DaylightTheme::SIDEBAR);
        surface.stroke_rounded_rect(plus_r, 6.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            &mut app.fonts,
            FontSlot::SansBold,
            15.0,
            plus_r.x + 9.5,
            plus_r.y + 20.5,
            DaylightTheme::INK_SUB,
            "+",
        );
        app.hit_regions.push(HitRegion {
            rect: plus_r,
            action: UiAction::ExecCommand(CommandId::NewWordBlank),
        });
    }

    // Right Action Pills (`Ctrl+K`, `Split`, `Open`, `Save`)
    let mut rx = (bar.w - 340.0).max(tx + 8.0);

    // Command Palette Pill
    let pal_r = Rect::new(rx, 10.0, 134.0, 30.0);
    surface.fill_rounded_rect(pal_r, 7.0, DaylightTheme::SIDEBAR);
    surface.stroke_rounded_rect(pal_r, 7.0, 1.0, DaylightTheme::BORDER_STRONG);
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansRegular,
        11.5,
        pal_r.x + 11.0,
        pal_r.y + 19.5,
        DaylightTheme::INK_SUB,
        "Commands  Ctrl+K",
    );
    app.hit_regions.push(HitRegion {
        rect: pal_r,
        action: UiAction::OpenCommandPalette,
    });
    rx += 140.0;

    let split_on = app.split_secondary_tab.is_some();
    let quick_btns: &[(&str, CommandId, bool)] = &[
        ("Split", CommandId::ToggleSplitView, split_on),
        ("Open", CommandId::OpenFile, false),
        ("Save", CommandId::SaveFile, true),
    ];

    for (lbl, cmd, primary) in quick_btns {
        let bw = 58.0;
        let br = Rect::new(rx, 10.0, bw, 30.0);
        if *primary && *cmd == CommandId::SaveFile {
            surface.fill_rounded_rect(br, 7.0, DaylightTheme::WORD_ACCENT);
            let lw = app.fonts.measure_text(FontSlot::SansBold, lbl, 12.0);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                12.0,
                br.x + (bw - lw) * 0.5,
                br.y + 19.5,
                DaylightTheme::PAPER,
                lbl,
            );
        } else {
            surface.fill_rounded_rect(
                br,
                7.0,
                if *primary {
                    DaylightTheme::WORD_SOFT
                } else {
                    DaylightTheme::PAPER
                },
            );
            surface.stroke_rounded_rect(br, 7.0, 1.0, DaylightTheme::BORDER);
            let lw = app.fonts.measure_text(FontSlot::SansBold, lbl, 12.0);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                12.0,
                br.x + (bw - lw) * 0.5,
                br.y + 19.5,
                if *primary {
                    DaylightTheme::WORD_ACCENT
                } else {
                    DaylightTheme::INK
                },
                lbl,
            );
        }
        app.hit_regions.push(HitRegion {
            rect: br,
            action: UiAction::ExecCommand(*cmd),
        });
        rx += bw + 6.0;
    }
}

fn paint_home_hub(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    area: Rect,
    recent_files: &[std::path::PathBuf],
) {
    surface.fill_rect(area, DaylightTheme::DESK_BG);

    let pad_x = ((area.w - 1040.0) * 0.5).max(28.0);
    let max_w = (area.w - pad_x * 2.0).min(1040.0);
    let mut y = area.y + 28.0;

    // Hero Banner Card
    let hero = Rect::new(pad_x, y, max_w, 118.0);
    surface.draw_paper_shadow(hero, 12.0, 12.0, 20);
    surface.fill_rounded_rect(hero, 12.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(hero, 12.0, 1.0, DaylightTheme::BORDER);

    surface.draw_logo_64(hero.x as i32 + 24, hero.y as i32 + 27);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        24.0,
        hero.x + 106.0,
        hero.y + 48.0,
        DaylightTheme::INK,
        "RustPad — Word & PDF Studio",
    );
    surface.draw_text_clipped(
        fonts,
        FontSlot::SansRegular,
        13.5,
        hero.x + 106.0,
        hero.y + 72.0,
        hero.w - 360.0,
        DaylightTheme::INK_SUB,
        "100% Pure-Rust Native Desktop Office Suite · Zero WebView · Zero Cloud Lock-in · Instant Cold Start",
    );
    surface.draw_text(
        fonts,
        FontSlot::MonoRegular,
        11.5,
        hero.x + 106.0,
        hero.y + 95.0,
        DaylightTheme::RUST_ORANGE,
        "By Nour Yahyaoui · github.com/Nour-yahyaoui/word · nourryahyaoui@gmail.com",
    );

    // Quick Hero CTAs on right
    let new_pdf_btn = Rect::new(hero.x + hero.w - 224.0, hero.y + 24.0, 200.0, 32.0);
    surface.fill_rounded_rect(new_pdf_btn, 7.0, DaylightTheme::PDF_ACCENT);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        12.0,
        new_pdf_btn.x + 22.0,
        new_pdf_btn.y + 20.5,
        DaylightTheme::PAPER,
        "+ New Blank PDF Studio",
    );
    hit_regions.push(HitRegion {
        rect: new_pdf_btn,
        action: UiAction::ExecCommand(CommandId::NewPdfBlank),
    });

    let open_btn = Rect::new(hero.x + hero.w - 224.0, hero.y + 64.0, 200.0, 32.0);
    surface.fill_rounded_rect(open_btn, 7.0, DaylightTheme::WORD_SOFT);
    surface.stroke_rounded_rect(open_btn, 7.0, 1.0, DaylightTheme::WORD_ACCENT);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        12.0,
        open_btn.x + 20.0,
        open_btn.y + 20.5,
        DaylightTheme::WORD_ACCENT,
        "Open .docx / .pdf / .md…",
    );
    hit_regions.push(HitRegion {
        rect: open_btn,
        action: UiAction::ExecCommand(CommandId::OpenFile),
    });

    y += 144.0;

    // Starter Templates Grid (6 cards, 3 columns)
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        12.0,
        pad_x,
        y,
        DaylightTheme::INK_MUTED,
        "CREATE WORD DOCUMENT (.DOCX) FROM STARTER TEMPLATE",
    );
    y += 14.0;

    let cols = if max_w >= 760.0 { 3 } else { 2 };
    let gap = 16.0;
    let card_w = (max_w - gap * ((cols - 1) as f32)) / (cols as f32);
    let card_h = 104.0;

    for (i, tpl) in TemplateKind::ALL.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let cx = pad_x + (col as f32) * (card_w + gap);
        let cy = y + (row as f32) * (card_h + gap);
        let card = Rect::new(cx, cy, card_w, card_h);
        let hov = card.contains(mouse_x, mouse_y);

        surface.draw_paper_shadow(card, 9.0, 8.0, if hov { 26 } else { 14 });
        surface.fill_rounded_rect(
            card,
            9.0,
            if hov {
                DaylightTheme::WORD_SOFT
            } else {
                DaylightTheme::PAPER
            },
        );
        surface.stroke_rounded_rect(
            card,
            9.0,
            if hov { 1.6 } else { 1.0 },
            if hov {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );

        // Mini sheet icon
        let doc_icon = Rect::new(card.x + 16.0, card.y + 18.0, 42.0, 56.0);
        surface.fill_rounded_rect(doc_icon, 4.0, DaylightTheme::SIDEBAR);
        surface.stroke_rounded_rect(doc_icon, 4.0, 1.0, DaylightTheme::WORD_ACCENT);
        surface.fill_rect(
            Rect::new(doc_icon.x + 7.0, doc_icon.y + 12.0, 26.0, 4.0),
            DaylightTheme::WORD_ACCENT,
        );
        surface.fill_rect(
            Rect::new(doc_icon.x + 7.0, doc_icon.y + 22.0, 28.0, 3.0),
            DaylightTheme::BORDER_STRONG,
        );
        surface.fill_rect(
            Rect::new(doc_icon.x + 7.0, doc_icon.y + 30.0, 22.0, 3.0),
            DaylightTheme::BORDER_STRONG,
        );

        surface.draw_text_clipped(
            fonts,
            FontSlot::SansBold,
            14.5,
            card.x + 72.0,
            card.y + 38.0,
            card.w - 84.0,
            DaylightTheme::INK,
            tpl.label(),
        );
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansRegular,
            12.0,
            card.x + 72.0,
            card.y + 60.0,
            card.w - 84.0,
            DaylightTheme::INK_SUB,
            tpl.subtitle(),
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.0,
            card.x + 72.0,
            card.y + 82.0,
            DaylightTheme::WORD_ACCENT,
            "Open in Studio ->",
        );

        hit_regions.push(HitRegion {
            rect: card,
            action: UiAction::CreateFromTemplate(*tpl),
        });
    }

    let rows_used = TemplateKind::ALL.len().div_ceil(cols);
    y += (rows_used as f32) * (card_h + gap) + 18.0;

    // Recent Workspace Documents Panel
    if y + 110.0 <= area.y + area.h {
        let rec_box = Rect::new(pad_x, y, max_w, (area.y + area.h - y - 20.0).min(168.0));
        surface.fill_rounded_rect(rec_box, 10.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(rec_box, 10.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.5,
            rec_box.x + 18.0,
            rec_box.y + 24.0,
            DaylightTheme::INK_MUTED,
            "WORKSPACE DOCUMENTS & POWER TOOLS",
        );

        if recent_files.is_empty() {
            surface.draw_text(
                fonts,
                FontSlot::SansRegular,
                12.5,
                rec_box.x + 18.0,
                rec_box.y + 52.0,
                DaylightTheme::INK_SUB,
                "No saved .docx or .pdf files in the current folder yet — pick any template above or press Ctrl+K.",
            );
        } else {
            let mut ry = rec_box.y + 36.0;
            for p in recent_files.iter().take(3) {
                let row = Rect::new(rec_box.x + 14.0, ry, rec_box.w - 28.0, 30.0);
                if row.contains(mouse_x, mouse_y) {
                    surface.fill_rounded_rect(row, 6.0, DaylightTheme::HOVER);
                }
                let disp = p.display().to_string();
                let is_pdf = disp.to_ascii_lowercase().ends_with(".pdf");
                surface.draw_text(
                    fonts,
                    FontSlot::SansBold,
                    11.0,
                    row.x + 10.0,
                    row.y + 19.5,
                    if is_pdf {
                        DaylightTheme::PDF_ACCENT
                    } else {
                        DaylightTheme::WORD_ACCENT
                    },
                    if is_pdf { "[PDF]" } else { "[DOCX]" },
                );
                surface.draw_text_clipped(
                    fonts,
                    FontSlot::SansRegular,
                    12.5,
                    row.x + 62.0,
                    row.y + 19.5,
                    row.w - 80.0,
                    DaylightTheme::INK,
                    &disp,
                );
                hit_regions.push(HitRegion {
                    rect: row,
                    action: UiAction::OpenRecentPath(p.clone()),
                });
                ry += 32.0;
            }
        }
    }
}

fn paint_status_bar(app: &mut RustPadApp, surface: &mut Surface, bar: Rect) {
    surface.fill_rect(bar, DaylightTheme::CHROME);
    surface.fill_rect(Rect::new(bar.x, bar.y, bar.w, 1.0), DaylightTheme::BORDER);

    surface.fill_rounded_rect(
        Rect::new(bar.x + 12.0, bar.y + 9.0, 8.0, 8.0),
        4.0,
        DaylightTheme::SUCCESS,
    );
    surface.draw_text_clipped(
        &mut app.fonts,
        FontSlot::SansRegular,
        11.5,
        bar.x + 26.0,
        bar.y + 18.0,
        (bar.w - 340.0).max(120.0),
        DaylightTheme::INK_SUB,
        &app.status_toast,
    );

    let zoom_pct = match app.tabs.get(app.active_tab) {
        Some(StudioTab::Word(w)) => (w.zoom * 100.0).round() as i32,
        Some(StudioTab::Pdf(p)) => (p.zoom * 100.0).round() as i32,
        _ => 100,
    };

    let right_x = (bar.w - 255.0).max(200.0);
    let minus_r = Rect::new(right_x, bar.y + 4.0, 24.0, 20.0);
    let reset_r = Rect::new(right_x + 28.0, bar.y + 4.0, 52.0, 20.0);
    let plus_r = Rect::new(right_x + 84.0, bar.y + 4.0, 24.0, 20.0);
    let about_r = Rect::new(right_x + 118.0, bar.y + 4.0, 124.0, 20.0);

    for (r, txt, cmd) in [
        (minus_r, "-", CommandId::ZoomOut),
        (plus_r, "+", CommandId::ZoomIn),
    ] {
        surface.fill_rounded_rect(r, 4.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(r, 4.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            &mut app.fonts,
            FontSlot::SansBold,
            12.0,
            r.x + 8.0,
            r.y + 14.5,
            DaylightTheme::INK,
            txt,
        );
        app.hit_regions.push(HitRegion {
            rect: r,
            action: UiAction::ExecCommand(cmd),
        });
    }

    surface.draw_text(
        &mut app.fonts,
        FontSlot::MonoRegular,
        11.0,
        reset_r.x + 8.0,
        reset_r.y + 14.5,
        DaylightTheme::INK_SUB,
        &format!("{zoom_pct}%"),
    );
    app.hit_regions.push(HitRegion {
        rect: reset_r,
        action: UiAction::ExecCommand(CommandId::ZoomReset),
    });

    surface.fill_rounded_rect(about_r, 4.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(about_r, 4.0, 1.0, DaylightTheme::BORDER);
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        10.5,
        about_r.x + 10.0,
        about_r.y + 14.2,
        DaylightTheme::RUST_ORANGE,
        "About RustPad v1.0",
    );
    app.hit_regions.push(HitRegion {
        rect: about_r,
        action: UiAction::ExecCommand(CommandId::OpenAbout),
    });
}

fn paint_active_modal(app: &mut RustPadApp, surface: &mut Surface, screen: Rect) {
    surface.fill_rect(screen, DaylightTheme::SCRIM);

    // Clicking backdrop closes modal (inserted BEFORE modal inner hits so inner buttons win)
    app.hit_regions.push(HitRegion {
        rect: screen,
        action: UiAction::ModalClose,
    });

    // Clone lightweight modal display state or update SignaturePad rect
    match &mut app.modal {
        Some(ActiveModal::CommandPalette { query, selected }) => {
            let q = query.clone();
            let sel = *selected;
            let mw = 560.0f32.min(screen.w - 32.0);
            let mh = 392.0f32.min(screen.h - 48.0);
            let card = Rect::new((screen.w - mw) * 0.5, 68.0, mw, mh);
            block_backdrop_inside(&mut app.hit_regions, card);

            surface.draw_paper_shadow(card, 12.0, 18.0, 48);
            surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

            let input_r = Rect::new(card.x + 16.0, card.y + 14.0, card.w - 32.0, 38.0);
            surface.fill_rounded_rect(input_r, 8.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(input_r, 8.0, 1.5, DaylightTheme::WORD_ACCENT);
            let prompt = if q.is_empty() {
                "Type a command (Word, PDF, Sign, Convert, Split, Compress)…".to_string()
            } else {
                q.clone()
            };
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::SansRegular,
                13.5,
                input_r.x + 12.0,
                input_r.y + 24.5,
                input_r.w - 24.0,
                if q.is_empty() {
                    DaylightTheme::INK_MUTED
                } else {
                    DaylightTheme::INK
                },
                &prompt,
            );

            let matches = filter_commands(&q);
            let mut ry = input_r.y + input_r.h + 10.0;
            for (i, spec) in matches.iter().take(9).enumerate() {
                let row = Rect::new(card.x + 12.0, ry, card.w - 24.0, 33.0);
                let is_sel = i == sel;
                if is_sel || row.contains(app.mouse_x, app.mouse_y) {
                    surface.fill_rounded_rect(
                        row,
                        7.0,
                        if is_sel {
                            DaylightTheme::WORD_SOFT
                        } else {
                            DaylightTheme::HOVER
                        },
                    );
                }
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::SansBold,
                    10.5,
                    row.x + 10.0,
                    row.y + 21.0,
                    if spec.category == "PDF" {
                        DaylightTheme::PDF_ACCENT
                    } else {
                        DaylightTheme::WORD_ACCENT
                    },
                    spec.category,
                );
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.5,
                    row.x + 68.0,
                    row.y + 21.0,
                    row.w - 144.0,
                    DaylightTheme::INK,
                    spec.title,
                );
                if !spec.shortcut.is_empty() {
                    let sw = app
                        .fonts
                        .measure_text(FontSlot::MonoRegular, spec.shortcut, 10.5);
                    surface.draw_text(
                        &mut app.fonts,
                        FontSlot::MonoRegular,
                        10.5,
                        row.x + row.w - sw - 12.0,
                        row.y + 21.0,
                        DaylightTheme::INK_MUTED,
                        spec.shortcut,
                    );
                }
                app.hit_regions.push(HitRegion {
                    rect: row,
                    action: UiAction::PalettePick(spec.id),
                });
                ry += 35.0;
            }
        }
        Some(ActiveModal::SignaturePad {
            strokes,
            pad_rect,
            signer_name,
            ..
        }) => {
            let mw = 500.0f32.min(screen.w - 32.0);
            let mh = 320.0;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);
            let pad = Rect::new(card.x + 20.0, card.y + 62.0, card.w - 40.0, 178.0);
            *pad_rect = pad;
            let strokes_copy = strokes.clone();
            let signer = signer_name.clone();

            block_backdrop_inside(&mut app.hit_regions, card);
            surface.draw_paper_shadow(card, 12.0, 18.0, 48);
            surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                16.0,
                card.x + 20.0,
                card.y + 34.0,
                DaylightTheme::INK,
                &format!("Draw Ink Signature ({signer})"),
            );
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansRegular,
                12.0,
                card.x + 20.0,
                card.y + 52.0,
                DaylightTheme::INK_SUB,
                "Drag mouse inside the signature sheet below, then click Place on PDF",
            );

            surface.fill_rounded_rect(pad, 8.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(pad, 8.0, 1.5, DaylightTheme::WORD_ACCENT);
            surface.fill_rect(
                Rect::new(pad.x + 24.0, pad.y + pad.h - 36.0, pad.w - 48.0, 1.0),
                DaylightTheme::BORDER_STRONG,
            );

            for s in &strokes_copy {
                for w in s.windows(2) {
                    surface.draw_line(
                        w[0].0,
                        w[0].1,
                        w[1].0,
                        w[1].1,
                        2.6,
                        DaylightTheme::WORD_ACCENT,
                    );
                }
            }

            paint_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                card,
                Some(("Clear Ink", UiAction::SignatureClear)),
                "Place Signature on PDF",
            );
        }
        Some(ActiveModal::FilePrompt {
            kind,
            input_path,
            recent_candidates,
        }) => {
            let k = *kind;
            let inp = input_path.clone();
            let cands = recent_candidates.clone();
            let mw = 520.0f32.min(screen.w - 32.0);
            let mh = 320.0;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);
            block_backdrop_inside(&mut app.hit_regions, card);

            surface.draw_paper_shadow(card, 12.0, 18.0, 48);
            surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

            let title = match k {
                FileModalKind::OpenAny => "Open Document (.docx / .pdf / .md)",
                FileModalKind::SaveActive => "Save Active Document As…",
                FileModalKind::ExportPdf => "Export Crisp Vector PDF (.pdf)",
                FileModalKind::MergePdf => "Merge Another PDF into Current Tab",
            };
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                16.0,
                card.x + 20.0,
                card.y + 34.0,
                DaylightTheme::INK,
                title,
            );

            let box_r = Rect::new(card.x + 20.0, card.y + 52.0, card.w - 40.0, 36.0);
            surface.fill_rounded_rect(box_r, 6.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(box_r, 6.0, 1.5, DaylightTheme::WORD_ACCENT);
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::MonoRegular,
                13.0,
                box_r.x + 10.0,
                box_r.y + 23.0,
                box_r.w - 20.0,
                DaylightTheme::INK,
                if inp.is_empty() {
                    "Type path e.g. examples/Welcome.docx…"
                } else {
                    &inp
                },
            );

            let mut cy = box_r.y + 50.0;
            for p in cands.iter().take(4) {
                let row = Rect::new(card.x + 20.0, cy, card.w - 40.0, 28.0);
                if row.contains(app.mouse_x, app.mouse_y) {
                    surface.fill_rounded_rect(row, 5.0, DaylightTheme::HOVER);
                }
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.0,
                    row.x + 8.0,
                    row.y + 18.5,
                    row.w - 16.0,
                    DaylightTheme::INK_SUB,
                    &p.display().to_string(),
                );
                app.hit_regions.push(HitRegion {
                    rect: row,
                    action: UiAction::FileCandidatePick(p.clone()),
                });
                cy += 30.0;
            }

            paint_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                card,
                None,
                "Confirm",
            );
        }
        Some(ActiveModal::FindReplace {
            find_text,
            replace_text,
            editing_replace,
            matches_count,
        }) => {
            let ft = find_text.clone();
            let rt = replace_text.clone();
            let rep_focus = *editing_replace;
            let cnt = *matches_count;
            let mw = 460.0f32.min(screen.w - 32.0);
            let mh = 248.0;
            let card = Rect::new((screen.w - mw) * 0.5, 90.0, mw, mh);
            block_backdrop_inside(&mut app.hit_regions, card);

            surface.draw_paper_shadow(card, 12.0, 18.0, 48);
            surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                15.5,
                card.x + 20.0,
                card.y + 32.0,
                DaylightTheme::INK,
                &format!("Find & Replace ({cnt} matches)"),
            );

            let f_box = Rect::new(card.x + 20.0, card.y + 52.0, card.w - 40.0, 36.0);
            let r_box = Rect::new(card.x + 20.0, card.y + 104.0, card.w - 40.0, 36.0);
            for (bx, active, label, val, focus_flag) in [
                (f_box, !rep_focus, "Find:", &ft, false),
                (r_box, rep_focus, "Replace:", &rt, true),
            ] {
                surface.fill_rounded_rect(bx, 6.0, DaylightTheme::SIDEBAR);
                surface.stroke_rounded_rect(
                    bx,
                    6.0,
                    if active { 1.6 } else { 1.0 },
                    if active {
                        DaylightTheme::WORD_ACCENT
                    } else {
                        DaylightTheme::BORDER
                    },
                );
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::SansBold,
                    11.5,
                    bx.x + 10.0,
                    bx.y + 22.5,
                    DaylightTheme::INK_MUTED,
                    label,
                );
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    13.0,
                    bx.x + 74.0,
                    bx.y + 22.5,
                    bx.w - 84.0,
                    DaylightTheme::INK,
                    val,
                );
                app.hit_regions.push(HitRegion {
                    rect: bx,
                    action: UiAction::FindFocusField(focus_flag),
                });
            }

            paint_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                card,
                None,
                "Replace All",
            );
            // Override confirm button to trigger FindReplaceAll
            if let Some(last) = app.hit_regions.last_mut() {
                last.action = UiAction::FindReplaceAll;
            }
        }
        Some(ActiveModal::SplitRange { range_spec }) => {
            let spec = range_spec.clone();
            paint_single_input_modal(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                screen,
                "Split / Extract PDF Page Ranges",
                "Enter 1-based page ranges (e.g. 1-2, 4):",
                &spec,
                "Extract into New Tab",
            );
        }
        Some(ActiveModal::Watermark { text }) => {
            let wm = text.clone();
            paint_single_input_modal(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                screen,
                "Set Page Watermark",
                "Enter diagonal watermark text (leave empty to clear):",
                &wm,
                "Apply Watermark",
            );
        }
        Some(ActiveModal::SnapshotsDrawer) => {
            let mw = 480.0f32.min(screen.w - 32.0);
            let mh = 340.0;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);
            block_backdrop_inside(&mut app.hit_regions, card);

            surface.draw_paper_shadow(card, 12.0, 18.0, 48);
            surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                16.0,
                card.x + 20.0,
                card.y + 34.0,
                DaylightTheme::INK,
                "Local Time-Machine Version History",
            );

            let entries = match app.tabs.get(app.active_tab) {
                Some(StudioTab::Word(w)) => w.snapshots.list().to_vec(),
                _ => Vec::new(),
            };
            let mut sy = card.y + 56.0;
            if entries.is_empty() {
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.5,
                    card.x + 20.0,
                    sy + 20.0,
                    DaylightTheme::INK_SUB,
                    "Switch to a Word (.docx) tab to capture & restore snapshots.",
                );
            } else {
                for snap in entries.iter().rev().take(5) {
                    let row = Rect::new(card.x + 18.0, sy, card.w - 36.0, 38.0);
                    surface.fill_rounded_rect(row, 6.0, DaylightTheme::SIDEBAR);
                    surface.stroke_rounded_rect(row, 6.0, 1.0, DaylightTheme::BORDER);
                    surface.draw_text_clipped(
                        &mut app.fonts,
                        FontSlot::SansBold,
                        12.0,
                        row.x + 12.0,
                        row.y + 23.0,
                        row.w - 120.0,
                        DaylightTheme::INK,
                        &format!("#{} · {} ({} words)", snap.id, snap.label, snap.word_count),
                    );
                    surface.draw_text(
                        &mut app.fonts,
                        FontSlot::SansBold,
                        11.5,
                        row.x + row.w - 74.0,
                        row.y + 23.0,
                        DaylightTheme::WORD_ACCENT,
                        "Restore",
                    );
                    app.hit_regions.push(HitRegion {
                        rect: row,
                        action: UiAction::RestoreSnapshot(snap.id),
                    });
                    sy += 44.0;
                }
            }

            paint_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                card,
                Some(("+ Capture Snapshot", UiAction::ExecCommand(CommandId::CaptureSnapshot))),
                "Close",
            );
        }
        Some(ActiveModal::About) => {
            let mw = 480.0f32.min(screen.w - 32.0);
            let mh = 268.0;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);
            block_backdrop_inside(&mut app.hit_regions, card);

            surface.draw_paper_shadow(card, 12.0, 18.0, 48);
            surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_logo_64(card.x as i32 + 24, card.y as i32 + 24);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                19.0,
                card.x + 102.0,
                card.y + 46.0,
                DaylightTheme::INK,
                "RustPad Studio v1.0.0",
            );
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansRegular,
                12.5,
                card.x + 102.0,
                card.y + 68.0,
                DaylightTheme::INK_SUB,
                "100% Pure-Rust Native Word (.docx) & PDF (.pdf) Suite",
            );

            let lines = [
                "Author:   Nour Yahyaoui",
                "GitHub:   https://github.com/Nour-yahyaoui/word",
                "Email:    nourryahyaoui@gmail.com",
                "Engine:   9 Pure-Rust Crates · Zero WebView · MIT License",
            ];
            for (i, line) in lines.iter().enumerate() {
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::MonoRegular,
                    11.5,
                    card.x + 24.0,
                    card.y + 114.0 + (i as f32) * 21.0,
                    Color::rgb(30, 41, 59),
                    line,
                );
            }

            paint_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                card,
                None,
                "Close",
            );
        }
        None => {}
    }
}

fn block_backdrop_inside(hit_regions: &mut Vec<HitRegion>, card: Rect) {
    // Consume clicks inside the modal body so they don't fall through to the backdrop close region
    hit_regions.push(HitRegion {
        rect: card,
        action: UiAction::FindFocusField(false),
    });
}

#[allow(clippy::too_many_arguments)]
fn paint_single_input_modal(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    screen: Rect,
    title: &str,
    subtitle: &str,
    val: &str,
    confirm_label: &str,
) {
    let mw = 440.0f32.min(screen.w - 32.0);
    let mh = 206.0;
    let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);
    block_backdrop_inside(hit_regions, card);

    surface.draw_paper_shadow(card, 12.0, 18.0, 48);
    surface.fill_rounded_rect(card, 12.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(card, 12.0, 1.0, DaylightTheme::BORDER_STRONG);

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        16.0,
        card.x + 20.0,
        card.y + 34.0,
        DaylightTheme::INK,
        title,
    );
    surface.draw_text(
        fonts,
        FontSlot::SansRegular,
        12.0,
        card.x + 20.0,
        card.y + 56.0,
        DaylightTheme::INK_SUB,
        subtitle,
    );

    let box_r = Rect::new(card.x + 20.0, card.y + 72.0, card.w - 40.0, 38.0);
    surface.fill_rounded_rect(box_r, 6.0, DaylightTheme::SIDEBAR);
    surface.stroke_rounded_rect(box_r, 6.0, 1.5, DaylightTheme::WORD_ACCENT);
    surface.draw_text_clipped(
        fonts,
        FontSlot::MonoRegular,
        13.5,
        box_r.x + 10.0,
        box_r.y + 24.0,
        box_r.w - 20.0,
        DaylightTheme::INK,
        val,
    );

    paint_modal_footer_buttons(surface, fonts, hit_regions, card, None, confirm_label);
}

fn paint_modal_footer_buttons(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    card: Rect,
    extra: Option<(&str, UiAction)>,
    confirm_label: &str,
) {
    let by = card.y + card.h - 46.0;
    if let Some((lbl, act)) = extra {
        let lw = fonts.measure_text(FontSlot::SansBold, lbl, 12.0) + 22.0;
        let er = Rect::new(card.x + 20.0, by, lw, 32.0);
        surface.fill_rounded_rect(er, 6.0, DaylightTheme::SIDEBAR);
        surface.stroke_rounded_rect(er, 6.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            12.0,
            er.x + 11.0,
            er.y + 20.5,
            DaylightTheme::INK,
            lbl,
        );
        hit_regions.push(HitRegion {
            rect: er,
            action: act,
        });
    }

    let cw = fonts.measure_text(FontSlot::SansBold, confirm_label, 12.0) + 28.0;
    let confirm_r = Rect::new(card.x + card.w - cw - 20.0, by, cw, 32.0);
    let cancel_r = Rect::new(confirm_r.x - 84.0, by, 76.0, 32.0);

    surface.fill_rounded_rect(cancel_r, 6.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(cancel_r, 6.0, 1.0, DaylightTheme::BORDER);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        12.0,
        cancel_r.x + 18.0,
        cancel_r.y + 20.5,
        DaylightTheme::INK_SUB,
        "Cancel",
    );
    hit_regions.push(HitRegion {
        rect: cancel_r,
        action: UiAction::ModalClose,
    });

    surface.fill_rounded_rect(confirm_r, 6.0, DaylightTheme::WORD_ACCENT);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        12.0,
        confirm_r.x + 14.0,
        confirm_r.y + 20.5,
        DaylightTheme::PAPER,
        confirm_label,
    );
    hit_regions.push(HitRegion {
        rect: confirm_r,
        action: UiAction::ModalConfirm,
    });
}
