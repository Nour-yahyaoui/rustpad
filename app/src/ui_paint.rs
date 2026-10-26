//! Master "Architectural Paper & Linen" Native Studio GUI Painter:
//! Unified Header Bar, Asymmetric Editorial Home Launchpad, Drag-Resizable Split Workspace,
//! Pluggable Right-Side AI Agent Studio Dock, Floating Style/Font Dropdowns, Modals & Status Bar.

use crate::commands::{filter_commands, CommandId};
use crate::pdf_view::paint_pdf_tab;
use crate::state::{
    ActiveModal, AiInputField, DragInteraction, FileModalKind, HitRegion, RustPadApp, StudioTab,
    ToolbarDropdown, UiAction,
};
use crate::word_view::paint_word_tab;
use doc_model::{FontFamily, ParagraphStyle, TemplateKind};
use power_tools::{AiProviderPreset, AiTask};
use rustpad_render::{Color, DaylightTheme, FontBook, FontSlot, Rect, Surface, UiIcon};

pub fn render_app(app: &mut RustPadApp, surface: &mut Surface) {
    app.hit_regions.clear();
    surface.reset_clip();
    surface.clear(DaylightTheme::DESK_BG);

    let s = app.ui_scale.clamp(0.85, 1.6);
    let win_w = surface.width as f32;
    let win_h = surface.height as f32;
    let header_h = (48.0 * s).round();
    let status_h = (30.0 * s).round();

    // 1. Unified Studio Header Bar
    paint_header_bar(app, surface, s, Rect::new(0.0, 0.0, win_w, header_h));

    // 2. Main Workspace + Optional Right-Side Pluggable AI Agent Dock
    let full_work = Rect::new(
        0.0,
        header_h,
        win_w,
        (win_h - header_h - status_h).max(40.0),
    );

    let show_ai_dock = app.ai_dock.open && full_work.w >= 620.0;
    let ai_w = if show_ai_dock {
        (app.ai_dock.width * s).clamp(264.0, full_work.w * 0.46).round()
    } else {
        0.0
    };
    let work = Rect::new(full_work.x, full_work.y, (full_work.w - ai_w).max(120.0), full_work.h);

    let active_idx = app.active_tab;
    if let Some(sec_idx) = app
        .split_secondary_tab
        .filter(|&sec| sec < app.tabs.len() && sec != active_idx && work.w >= 560.0)
    {
        let ratio = app.split_ratio.clamp(0.24, 0.76);
        let left_w = ((work.w - 5.0) * ratio).floor();
        let left_rect = Rect::new(work.x, work.y, left_w, work.h);
        let right_rect = Rect::new(work.x + left_w + 5.0, work.y, work.w - left_w - 5.0, work.h);

        paint_tab_pane(app, surface, active_idx, true, left_rect);
        paint_tab_pane(app, surface, sec_idx, false, right_rect);

        // Draggable Split Workspace Divider
        let div_vis = Rect::new(work.x + left_w, work.y, 5.0, work.h);
        let div_hit = Rect::new(work.x + left_w - 3.0, work.y, 11.0, work.h);
        let hov = div_hit.contains(app.mouse_x, app.mouse_y);
        surface.fill_rect(
            div_vis,
            if hov {
                DaylightTheme::RUST_ORANGE
            } else {
                DaylightTheme::BORDER_STRONG
            },
        );
        app.hit_regions.push(HitRegion::splitter(
            div_hit,
            UiAction::BeginDrag(DragInteraction::WorkspaceSplit {
                work_x: work.x,
                work_w: work.w,
            }),
        ));
    } else {
        paint_tab_pane(app, surface, active_idx, true, work);
    }

    if show_ai_dock {
        let dock_rect = Rect::new(full_work.x + work.w, full_work.y, ai_w, full_work.h);
        app.ai_dock_rect = Some(dock_rect);
        paint_ai_dock(app, surface, s, dock_rect);

        let split_hit = Rect::new(dock_rect.x - 4.0, dock_rect.y, 8.0, dock_rect.h);
        if split_hit.contains(app.mouse_x, app.mouse_y) {
            surface.fill_rect(
                Rect::new(dock_rect.x - 1.5, dock_rect.y, 3.0, dock_rect.h),
                DaylightTheme::AI_ACCENT,
            );
        }
        app.hit_regions.push(HitRegion::splitter(
            split_hit,
            UiAction::BeginDrag(DragInteraction::AiDockSplitter {
                window_right: win_w,
            }),
        ));
    } else {
        app.ai_dock_rect = None;
    }

    // 3. Bottom Studio Status Bar (with UI Scale & Paper Zoom steppers)
    paint_status_bar(
        app,
        surface,
        s,
        Rect::new(0.0, win_h - status_h, win_w, status_h),
    );

    // 4. Floating Toolbar Dropdown Popover (Style / Font Family)
    if let Some(dd) = app.open_dropdown {
        paint_toolbar_dropdown(app, surface, s, dd, win_w, win_h);
    }

    // 5. Modal Overlay
    if app.modal.is_some() {
        paint_active_modal(app, surface, s, Rect::new(0.0, 0.0, win_w, win_h));
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
    let s = app.ui_scale;
    let w_sb_open = app.word_sidebar_open;
    let w_sb_w = app.word_sidebar_w;
    let p_l_open = app.pdf_left_open;
    let p_l_w = app.pdf_left_w;
    let p_r_open = app.pdf_right_open;
    let p_r_w = app.pdf_right_w;

    let recent = app.recent_files.clone();
    let (fonts, spell, hits, tabs) = (
        &mut app.fonts,
        &app.spellchecker,
        &mut app.hit_regions,
        &mut app.tabs,
    );
    let Some(tab_mut) = tabs.get_mut(tab_idx) else {
        return;
    };
    match tab_mut {
        StudioTab::HomeHub => {
            paint_home_hub(surface, fonts, hits, mx, my, s, area, &recent);
        }
        StudioTab::Word(w) => {
            paint_word_tab(
                surface,
                fonts,
                spell,
                hits,
                tab_idx,
                is_focused_pane,
                blink,
                mx,
                my,
                s,
                w_sb_open,
                w_sb_w,
                area,
                w,
            );
        }
        StudioTab::Pdf(p) => {
            paint_pdf_tab(
                surface,
                fonts,
                hits,
                tab_idx,
                is_focused_pane,
                blink,
                mx,
                my,
                s,
                p_l_open,
                p_l_w,
                p_r_open,
                p_r_w,
                area,
                p,
            );
        }
    }
}

fn paint_header_bar(app: &mut RustPadApp, surface: &mut Surface, s: f32, bar: Rect) {
    surface.fill_rect(bar, DaylightTheme::PAPER);
    surface.fill_rect(
        Rect::new(bar.x, bar.y + bar.h - 1.0, bar.w, 1.0),
        DaylightTheme::BORDER,
    );

    let logo_y = ((bar.h - 36.0) * 0.5).max(4.0) as i32;
    surface.draw_logo_36(12, logo_y);
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        14.5 * s,
        54.0,
        bar.h * 0.62,
        DaylightTheme::INK,
        "RustPad",
    );
    app.hit_regions.push(HitRegion::button(
        Rect::new(8.0, 4.0, 114.0 * s, bar.h - 8.0),
        UiAction::OpenHomeTab,
    ));

    let right_reserve = 436.0 * s;
    let tabs_max_x = (bar.w - right_reserve).max(160.0);

    let tab_h = (32.0 * s).round();
    let tab_y = ((bar.h - tab_h) * 0.5).round();
    let mut tx = 118.0 * s;

    for idx in 0..app.tabs.len() {
        let title = app.tabs[idx].title();
        let (badge, is_pdf) = app.tabs[idx].badge();
        let active = idx == app.active_tab;
        let tw = app
            .fonts
            .measure_text(FontSlot::SansBold, &title, 11.8 * s)
            .min(148.0 * s);
        let tab_w = if idx == 0 {
            (68.0 * s).round()
        } else {
            (tw + 66.0 * s).clamp(112.0 * s, 212.0 * s).round()
        };
        if tx + tab_w > tabs_max_x {
            break;
        }
        let tab_r = Rect::new(tx, tab_y, tab_w, tab_h);
        let hov = tab_r.contains(app.mouse_x, app.mouse_y);

        surface.fill_rounded_rect(
            tab_r,
            6.0,
            if active {
                DaylightTheme::PAPER
            } else if hov {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::SIDEBAR
            },
        );
        surface.stroke_rounded_rect(
            tab_r,
            6.0,
            if active { 1.4 } else { 1.0 },
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
            let hw = app.fonts.measure_text(FontSlot::SansBold, "Home", 11.8 * s);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                11.8 * s,
                tab_r.x + (tab_w - hw) * 0.5,
                tab_r.y + tab_h * 0.65,
                if active {
                    DaylightTheme::RUST_ORANGE
                } else {
                    DaylightTheme::INK_SUB
                },
                "Home",
            );
            app.hit_regions
                .push(HitRegion::button(tab_r, UiAction::SwitchTab(idx)));
        } else {
            let pill = Rect::new(tab_r.x + 7.0 * s, tab_r.y + (tab_h - 16.0 * s) * 0.5, 36.0 * s, 16.0 * s);
            surface.fill_rounded_rect(
                pill,
                3.0,
                if is_pdf {
                    DaylightTheme::PDF_ACCENT
                } else {
                    DaylightTheme::WORD_ACCENT
                },
            );
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                9.2 * s,
                pill.x + 5.0 * s,
                pill.y + 11.5 * s,
                DaylightTheme::PAPER,
                badge,
            );
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::SansBold,
                11.8 * s,
                pill.x + pill.w + 6.0 * s,
                tab_r.y + tab_h * 0.65,
                tab_r.w - 68.0 * s,
                DaylightTheme::INK,
                &title,
            );
            let close_r = Rect::new(
                tab_r.x + tab_r.w - 22.0 * s,
                tab_r.y + (tab_h - 16.0 * s) * 0.5,
                16.0 * s,
                16.0 * s,
            );
            surface.draw_icon(
                UiIcon::Close,
                close_r.x + close_r.w * 0.5,
                close_r.y + close_r.h * 0.5,
                8.5 * s,
                DaylightTheme::INK_MUTED,
            );
            app.hit_regions
                .push(HitRegion::button(tab_r, UiAction::SwitchTab(idx)));
            app.hit_regions
                .push(HitRegion::button(close_r, UiAction::CloseTab(idx)));
        }
        tx += tab_w + 5.0 * s;
    }

    // '+' New Blank Word Tab
    if tx + 32.0 * s <= tabs_max_x {
        let plus_r = Rect::new(tx, tab_y, 30.0 * s, tab_h);
        let hov = plus_r.contains(app.mouse_x, app.mouse_y);
        surface.fill_rounded_rect(
            plus_r,
            6.0,
            if hov {
                DaylightTheme::HOVER
            } else {
                DaylightTheme::SIDEBAR
            },
        );
        surface.stroke_rounded_rect(plus_r, 6.0, 1.0, DaylightTheme::BORDER);
        surface.draw_icon(
            UiIcon::Plus,
            plus_r.x + plus_r.w * 0.5,
            plus_r.y + plus_r.h * 0.5,
            11.0 * s,
            DaylightTheme::INK_SUB,
        );
        app.hit_regions.push(HitRegion::button(
            plus_r,
            UiAction::ExecCommand(CommandId::NewWordBlank),
        ));
    }

    // Right Studio Action Cluster (`AI Studio`, `Ctrl+K`, `Split`, `Open`, `Save`)
    let mut rx = (bar.w - 424.0 * s).max(tx + 8.0 * s);

    // 1. Pluggable AI Studio Dock Toggle (`Ctrl+J`)
    let ai_w = (94.0 * s).round();
    let ai_r = Rect::new(rx, tab_y, ai_w, tab_h);
    let ai_on = app.ai_dock.open;
    let ai_hov = ai_r.contains(app.mouse_x, app.mouse_y);
    surface.fill_rounded_rect(
        ai_r,
        6.0,
        if ai_on {
            DaylightTheme::AI_ACCENT
        } else if ai_hov {
            DaylightTheme::AI_SOFT
        } else {
            DaylightTheme::SIDEBAR
        },
    );
    surface.stroke_rounded_rect(
        ai_r,
        6.0,
        1.0,
        if ai_on || ai_hov {
            DaylightTheme::AI_ACCENT
        } else {
            DaylightTheme::BORDER_STRONG
        },
    );
    let ai_fg = if ai_on {
        DaylightTheme::PAPER
    } else {
        DaylightTheme::AI_ACCENT
    };
    surface.draw_icon(
        UiIcon::Sparkles,
        ai_r.x + 14.0 * s,
        ai_r.y + tab_h * 0.5,
        12.5 * s,
        ai_fg,
    );
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        11.5 * s,
        ai_r.x + 25.0 * s,
        ai_r.y + tab_h * 0.65,
        ai_fg,
        "AI Studio",
    );
    app.hit_regions.push(HitRegion::button(
        ai_r,
        UiAction::ExecCommand(CommandId::ToggleAiDock),
    ));
    rx += ai_w + 6.0 * s;

    // 2. Command Palette Search Trigger (`Ctrl+K`)
    let pal_w = (128.0 * s).round();
    let pal_r = Rect::new(rx, tab_y, pal_w, tab_h);
    let pal_hov = pal_r.contains(app.mouse_x, app.mouse_y);
    surface.fill_rounded_rect(
        pal_r,
        6.0,
        if pal_hov {
            DaylightTheme::HOVER
        } else {
            DaylightTheme::SIDEBAR
        },
    );
    surface.stroke_rounded_rect(pal_r, 6.0, 1.0, DaylightTheme::BORDER);
    surface.draw_icon(
        UiIcon::Search,
        pal_r.x + 13.0 * s,
        pal_r.y + tab_h * 0.5,
        11.5 * s,
        DaylightTheme::INK_MUTED,
    );
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansRegular,
        11.2 * s,
        pal_r.x + 24.0 * s,
        pal_r.y + tab_h * 0.65,
        DaylightTheme::INK_SUB,
        "Tools  Ctrl+K",
    );
    app.hit_regions
        .push(HitRegion::button(pal_r, UiAction::OpenCommandPalette));
    rx += pal_w + 6.0 * s;

    let split_on = app.split_secondary_tab.is_some();
    for (lbl, cmd, active, primary) in [
        ("Split", CommandId::ToggleSplitView, split_on, false),
        ("Open", CommandId::OpenFile, false, false),
        ("Save", CommandId::SaveFile, false, true),
    ] {
        let bw = (56.0 * s).round();
        let br = Rect::new(rx, tab_y, bw, tab_h);
        let hov = br.contains(app.mouse_x, app.mouse_y);
        if primary {
            surface.fill_rounded_rect(
                br,
                6.0,
                if hov {
                    DaylightTheme::RUST_ORANGE
                } else {
                    DaylightTheme::WORD_ACCENT
                },
            );
            let lw = app.fonts.measure_text(FontSlot::SansBold, lbl, 11.8 * s);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                11.8 * s,
                br.x + (bw - lw) * 0.5,
                br.y + tab_h * 0.65,
                DaylightTheme::PAPER,
                lbl,
            );
        } else {
            surface.fill_rounded_rect(
                br,
                6.0,
                if active {
                    DaylightTheme::WORD_SOFT
                } else if hov {
                    DaylightTheme::HOVER
                } else {
                    DaylightTheme::PAPER
                },
            );
            surface.stroke_rounded_rect(br, 6.0, 1.0, DaylightTheme::BORDER);
            let lw = app.fonts.measure_text(FontSlot::SansBold, lbl, 11.8 * s);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                11.8 * s,
                br.x + (bw - lw) * 0.5,
                br.y + tab_h * 0.65,
                if active {
                    DaylightTheme::WORD_ACCENT
                } else {
                    DaylightTheme::INK
                },
                lbl,
            );
        }
        app.hit_regions
            .push(HitRegion::button(br, UiAction::ExecCommand(cmd)));
        rx += bw + 5.0 * s;
    }
}

