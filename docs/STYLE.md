# RustPad Visual & Interaction Design Charter (`STYLE.md`)

## 1. Physical Ancestor & Studio Identity
RustPad's visual language derives from **an architect's daylight drafting table**: fine cotton rag paper resting on warm stone-linen board, carbon fountain-pen ink, hairline brass rules, and a restrained terracotta wax mark.

- **Mood**: Quiet, tactile, high-contrast, instrument-grade desktop software (informed by iA Writer, Linear Desktop, and Swiss editorial typography).
- **Ratio**: 90% warm stone/paper neutrals, 8% carbon & graphite ink, 2% intentional functional accents.

### Explicitly Rejected Directions (`REJECTED:`)
- `REJECTED:` Cold Tailwind SaaS gray/blue (`#F4F6F9` + `#2563EB` + `#E11D48`) — reads immediately as generic web-dashboard template output.
- `REJECTED:` 19 uniform bordered text pills in a single toolbar strip (`[Title] [Sans] [A-] [A+] [Ink] [Mark]`) — creates visual noise, truncates on narrow panes, and hides current state.
- `REJECTED:` 6 identical shadowed feature cards on the Home Hub with repeated `"Open in Studio ->"` links — classic AI-generated landing grid anti-pattern.
- `REJECTED:` Blocking modal dialog for AI assistance — prevents reading, scrolling, or referencing the document while drafting with an agent.
- `REJECTED:` Decorative gradients, radial glows, glassmorphism, or emoji icons anywhere in the desktop chrome or web showcase.

---

## 2. Color Tokens (`DaylightTheme`)

| Role | Token | Hex | Usage |
| :--- | :--- | :--- | :--- |
| Drafting Desk | `DESK_BG` | `#EFECE7` | Background behind paginated A4/Letter sheets |
| Studio Rail | `SIDEBAR` | `#F7F5F0` | Resizable Outline, PDF Pages, Inspector & AI Dock |
| Quiet Chrome | `CHROME` | `#FAF9F6` | Top tab bar, grouped icon ribbon & status bar |
| Cotton Sheet | `PAPER` | `#FFFFFF` | Document page surface, active tab, popover menus |
| Tactile Hover | `HOVER` | `#EAE6DF` | Immediate 60fps hover feedback on controls & rows |
| Hairline Rule | `BORDER` | `#DFDBD2` | 1px structural dividers & table grid borders |
| Control Edge | `BORDER_STRONG` | `#C8C2B6` | Active inputs, scrollbar thumbs, drag splitters |
| Carbon Ink | `INK` | `#181816` | Primary prose & active UI labels (`>14:1` on paper) |
| Graphite Sub | `INK_SUB` | `#52504A` | Secondary controls & outline items (`>7:1` contrast) |
| Editorial Muted | `INK_MUTED` | `#7A766E` | Shortcuts, coordinates & page running headers |
| Prussian Word | `WORD_ACCENT` | `#1F4B7A` | `.docx` focus ring, caret, selection & active toggles |
| Vermilion Seal | `PDF_ACCENT` | `#A63626` | `.pdf` badge, redaction/stamp accents |
| Rust Terracotta | `RUST_ORANGE` | `#B84A1C` | Studio brand mark & active splitter highlight |
| Botanical AI | `AI_ACCENT` | `#2D5A47` | Pluggable AI Agent Studio dock & streaming badge |

---

## 3. Typography & Ergonomic Scale
- **UI Chrome**: Embedded Inter (`SansRegular` / `SansBold`) at `13px–14.5px` base multiplied by `ui_scale` (`100%`–`150%`, auto-initialized from OS monitor DPI).
- **Document Prose**: `12.5pt` default body size at `112%` default canvas zoom with `1.48` line height; real typographic hierarchy (`Title 28pt`, `H1 22.5pt`, `H2 18pt`, `H3 14.5pt`, `Body 12.5pt`).
- **Tabular Metrics**: `tabular-nums` alignment for zoom percentages, word counts, page numbers, and latency indicators.

---

## 4. Interaction & Ergonomics Standards
1. **Direct Manipulation First**:
   - Every sidebar (Word Outline, Right AI Dock, PDF Pages, PDF Inspector) and Split Workspace divider has an interactive `6px` hit-zone with `<->` (`ResizeEW`) cursor feedback and a 1-click collapse toggle.
   - Tables wrap multi-line text automatically, render an in-cell caret, support `Tab` to advance/append rows, allow column border dragging, and expose direct `+ Add Row` / `+ Col` rails on the page plus contextual ribbon controls.
2. **Smooth Physics & Cursor Honesty**:
   - Pixel-stepped scroll (`56px` per notch) clamped strictly to `[0, max_scroll]` paired with a draggable canvas scrollbar thumb.
   - Native OS cursors (`IBeam` over editable text/cells, `Hand` over buttons/links, `ResizeEW` over splitters and table column borders).
3. **Non-Blocking AI Safety**:
   - AI requests execute on a background `std::thread` polled over `mpsc` so editing and scrolling never freeze.
   - Document text is isolated inside `<document_context>...</document_context>` delimiters; applying AI output records an `UndoHistory` checkpoint first so `Ctrl+Z` always reverts in one keystroke.