/// Resizable Right-Side Pluggable AI Agent Studio Dock (`Ollama`, `nour-code`, `LM Studio`, `OpenAI`, `Claude`).
fn paint_ai_dock(app: &mut RustPadApp, surface: &mut Surface, s: f32, dock: Rect) {
    surface.set_clip(dock);
    surface.fill_rect(dock, DaylightTheme::SIDEBAR);
    surface.fill_rect(Rect::new(dock.x, dock.y, 1.0, dock.h), DaylightTheme::BORDER);

    let pad = 14.0 * s;
    let inner_w = (dock.w - pad * 2.0).max(160.0);
    let mut y = dock.y + 14.0 * s;

    // Header
    surface.draw_icon(
        UiIcon::Sparkles,
        dock.x + pad + 7.0 * s,
        y + 6.0 * s,
        13.0 * s,
        DaylightTheme::AI_ACCENT,
    );
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        12.5 * s,
        dock.x + pad + 20.0 * s,
        y + 11.0 * s,
        DaylightTheme::INK,
        "AI Agent Studio",
    );
    let close_r = Rect::new(dock.x + dock.w - pad - 22.0 * s, y - 3.0 * s, 22.0 * s, 22.0 * s);
    surface.draw_icon(
        UiIcon::Close,
        close_r.x + close_r.w * 0.5,
        close_r.y + close_r.h * 0.5,
        10.0 * s,
        DaylightTheme::INK_MUTED,
    );
    app.hit_regions.push(HitRegion::button(
        close_r,
        UiAction::ExecCommand(CommandId::ToggleAiDock),
    ));
    y += 22.0 * s;

    surface.draw_text_clipped(
        &mut app.fonts,
        FontSlot::SansRegular,
        10.8 * s,
        dock.x + pad,
        y + 8.0 * s,
        inner_w,
        DaylightTheme::INK_MUTED,
        "Local HTTP (Ollama, nour-code) or Cloud API · Ctrl+V to paste",
    );
    y += 18.0 * s;

    // 1. Provider Preset Chips (wrapped cleanly)
    let mut px = dock.x + pad;
    let chip_h = (23.0 * s).round();
    for preset in AiProviderPreset::ALL {
        let lbl = match preset {
            AiProviderPreset::OllamaLocal => "Ollama",
            AiProviderPreset::NourCodeLocal => "nour-code",
            AiProviderPreset::LmStudioLocal => "LM Studio",
            AiProviderPreset::OpenAiCompatible => "OpenAI/Cloud",
            AiProviderPreset::AnthropicClaude => "Claude",
        };
        let tw = app.fonts.measure_text(FontSlot::SansBold, lbl, 10.4 * s);
        let cw = (tw + 13.0 * s).round();
        if px + cw > dock.x + dock.w - pad && px > dock.x + pad {
            px = dock.x + pad;
            y += chip_h + 4.0 * s;
        }
        let r = Rect::new(px, y, cw, chip_h);
        let active = app.ai_dock.config.preset == *preset;
        let hov = r.contains(app.mouse_x, app.mouse_y);
        surface.fill_rounded_rect(
            r,
            4.0,
            if active {
                DaylightTheme::AI_ACCENT
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
                DaylightTheme::AI_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );
        surface.draw_text(
            &mut app.fonts,
            FontSlot::SansBold,
            10.4 * s,
            r.x + 6.5 * s,
            r.y + chip_h * 0.69,
            if active {
                DaylightTheme::PAPER
            } else {
                DaylightTheme::INK
            },
            lbl,
        );
        app.hit_regions
            .push(HitRegion::button(r, UiAction::AiSelectProvider(*preset)));
        px += cw + 4.0 * s;
    }
    y += chip_h + 10.0 * s;

    // 2. Editable Endpoint URL, Model (+ Detect Local button), and API Key Fields
    let ep_val = app.ai_dock.config.endpoint_url.clone();
    y = paint_ai_field(
        app,
        surface,
        s,
        dock.x + pad,
        y,
        inner_w,
        "ENDPOINT URL",
        &ep_val,
        "http://127.0.0.1:11434/v1/chat/completions",
        AiInputField::EndpointUrl,
    );

    // Model + Detect button row
    let det_w = (72.0 * s).round();
    let model_val = app.ai_dock.config.model.clone();
    let _ = paint_ai_field(
        app,
        surface,
        s,
        dock.x + pad,
        y,
        inner_w - det_w - 6.0 * s,
        "MODEL",
        &model_val,
        "qwen2.5:7b / nour-code / gpt-4o-mini",
        AiInputField::Model,
    );
    let det_r = Rect::new(
        dock.x + pad + inner_w - det_w,
        y + 13.0 * s,
        det_w,
        (28.0 * s).round(),
    );
    let det_hov = det_r.contains(app.mouse_x, app.mouse_y);
    surface.fill_rounded_rect(
        det_r,
        4.0,
        if det_hov {
            DaylightTheme::AI_SOFT
        } else {
            DaylightTheme::PAPER
        },
    );
    surface.stroke_rounded_rect(det_r, 4.0, 1.0, DaylightTheme::BORDER_STRONG);
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        10.2 * s,
        det_r.x + 8.0 * s,
        det_r.y + det_r.h * 0.67,
        DaylightTheme::AI_ACCENT,
        "Detect Local",
    );
    app.hit_regions
        .push(HitRegion::button(det_r, UiAction::AiDetectOllamaModels));
    y += 47.0 * s;

    // Discovered local models quick chips (if any)
    let disc = app.ai_dock.discovered_models.clone();
    if !disc.is_empty() {
        let mut mx = dock.x + pad;
        for m in disc.iter().take(3) {
            let tw = app.fonts.measure_text(FontSlot::MonoRegular, m, 9.8 * s);
            let mw = (tw + 12.0 * s).min(inner_w);
            if mx + mw > dock.x + pad + inner_w {
                break;
            }
            let mr = Rect::new(mx, y, mw, 20.0 * s);
            surface.fill_rounded_rect(mr, 3.0, DaylightTheme::AI_SOFT);
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::MonoRegular,
                9.8 * s,
                mr.x + 6.0 * s,
                mr.y + 14.0 * s,
                mw - 10.0 * s,
                DaylightTheme::AI_ACCENT,
                m,
            );
            app.hit_regions.push(HitRegion::button(
                mr,
                UiAction::AiPickDiscoveredModel(m.clone()),
            ));
            mx += mw + 4.0 * s;
        }
        y += 25.0 * s;
    }

    if !app.ai_dock.config.preset.is_local() || !app.ai_dock.config.api_key.is_empty() {
        let masked = if app.ai_dock.config.api_key.is_empty() {
            String::new()
        } else {
            let k = &app.ai_dock.config.api_key;
            let tail: String = k
                .chars()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!("••••••••{tail}")
        };
        y = paint_ai_field(
            app,
            surface,
            s,
            dock.x + pad,
            y,
            inner_w,
            "API KEY (SESSION MEMORY ONLY)",
            &masked,
            "sk-... or set OPENAI_API_KEY / ANTHROPIC_API_KEY",
            AiInputField::ApiKey,
        );
    }

    // 3. Editorial Task Selector
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        9.8 * s,
        dock.x + pad,
        y + 9.0 * s,
        DaylightTheme::INK_MUTED,
        "ACTION ON ACTIVE DOCUMENT",
    );
    y += 14.0 * s;

    let mut tx = dock.x + pad;
    for task in AiTask::ALL {
        let lbl = task.short_label();
        let tw = app.fonts.measure_text(FontSlot::SansBold, lbl, 10.6 * s);
        let bw = (tw + 14.0 * s).round();
        if tx + bw > dock.x + pad + inner_w && tx > dock.x + pad {
            tx = dock.x + pad;
            y += chip_h + 4.0 * s;
        }
        let r = Rect::new(tx, y, bw, chip_h);
        let active = app.ai_dock.selected_task == *task;
        let hov = r.contains(app.mouse_x, app.mouse_y);
        surface.fill_rounded_rect(
            r,
            4.0,
            if active {
                DaylightTheme::AI_SOFT
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
                DaylightTheme::AI_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );
        surface.draw_text(
            &mut app.fonts,
            FontSlot::SansBold,
            10.6 * s,
            r.x + 7.0 * s,
            r.y + chip_h * 0.69,
            if active {
                DaylightTheme::AI_ACCENT
            } else {
                DaylightTheme::INK
            },
            lbl,
        );
        app.hit_regions
            .push(HitRegion::button(r, UiAction::AiSelectTask(*task)));
        tx += bw + 4.0 * s;
    }
    y += chip_h + 9.0 * s;

    let prompt_val = app.ai_dock.custom_prompt.clone();
    y = paint_ai_field(
        app,
        surface,
        s,
        dock.x + pad,
        y,
        inner_w,
        "FOCUS INSTRUCTION (OPTIONAL)",
        &prompt_val,
        "e.g. Add benchmark comparison section & conclusion",
        AiInputField::CustomPrompt,
    );

    // 4. Primary Non-Blocking Run / Cancel Button
    let run_h = (32.0 * s).round();
    let run_r = Rect::new(dock.x + pad, y, inner_w, run_h);
    let busy = app.ai_dock.is_busy();
    surface.fill_rounded_rect(
        run_r,
        5.0,
        if busy {
            DaylightTheme::RUST_ORANGE
        } else {
            DaylightTheme::AI_ACCENT
        },
    );
    let run_lbl = if busy {
        "Cancel · Agent Generating…"
    } else {
        "Run AI Agent (Enter)"
    };
    let rlw = app.fonts.measure_text(FontSlot::SansBold, run_lbl, 11.8 * s);
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        11.8 * s,
        run_r.x + (run_r.w - rlw) * 0.5,
        run_r.y + run_h * 0.66,
        DaylightTheme::PAPER,
        run_lbl,
    );
    app.hit_regions.push(HitRegion::button(
        run_r,
        if busy {
            UiAction::AiCancelTask
        } else {
            UiAction::AiRunCurrentTask
        },
    ));
    y += run_h + 7.0 * s;

    // Status line
    let st_col = if app.ai_dock.is_error {
        DaylightTheme::PDF_ACCENT
    } else {
        DaylightTheme::INK_SUB
    };
    let st_line = app.ai_dock.status_line.clone();
    surface.draw_text_clipped(
        &mut app.fonts,
        FontSlot::SansRegular,
        10.5 * s,
        dock.x + pad,
        y + 9.0 * s,
        inner_w,
        st_col,
        &st_line,
    );
    y += 16.0 * s;

    // 5. Output Preview Box + 1-Click Apply Buttons (`Insert Below`, `Replace`, `Copy`)
    let bottom_btns_h = (34.0 * s).round();
    let out_h = (dock.y + dock.h - y - bottom_btns_h - 14.0 * s).max(56.0);
    let out_r = Rect::new(dock.x + pad, y, inner_w, out_h);
    surface.fill_rounded_rect(out_r, 5.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(out_r, 5.0, 1.0, DaylightTheme::BORDER);

    surface.set_clip(Rect::new(out_r.x + 2.0, out_r.y + 2.0, out_r.w - 4.0, out_r.h - 4.0));
    let out_txt = app.ai_dock.output_text.clone();
    if out_txt.is_empty() {
        surface.draw_text_clipped(
            &mut app.fonts,
            FontSlot::SansRegular,
            11.2 * s,
            out_r.x + 10.0 * s,
            out_r.y + 22.0 * s,
            out_r.w - 20.0 * s,
            DaylightTheme::INK_MUTED,
            "Agent response will appear here for inspection before touching your document.",
        );
    } else {
        let mut oy = out_r.y + 18.0 * s - app.ai_dock.output_scroll_y;
        let line_px = 11.4 * s;
        for raw_ln in out_txt.lines() {
            if raw_ln.trim().is_empty() {
                oy += 8.0 * s;
                continue;
            }
            // Word-wrap output inside preview box
            let mut cur = String::new();
            for w_tok in raw_ln.split_inclusive(' ') {
                let cand = format!("{cur}{w_tok}");
                if !cur.is_empty()
                    && app.fonts.measure_text(FontSlot::SansRegular, &cand, line_px)
                        > out_r.w - 20.0 * s
                {
                    if oy >= out_r.y && oy <= out_r.y + out_r.h + 12.0 {
                        surface.draw_text(
                            &mut app.fonts,
                            FontSlot::SansRegular,
                            line_px,
                            out_r.x + 10.0 * s,
                            oy,
                            DaylightTheme::INK,
                            cur.trim_end(),
                        );
                    }
                    oy += 15.5 * s;
                    cur = w_tok.to_string();
                } else {
                    cur = cand;
                }
            }
            if !cur.is_empty() {
                if oy >= out_r.y && oy <= out_r.y + out_r.h + 12.0 {
                    surface.draw_text(
                        &mut app.fonts,
                        FontSlot::SansRegular,
                        line_px,
                        out_r.x + 10.0 * s,
                        oy,
                        DaylightTheme::INK,
                        cur.trim_end(),
                    );
                }
                oy += 16.0 * s;
            }
        }
    }
    surface.set_clip(dock);

    let act_y = out_r.y + out_r.h + 7.0 * s;
    let col3 = (inner_w - 8.0 * s) / 3.0;
    for (idx, (lbl, act, primary)) in [
        (
            "Insert Below",
            UiAction::AiApplyOutput {
                replace_current: false,
            },
            true,
        ),
        (
            "Replace",
            UiAction::AiApplyOutput {
                replace_current: true,
            },
            false,
        ),
        ("Copy", UiAction::AiCopyOutput, false),
    ]
    .into_iter()
    .enumerate()
    {
        let br = Rect::new(
            dock.x + pad + (idx as f32) * (col3 + 4.0 * s),
            act_y,
            col3,
            (28.0 * s).round(),
        );
        surface.fill_rounded_rect(
            br,
            4.0,
            if primary {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::PAPER
            },
        );
        if !primary {
            surface.stroke_rounded_rect(br, 4.0, 1.0, DaylightTheme::BORDER_STRONG);
        }
        let lw = app.fonts.measure_text(FontSlot::SansBold, lbl, 10.8 * s);
        surface.draw_text(
            &mut app.fonts,
            FontSlot::SansBold,
            10.8 * s,
            br.x + (br.w - lw) * 0.5,
            br.y + br.h * 0.67,
            if primary {
                DaylightTheme::PAPER
            } else {
                DaylightTheme::INK
            },
            lbl,
        );
        app.hit_regions.push(HitRegion::button(br, act));
    }

    surface.reset_clip();
}

#[allow(clippy::too_many_arguments)]
fn paint_ai_field(
    app: &mut RustPadApp,
    surface: &mut Surface,
    s: f32,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    placeholder: &str,
    field: AiInputField,
) -> f32 {
    surface.draw_text(
        &mut app.fonts,
        FontSlot::SansBold,
        9.5 * s,
        x,
        y + 9.0 * s,
        DaylightTheme::INK_MUTED,
        label,
    );
    let bh = (28.0 * s).round();
    let box_r = Rect::new(x, y + 13.0 * s, w, bh);
    let focused = app.ai_dock.focused_field == Some(field);
    surface.fill_rounded_rect(box_r, 4.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(
        box_r,
        4.0,
        if focused { 1.5 } else { 1.0 },
        if focused {
            DaylightTheme::AI_ACCENT
        } else {
            DaylightTheme::BORDER
        },
    );
    let disp_owned: String;
    let disp: &str = if value.is_empty() {
        placeholder
    } else if focused && app.cursor_blink_on {
        disp_owned = format!("{value}|");
        &disp_owned
    } else {
        value
    };
    surface.draw_text_clipped(
        &mut app.fonts,
        FontSlot::MonoRegular,
        10.5 * s,
        box_r.x + 8.0 * s,
        box_r.y + bh * 0.67,
        w - 16.0 * s,
        if value.is_empty() {
            DaylightTheme::INK_MUTED
        } else {
            DaylightTheme::INK
        },
        disp,
    );
    app.hit_regions
        .push(HitRegion::text(box_r, UiAction::AiFocusField(Some(field))));
    y + 47.0 * s
}

/// Asymmetric Editorial Home Launchpad (replaces the old 6-card SaaS template grid).
#[allow(clippy::too_many_arguments)]
fn paint_home_hub(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    s: f32,
    area: Rect,
    recent_files: &[std::path::PathBuf],
) {
    surface.fill_rect(area, DaylightTheme::CHROME);

    let max_w = (1140.0 * s).min(area.w - 48.0 * s).max(320.0);
    let ox = area.x + (area.w - max_w) * 0.5;
    let mut y = area.y + 34.0 * s;

    // Studio Masthead Rule
    surface.draw_text(
        fonts,
        FontSlot::MonoRegular,
        10.8 * s,
        ox,
        y,
        DaylightTheme::RUST_ORANGE,
        &format!(
            "RUSTPAD STUDIO v{} · PURE-RUST NATIVE DESKTOP · ZERO WEBVIEW",
            env!("CARGO_PKG_VERSION")
        ),
    );
    y += 12.0 * s;
    surface.fill_rect(Rect::new(ox, y, max_w, 1.0), DaylightTheme::BORDER_STRONG);
    y += 34.0 * s;

    let two_col = max_w >= 760.0;
    let left_w = if two_col {
        (max_w * 0.42).floor()
    } else {
        max_w
    };
    let right_x = if two_col {
        ox + left_w + 36.0 * s
    } else {
        ox
    };
    let right_w = if two_col {
        max_w - left_w - 36.0 * s
    } else {
        max_w
    };

    // Left Column: Editorial Headline + Primary Actions + Recent Workspace Ledger
    surface.draw_text(
        fonts,
        FontSlot::SerifRegular,
        28.0 * s,
        ox,
        y,
        DaylightTheme::INK,
        "Draft, inspect & sign.",
    );
    surface.draw_text_clipped(
        fonts,
        FontSlot::SansRegular,
        13.0 * s,
        ox,
        y + 24.0 * s,
        left_w,
        DaylightTheme::INK_SUB,
        "Author ECMA-376 Word (.docx) documents, edit ISO 32000 PDFs, and run local or cloud AI agents offline.",
    );
    surface.draw_text_clipped(
        fonts,
        FontSlot::MonoRegular,
        10.8 * s,
        ox,
        y + 46.0 * s,
        left_w,
        DaylightTheme::INK_MUTED,
        "Nour Yahyaoui · github.com/Nour-yahyaoui/rustpad",
    );

    let mut ly = y + 68.0 * s;
    let btn_h = (38.0 * s).round();
    let half_btn = (left_w - 8.0 * s) * 0.5;
    for (i, (lbl, cmd, fill, fg)) in [
        (
            "+ New Blank Word (.docx)",
            CommandId::NewWordBlank,
            DaylightTheme::WORD_ACCENT,
            DaylightTheme::PAPER,
        ),
        (
            "+ New Blank PDF (.pdf)",
            CommandId::NewPdfBlank,
            DaylightTheme::PDF_ACCENT,
            DaylightTheme::PAPER,
        ),
        (
            "Open File (.docx / .pdf / .md)",
            CommandId::OpenFile,
            DaylightTheme::PAPER,
            DaylightTheme::INK,
        ),
        (
            "Connect AI Agent (Ctrl+J)",
            CommandId::ToggleAiDock,
            DaylightTheme::AI_SOFT,
            DaylightTheme::AI_ACCENT,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let col = i % 2;
        let row = i / 2;
        let r = Rect::new(
            ox + (col as f32) * (half_btn + 8.0 * s),
            ly + (row as f32) * (btn_h + 8.0 * s),
            half_btn,
            btn_h,
        );
        surface.fill_rounded_rect(r, 5.0, fill);
        if fill == DaylightTheme::PAPER || fill == DaylightTheme::AI_SOFT {
            surface.stroke_rounded_rect(r, 5.0, 1.0, DaylightTheme::BORDER_STRONG);
        }
        let lw = fonts.measure_text(FontSlot::SansBold, lbl, 11.6 * s);
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansBold,
            11.6 * s,
            r.x + ((r.w - lw) * 0.5).max(10.0),
            r.y + btn_h * 0.64,
            r.w - 16.0,
            fg,
            lbl,
        );
        hit_regions.push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
    }
    ly += (btn_h + 8.0 * s) * 2.0 + 24.0 * s;

    // Recent Workspace Ledger (Hairline rows, crisp format tags)
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.5 * s,
        ox,
        ly,
        DaylightTheme::INK_MUTED,
        "RECENT WORKSPACE FILES",
    );
    ly += 10.0 * s;
    surface.fill_rect(Rect::new(ox, ly, left_w, 1.0), DaylightTheme::BORDER);
    ly += 4.0 * s;

    if recent_files.is_empty() {
        surface.draw_text(
            fonts,
            FontSlot::SansRegular,
            12.0 * s,
            ox,
            ly + 22.0 * s,
            DaylightTheme::INK_MUTED,
            "No local .docx or .pdf files in current directory yet.",
        );
    } else {
        for rf in recent_files.iter().take(6) {
            let row_h = (32.0 * s).round();
            if ly + row_h > area.y + area.h - 16.0 {
                break;
            }
            let r = Rect::new(ox, ly, left_w, row_h);
            if r.contains(mouse_x, mouse_y) {
                surface.fill_rounded_rect(r, 4.0, DaylightTheme::HOVER);
            }
            let is_pdf = rf
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false);
            let tag_r = Rect::new(r.x + 4.0 * s, r.y + (row_h - 17.0 * s) * 0.5, 38.0 * s, 17.0 * s);
            surface.fill_rounded_rect(
                tag_r,
                3.0,
                if is_pdf {
                    DaylightTheme::PDF_SOFT
                } else {
                    DaylightTheme::WORD_SOFT
                },
            );
            surface.draw_text(
                fonts,
                FontSlot::MonoRegular,
                9.5 * s,
                tag_r.x + 5.0 * s,
                tag_r.y + 12.2 * s,
                if is_pdf {
                    DaylightTheme::PDF_ACCENT
                } else {
                    DaylightTheme::WORD_ACCENT
                },
                if is_pdf { "PDF" } else { "DOCX" },
            );
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                12.0 * s,
                tag_r.x + tag_r.w + 10.0 * s,
                r.y + row_h * 0.66,
                left_w - 58.0 * s,
                DaylightTheme::INK,
                &rf.display().to_string(),
            );
            surface.fill_rect(
                Rect::new(r.x, r.y + row_h - 1.0, left_w, 1.0),
                DaylightTheme::BORDER,
            );
            hit_regions.push(HitRegion::button(r, UiAction::OpenRecentPath(rf.clone())));
            ly += row_h;
        }
    }

    // Right Column: Paper Miniature Starter Shelf
    let ry_start = if two_col { y - 14.0 * s } else { ly + 28.0 * s };
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        10.5 * s,
        right_x,
        ry_start,
        DaylightTheme::INK_MUTED,
        "PAPER TEMPLATES (.DOCX)",
    );

    let cols = if right_w >= 460.0 { 2 } else { 1 };
    let gap = 12.0 * s;
    let card_w = (right_w - gap * ((cols - 1) as f32)) / (cols as f32);
    let card_h = (104.0 * s).round();

    for (idx, tpl) in TemplateKind::ALL.iter().enumerate() {
        let c = idx % cols;
        let r_idx = idx / cols;
        let cx = right_x + (c as f32) * (card_w + gap);
        let cy = ry_start + 12.0 * s + (r_idx as f32) * (card_h + gap);
        if cy + card_h > area.y + area.h - 12.0 {
            break;
        }
        let card = Rect::new(cx, cy, card_w, card_h);
        let hov = card.contains(mouse_x, mouse_y);
        surface.fill_rounded_rect(
            card,
            5.0,
            if hov {
                DaylightTheme::PAPER
            } else {
                DaylightTheme::SIDEBAR
            },
        );
        surface.stroke_rounded_rect(
            card,
            5.0,
            1.0,
            if hov {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );

        // Crisp A4 miniature sheet preview
        let sheet = Rect::new(card.x + 12.0 * s, card.y + 12.0 * s, 52.0 * s, card.h - 24.0 * s);
        surface.fill_rect(sheet, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(sheet, 2.0, 1.0, DaylightTheme::BORDER_STRONG);
        surface.fill_rect(
            Rect::new(sheet.x + 7.0 * s, sheet.y + 9.0 * s, 28.0 * s, 3.0 * s),
            DaylightTheme::WORD_ACCENT,
        );
        for ln in 0..4 {
            surface.fill_rect(
                Rect::new(
                    sheet.x + 7.0 * s,
                    sheet.y + (18.0 + (ln as f32) * 11.0) * s,
                    sheet.w - 14.0 * s,
                    2.0,
                ),
                DaylightTheme::BORDER,
            );
        }

        let tx = sheet.x + sheet.w + 13.0 * s;
        let tw_avail = (card.w - (tx - card.x) - 12.0 * s).max(40.0);
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansBold,
            13.0 * s,
            tx,
            card.y + 32.0 * s,
            tw_avail,
            DaylightTheme::INK,
            tpl.label(),
        );
        // 2-line clean subtitle wrap instead of truncated `...`
        let sub = tpl.subtitle();
        surface.draw_text_clipped(
            fonts,
            FontSlot::SansRegular,
            11.2 * s,
            tx,
            card.y + 52.0 * s,
            tw_avail,
            DaylightTheme::INK_SUB,
            sub,
        );
        surface.draw_text(
            fonts,
            FontSlot::MonoRegular,
            10.2 * s,
            tx,
            card.y + card.h - 16.0 * s,
            if hov {
                DaylightTheme::RUST_ORANGE
            } else {
                DaylightTheme::WORD_ACCENT
            },
            "Create sheet ->",
        );

        hit_regions.push(HitRegion::button(card, UiAction::CreateFromTemplate(*tpl)));
    }
}

fn paint_toolbar_dropdown(
    app: &mut RustPadApp,
    surface: &mut Surface,
    s: f32,
    dd: ToolbarDropdown,
    win_w: f32,
    win_h: f32,
) {
    // Full-window click-away backdrop catcher
    app.hit_regions.push(HitRegion {
        rect: Rect::new(0.0, 0.0, win_w, win_h),
        action: UiAction::CloseDropdown,
        cursor: rustpad_window::CursorShape::Arrow,
    });

    match dd {
        ToolbarDropdown::ParagraphStyle { anchor_x, anchor_y } => {
            let items = [
                (ParagraphStyle::Normal, "Body Text", 12.5),
                (ParagraphStyle::Heading1, "Heading 1", 16.5),
                (ParagraphStyle::Heading2, "Heading 2", 14.5),
                (ParagraphStyle::Heading3, "Heading 3", 13.0),
                (ParagraphStyle::Title, "Document Title", 18.0),
                (ParagraphStyle::Subtitle, "Subtitle", 12.5),
                (ParagraphStyle::Quote, "Blockquote", 12.5),
            ];
            let mw = 206.0 * s;
            let row_h = (32.0 * s).round();
            let mh = (items.len() as f32) * row_h + 12.0 * s;
            let mx = (anchor_x as f32).min(win_w - mw - 12.0).max(8.0);
            let my = (anchor_y as f32 + 4.0).min(win_h - mh - 12.0);
            let card = Rect::new(mx, my, mw, mh);

            surface.draw_paper_shadow(card, 6.0, 14.0, 24);
            surface.fill_rounded_rect(card, 6.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 6.0, 1.0, DaylightTheme::BORDER_STRONG);

            for (i, (st, label, fsz)) in items.iter().enumerate() {
                let r = Rect::new(card.x + 6.0 * s, card.y + 6.0 * s + (i as f32) * row_h, mw - 12.0 * s, row_h);
                if r.contains(app.mouse_x, app.mouse_y) {
                    surface.fill_rounded_rect(r, 4.0, DaylightTheme::HOVER);
                }
                let slot = if st.is_bold_by_default() {
                    FontSlot::SansBold
                } else if *st == ParagraphStyle::Quote {
                    FontSlot::SerifRegular
                } else {
                    FontSlot::SansRegular
                };
                surface.draw_text(
                    &mut app.fonts,
                    slot,
                    fsz * s,
                    r.x + 10.0 * s,
                    r.y + row_h * 0.68,
                    DaylightTheme::INK,
                    label,
                );
                app.hit_regions
                    .push(HitRegion::button(r, UiAction::SelectParagraphStyle(*st)));
            }
        }
        ToolbarDropdown::FontFamily { anchor_x, anchor_y } => {
            let items = [
                (FontFamily::Sans, "Inter / Sans-Serif", FontSlot::SansRegular),
                (FontFamily::Serif, "Georgia / Editorial Serif", FontSlot::SerifRegular),
                (FontFamily::Mono, "JetBrains / Monospace", FontSlot::MonoRegular),
            ];
            let mw = 214.0 * s;
            let row_h = (34.0 * s).round();
            let mh = (items.len() as f32) * row_h + 12.0 * s;
            let mx = (anchor_x as f32).min(win_w - mw - 12.0).max(8.0);
            let my = (anchor_y as f32 + 4.0).min(win_h - mh - 12.0);
            let card = Rect::new(mx, my, mw, mh);

            surface.draw_paper_shadow(card, 6.0, 14.0, 24);
            surface.fill_rounded_rect(card, 6.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 6.0, 1.0, DaylightTheme::BORDER_STRONG);

            for (i, (fam, label, slot)) in items.iter().enumerate() {
                let r = Rect::new(card.x + 6.0 * s, card.y + 6.0 * s + (i as f32) * row_h, mw - 12.0 * s, row_h);
                if r.contains(app.mouse_x, app.mouse_y) {
                    surface.fill_rounded_rect(r, 4.0, DaylightTheme::HOVER);
                }
                surface.draw_text(
                    &mut app.fonts,
                    *slot,
                    12.5 * s,
                    r.x + 10.0 * s,
                    r.y + row_h * 0.66,
                    DaylightTheme::INK,
                    label,
                );
                app.hit_regions
                    .push(HitRegion::button(r, UiAction::SelectFontFamily(*fam)));
            }
        }
    }
}

fn paint_status_bar(app: &mut RustPadApp, surface: &mut Surface, s: f32, bar: Rect) {
    surface.fill_rect(bar, DaylightTheme::CHROME);
    surface.fill_rect(Rect::new(bar.x, bar.y, bar.w, 1.0), DaylightTheme::BORDER);

    surface.fill_rounded_rect(
        Rect::new(12.0, bar.y + bar.h * 0.5 - 3.5, 7.0, 7.0),
        3.5,
        if app.ai_dock.is_busy() {
            DaylightTheme::RUST_ORANGE
        } else {
            DaylightTheme::SUCCESS
        },
    );

    let controls_w = 368.0 * s;
    surface.draw_text_clipped(
        &mut app.fonts,
        FontSlot::SansRegular,
        11.4 * s,
        26.0,
        bar.y + bar.h * 0.65,
        (bar.w - controls_w - 40.0).max(80.0),
        DaylightTheme::INK_SUB,
        &app.status_toast,
    );

    let zoom_pct = match app.tabs.get(app.active_tab) {
        Some(StudioTab::Word(w)) => (w.zoom * 100.0).round() as i32,
        Some(StudioTab::Pdf(p)) => (p.zoom * 100.0).round() as i32,
        _ => 100,
    };
    let ui_pct = (app.ui_scale * 100.0).round() as i32;

    let bh = (22.0 * s).round();
    let by = (bar.y + (bar.h - bh) * 0.5).round();
    let mut rx = (bar.w - controls_w).max(160.0);

    let ui_lbl = format!("{ui_pct}%");
    let zoom_lbl = format!("{zoom_pct}%");

    // 1. UI Font Scale Stepper (`UI - 100% +`)
    for (lbl, w_px, cmd) in [
        ("UI-", 28.0 * s, CommandId::UiScaleDown),
        (ui_lbl.as_str(), 44.0 * s, CommandId::UiScaleReset),
        ("UI+", 28.0 * s, CommandId::UiScaleUp),
    ] {
        let r = Rect::new(rx, by, w_px.round(), bh);
        surface.fill_rounded_rect(r, 4.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(r, 4.0, 1.0, DaylightTheme::BORDER);
        let tw = app.fonts.measure_text(FontSlot::MonoRegular, lbl, 10.0 * s);
        surface.draw_text(
            &mut app.fonts,
            FontSlot::MonoRegular,
            10.0 * s,
            r.x + (r.w - tw) * 0.5,
            r.y + bh * 0.68,
            DaylightTheme::INK_SUB,
            lbl,
        );
        app.hit_regions
            .push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
        rx += r.w + 3.0 * s;
    }

    rx += 7.0 * s;

    // 2. Paper Zoom Stepper (`- 112% +`)
    for (lbl, w_px, cmd) in [
        ("-", 24.0 * s, CommandId::ZoomOut),
        (zoom_lbl.as_str(), 48.0 * s, CommandId::ZoomReset),
        ("+", 24.0 * s, CommandId::ZoomIn),
    ] {
        let r = Rect::new(rx, by, w_px.round(), bh);
        surface.fill_rounded_rect(r, 4.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(r, 4.0, 1.0, DaylightTheme::BORDER);
        let tw = app.fonts.measure_text(FontSlot::MonoRegular, lbl, 10.4 * s);
        surface.draw_text(
            &mut app.fonts,
            FontSlot::MonoRegular,
            10.4 * s,
            r.x + (r.w - tw) * 0.5,
            r.y + bh * 0.68,
            DaylightTheme::INK,
            lbl,
        );
        app.hit_regions
            .push(HitRegion::button(r, UiAction::ExecCommand(cmd)));
        rx += r.w + 3.0 * s;
    }

    rx += 7.0 * s;

    // 3. Dynamic Version Badge (`RustPad v1.3.0`)
    let ver_lbl = format!("RustPad v{}", env!("CARGO_PKG_VERSION"));
    let vw = (app.fonts.measure_text(FontSlot::SansBold, &ver_lbl, 10.6 * s) + 16.0 * s).round();
    if rx + vw <= bar.w - 6.0 {
        let about_r = Rect::new(rx, by, vw, bh);
        surface.fill_rounded_rect(about_r, 4.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(about_r, 4.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            &mut app.fonts,
            FontSlot::SansBold,
            10.6 * s,
            about_r.x + 8.0 * s,
            about_r.y + bh * 0.68,
            DaylightTheme::RUST_ORANGE,
            &ver_lbl,
        );
        app.hit_regions.push(HitRegion::button(
            about_r,
            UiAction::ExecCommand(CommandId::OpenAbout),
        ));
    }
}

fn paint_active_modal(app: &mut RustPadApp, surface: &mut Surface, s: f32, screen: Rect) {
    surface.fill_rect(screen, DaylightTheme::SCRIM);

    let Some(modal_snapshot) = app.modal.clone() else {
        return;
    };

    match modal_snapshot {
        ActiveModal::CommandPalette { query: q, selected: sel } => {
            let items = filter_commands(&q);
            let mw = (580.0 * s).min(screen.w - 32.0);
            let mh = (390.0 * s).min(screen.h - 48.0);
            let card = Rect::new((screen.w - mw) * 0.5, (72.0 * s).min(screen.h * 0.12), mw, mh);

            surface.draw_paper_shadow(card, 8.0, 18.0, 32);
            surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

            let input_r = Rect::new(card.x + 14.0 * s, card.y + 14.0 * s, card.w - 28.0 * s, 38.0 * s);
            surface.fill_rounded_rect(input_r, 5.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(input_r, 5.0, 1.5, DaylightTheme::WORD_ACCENT);

            let prompt = if q.is_empty() {
                "Search 55+ studio commands (table, ai, pdf, split, zoom, convert)…".to_string()
            } else {
                format!("{q}|")
            };
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::SansRegular,
                13.0 * s,
                input_r.x + 12.0 * s,
                input_r.y + 24.0 * s,
                input_r.w - 24.0 * s,
                if q.is_empty() {
                    DaylightTheme::INK_MUTED
                } else {
                    DaylightTheme::INK
                },
                &prompt,
            );

            let mut ry = input_r.y + input_r.h + 10.0 * s;
            let row_h = (32.0 * s).round();
            for (idx, cmd) in items.iter().take(9).enumerate() {
                if ry + row_h > card.y + card.h - 8.0 {
                    break;
                }
                let row = Rect::new(card.x + 10.0 * s, ry, card.w - 20.0 * s, row_h - 2.0);
                let is_sel = idx == sel;
                let hov = row.contains(app.mouse_x, app.mouse_y);
                if is_sel || hov {
                    surface.fill_rounded_rect(
                        row,
                        5.0,
                        if is_sel {
                            DaylightTheme::WORD_SOFT
                        } else {
                            DaylightTheme::HOVER
                        },
                    );
                }

                let cat_col = match cmd.category {
                    "PDF" => DaylightTheme::PDF_ACCENT,
                    "AI" => DaylightTheme::AI_ACCENT,
                    "Table" => DaylightTheme::RUST_ORANGE,
                    _ => DaylightTheme::WORD_ACCENT,
                };
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::MonoRegular,
                    10.2 * s,
                    row.x + 10.0 * s,
                    row.y + row.h * 0.66,
                    cat_col,
                    cmd.category,
                );
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.2 * s,
                    row.x + 76.0 * s,
                    row.y + row.h * 0.66,
                    row.w - 156.0 * s,
                    DaylightTheme::INK,
                    cmd.title,
                );
                if !cmd.shortcut.is_empty() {
                    let sw = app.fonts.measure_text(FontSlot::MonoRegular, cmd.shortcut, 10.5 * s);
                    surface.draw_text(
                        &mut app.fonts,
                        FontSlot::MonoRegular,
                        10.5 * s,
                        row.x + row.w - sw - 12.0 * s,
                        row.y + row.h * 0.66,
                        DaylightTheme::INK_MUTED,
                        cmd.shortcut,
                    );
                }
                app.hit_regions
                    .push(HitRegion::button(row, UiAction::PalettePick(cmd.id)));
                ry += row_h;
            }
        }

        ActiveModal::SignaturePad {
            strokes: strokes_clone,
            signer_name: signer,
            ..
        } => {
            let mw = (520.0 * s).min(screen.w - 32.0);
            let mh = 330.0 * s;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);
            let pad = Rect::new(card.x + 20.0 * s, card.y + 64.0 * s, card.w - 40.0 * s, 180.0 * s);
            if let Some(ActiveModal::SignaturePad { pad_rect, .. }) = &mut app.modal {
                *pad_rect = pad;
            }

            surface.draw_paper_shadow(card, 8.0, 18.0, 32);
            surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                15.0 * s,
                card.x + 20.0 * s,
                card.y + 28.0 * s,
                DaylightTheme::INK,
                "Draw Vector Ink Signature",
            );
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansRegular,
                11.8 * s,
                card.x + 20.0 * s,
                card.y + 48.0 * s,
                DaylightTheme::INK_SUB,
                &format!("Drag inside pad to sign as {signer}, then click Place Signature:"),
            );

            surface.fill_rounded_rect(pad, 6.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(pad, 6.0, 1.5, DaylightTheme::WORD_ACCENT);
            surface.fill_rect(
                Rect::new(pad.x + 20.0, pad.y + pad.h - 32.0, pad.w - 40.0, 1.0),
                DaylightTheme::BORDER_STRONG,
            );

            for stroke in &strokes_clone {
                for seg in stroke.windows(2) {
                    surface.draw_line(
                        seg[0].0,
                        seg[0].1,
                        seg[1].0,
                        seg[1].1,
                        2.4,
                        DaylightTheme::WORD_ACCENT,
                    );
                }
            }

            draw_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                card,
                Some(("Clear Ink", UiAction::SignatureClear)),
                ("Place Signature", UiAction::ModalConfirm),
            );
        }

        ActiveModal::FilePrompt {
            kind: k,
            input_path: inp,
            recent_candidates: cands,
        } => {
            let mw = (540.0 * s).min(screen.w - 32.0);
            let mh = 340.0 * s;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);

            surface.draw_paper_shadow(card, 8.0, 18.0, 32);
            surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

            let title = match k {
                FileModalKind::OpenAny => "Open Document (.docx, .pdf, .md)",
                FileModalKind::SaveActive => "Save Active Document",
                FileModalKind::ExportPdf => "Export Vector PDF (.pdf)",
                FileModalKind::MergePdf => "Select PDF File to Merge",
            };
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                15.0 * s,
                card.x + 20.0 * s,
                card.y + 30.0 * s,
                DaylightTheme::INK,
                title,
            );

            let box_r = Rect::new(card.x + 20.0 * s, card.y + 52.0 * s, card.w - 40.0 * s, 36.0 * s);
            surface.fill_rounded_rect(box_r, 5.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(box_r, 5.0, 1.5, DaylightTheme::WORD_ACCENT);
            surface.draw_text_clipped(
                &mut app.fonts,
                FontSlot::MonoRegular,
                12.2 * s,
                box_r.x + 10.0 * s,
                box_r.y + 23.0 * s,
                box_r.w - 20.0 * s,
                DaylightTheme::INK,
                &format!("{inp}|"),
            );

            let mut cy = box_r.y + box_r.h + 14.0 * s;
            for cand in cands.iter().take(5) {
                let row = Rect::new(card.x + 20.0 * s, cy, card.w - 40.0 * s, 28.0 * s);
                if row.contains(app.mouse_x, app.mouse_y) {
                    surface.fill_rounded_rect(row, 4.0, DaylightTheme::HOVER);
                }
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.0 * s,
                    row.x + 8.0 * s,
                    row.y + 19.0 * s,
                    row.w - 16.0 * s,
                    DaylightTheme::INK_SUB,
                    &cand.display().to_string(),
                );
                app.hit_regions.push(HitRegion::button(
                    row,
                    UiAction::FileCandidatePick(cand.clone()),
                ));
                cy += 29.0 * s;
            }

            draw_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                card,
                None,
                ("Confirm (Enter)", UiAction::ModalConfirm),
            );
        }

        ActiveModal::FindReplace {
            find_text: ft,
            replace_text: rt,
            editing_replace: repl_focus,
            matches_count: mc,
        } => {
            let mw = (480.0 * s).min(screen.w - 32.0);
            let mh = 256.0 * s;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);

            surface.draw_paper_shadow(card, 8.0, 18.0, 32);
            surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                15.0 * s,
                card.x + 20.0 * s,
                card.y + 30.0 * s,
                DaylightTheme::INK,
                &format!("Find & Replace ({mc} matches)"),
            );

            let f_box = Rect::new(card.x + 20.0 * s, card.y + 52.0 * s, card.w - 40.0 * s, 36.0 * s);
            let r_box = Rect::new(card.x + 20.0 * s, card.y + 104.0 * s, card.w - 40.0 * s, 36.0 * s);
            for (bx, lbl, val, active, focus_flag) in [
                (f_box, "Find:", &ft, !repl_focus, false),
                (r_box, "Replace:", &rt, repl_focus, true),
            ] {
                surface.fill_rounded_rect(bx, 5.0, DaylightTheme::SIDEBAR);
                surface.stroke_rounded_rect(
                    bx,
                    5.0,
                    if active { 1.5 } else { 1.0 },
                    if active {
                        DaylightTheme::WORD_ACCENT
                    } else {
                        DaylightTheme::BORDER
                    },
                );
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::SansBold,
                    11.5 * s,
                    bx.x + 10.0 * s,
                    bx.y + 23.0 * s,
                    DaylightTheme::INK_MUTED,
                    lbl,
                );
                let disp = if active { format!("{val}|") } else { val.clone() };
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.5 * s,
                    bx.x + 78.0 * s,
                    bx.y + 23.0 * s,
                    bx.w - 88.0 * s,
                    DaylightTheme::INK,
                    &disp,
                );
                app.hit_regions
                    .push(HitRegion::text(bx, UiAction::FindFocusField(focus_flag)));
            }

            draw_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                card,
                None,
                ("Replace All", UiAction::FindReplaceAll),
            );
        }

        ActiveModal::SplitRange { range_spec } => {
            let spec = range_spec.clone();
            paint_single_input_modal(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                screen,
                "Split / Extract PDF Page Ranges",
                "Enter page numbers or ranges (e.g. 1-2, 4):",
                &spec,
                "Extract to New Tab",
            );
        }

        ActiveModal::Watermark { text } => {
            let wm = text.clone();
            paint_single_input_modal(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                screen,
                "Diagonal Page Watermark",
                "Enter watermark text (or leave empty to clear):",
                &wm,
                "Apply Watermark",
            );
        }

        ActiveModal::SnapshotsDrawer => {
            let entries = match app.tabs.get(app.active_tab) {
                Some(StudioTab::Word(w)) => w
                    .snapshots
                    .list()
                    .iter()
                    .rev()
                    .map(|e| (e.id, e.label.clone(), e.word_count, e.compressed_bytes.len()))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            let mw = (500.0 * s).min(screen.w - 32.0);
            let mh = 340.0 * s;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);

            surface.draw_paper_shadow(card, 8.0, 18.0, 32);
            surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                15.0 * s,
                card.x + 20.0 * s,
                card.y + 30.0 * s,
                DaylightTheme::INK,
                "Local Time-Machine Version History",
            );

            let mut sy = card.y + 52.0 * s;
            if entries.is_empty() {
                surface.draw_text(
                    &mut app.fonts,
                    FontSlot::SansRegular,
                    12.5 * s,
                    card.x + 20.0 * s,
                    sy + 20.0 * s,
                    DaylightTheme::INK_SUB,
                    "Open a Word (.docx) tab to capture & restore snapshots.",
                );
            } else {
                for (id, label, wc, bytes) in entries.iter().take(5) {
                    let row = Rect::new(card.x + 20.0 * s, sy, card.w - 40.0 * s, 40.0 * s);
                    surface.fill_rounded_rect(row, 5.0, DaylightTheme::SIDEBAR);
                    surface.stroke_rounded_rect(row, 5.0, 1.0, DaylightTheme::BORDER);
                    surface.draw_text_clipped(
                        &mut app.fonts,
                        FontSlot::SansBold,
                        12.0 * s,
                        row.x + 12.0 * s,
                        row.y + 18.0 * s,
                        row.w - 120.0 * s,
                        DaylightTheme::INK,
                        &format!("#{id} · {label} ({wc} words, {bytes} B)"),
                    );
                    surface.draw_text(
                        &mut app.fonts,
                        FontSlot::MonoRegular,
                        11.0 * s,
                        row.x + row.w - 86.0 * s,
                        row.y + 23.0 * s,
                        DaylightTheme::WORD_ACCENT,
                        "Restore ->",
                    );
                    app.hit_regions
                        .push(HitRegion::button(row, UiAction::RestoreSnapshot(*id)));
                    sy += 46.0 * s;
                }
            }

            draw_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                card,
                Some((
                    "+ Capture Snapshot",
                    UiAction::ExecCommand(CommandId::CaptureSnapshot),
                )),
                ("Close", UiAction::ModalClose),
            );
        }

        ActiveModal::About => {
            let mw = (480.0 * s).min(screen.w - 32.0);
            let mh = 264.0 * s;
            let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);

            surface.draw_paper_shadow(card, 8.0, 18.0, 32);
            surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
            surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

            surface.draw_logo_64((card.x + 22.0 * s) as i32, (card.y + 22.0 * s) as i32);
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansBold,
                17.5 * s,
                card.x + 98.0 * s,
                card.y + 44.0 * s,
                DaylightTheme::INK,
                &format!("RustPad v{}", env!("CARGO_PKG_VERSION")),
            );
            surface.draw_text(
                &mut app.fonts,
                FontSlot::SansRegular,
                12.2 * s,
                card.x + 98.0 * s,
                card.y + 66.0 * s,
                DaylightTheme::INK_SUB,
                "Pure-Rust Native Word (.docx), Vector PDF & AI Studio",
            );

            let lines = [
                "Author:   Nour Yahyaoui <nourryahyaoui@gmail.com>",
                "GitHub:   https://github.com/Nour-yahyaoui/rustpad",
                "LinkedIn: linkedin.com/in/nour-yahyaoui-86987b36b",
                "License:  MIT · Zero WebView · Local + Cloud AI Ready",
            ];
            let mut ly = card.y + 112.0 * s;
            for ln in lines {
                surface.draw_text_clipped(
                    &mut app.fonts,
                    FontSlot::MonoRegular,
                    11.0 * s,
                    card.x + 24.0 * s,
                    ly,
                    card.w - 48.0 * s,
                    DaylightTheme::INK,
                    ln,
                );
                ly += 20.0 * s;
            }

            draw_modal_footer_buttons(
                surface,
                &mut app.fonts,
                &mut app.hit_regions,
                s,
                card,
                None,
                ("Close", UiAction::ModalClose),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_single_input_modal(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    s: f32,
    screen: Rect,
    title: &str,
    subtitle: &str,
    val: &str,
    confirm_label: &str,
) {
    let mw = (460.0 * s).min(screen.w - 32.0);
    let mh = 214.0 * s;
    let card = Rect::new((screen.w - mw) * 0.5, (screen.h - mh) * 0.5, mw, mh);

    surface.draw_paper_shadow(card, 8.0, 18.0, 32);
    surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER_STRONG);

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        15.0 * s,
        card.x + 20.0 * s,
        card.y + 30.0 * s,
        DaylightTheme::INK,
        title,
    );
    surface.draw_text(
        fonts,
        FontSlot::SansRegular,
        12.0 * s,
        card.x + 20.0 * s,
        card.y + 52.0 * s,
        DaylightTheme::INK_SUB,
        subtitle,
    );

    let box_r = Rect::new(card.x + 20.0 * s, card.y + 72.0 * s, card.w - 40.0 * s, 38.0 * s);
    surface.fill_rounded_rect(box_r, 5.0, DaylightTheme::SIDEBAR);
    surface.stroke_rounded_rect(box_r, 5.0, 1.5, DaylightTheme::WORD_ACCENT);
    surface.draw_text_clipped(
        fonts,
        FontSlot::MonoRegular,
        12.5 * s,
        box_r.x + 10.0 * s,
        box_r.y + 24.0 * s,
        box_r.w - 20.0 * s,
        DaylightTheme::INK,
        &format!("{val}|"),
    );

    draw_modal_footer_buttons(
        surface,
        fonts,
        hit_regions,
        s,
        card,
        None,
        (confirm_label, UiAction::ModalConfirm),
    );
}

fn draw_modal_footer_buttons(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    s: f32,
    card: Rect,
    extra_left: Option<(&str, UiAction)>,
    confirm: (&str, UiAction),
) {
    let bh = (32.0 * s).round();
    let by = card.y + card.h - bh - 16.0 * s;

    if let Some((lbl, act)) = extra_left {
        let lw = fonts.measure_text(FontSlot::SansBold, lbl, 11.8 * s) + 22.0 * s;
        let er = Rect::new(card.x + 20.0 * s, by, lw, bh);
        surface.fill_rounded_rect(er, 5.0, DaylightTheme::SIDEBAR);
        surface.stroke_rounded_rect(er, 5.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.8 * s,
            er.x + 11.0 * s,
            er.y + bh * 0.66,
            DaylightTheme::INK,
            lbl,
        );
        hit_regions.push(HitRegion::button(er, act));
    }

    let cw = fonts.measure_text(FontSlot::SansBold, confirm.0, 11.8 * s) + 24.0 * s;
    let confirm_r = Rect::new(card.x + card.w - cw - 20.0 * s, by, cw, bh);
    let cancel_r = Rect::new(confirm_r.x - 86.0 * s, by, 78.0 * s, bh);

    surface.fill_rounded_rect(cancel_r, 5.0, DaylightTheme::PAPER);
    surface.stroke_rounded_rect(cancel_r, 5.0, 1.0, DaylightTheme::BORDER);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        11.8 * s,
        cancel_r.x + 16.0 * s,
        cancel_r.y + bh * 0.66,
        DaylightTheme::INK_SUB,
        "Cancel",
    );
    hit_regions.push(HitRegion::button(cancel_r, UiAction::ModalClose));

    surface.fill_rounded_rect(confirm_r, 5.0, DaylightTheme::WORD_ACCENT);
    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        11.8 * s,
        confirm_r.x + 12.0 * s,
        confirm_r.y + bh * 0.66,
        DaylightTheme::PAPER,
        confirm.0,
    );
    hit_regions.push(HitRegion::button(confirm_r, confirm.1));
}
