# STYLE.md — RustPad Project Landing Page Visual Specification

- **PROJECT**: RustPad (`rustpad`) — 100% Pure-Rust Native Desktop Word (`.docx`) & Vector PDF (`.pdf`) Studio
- **AUTHOR**: Nour Yahyaoui (`https://github.com/Nour-yahyaoui`, `nourryahyaoui@gmail.com`)
- **BASE RECIPE**: Domain-shifted **R4 Technical Datasheet / Spec Sheet** hybridized with **Swiss Letterpress Proof Sheet** conventions (not verbatim R1 `#F5F3EE`, not generic `#ffffff`/`#111827`).

## ANCESTOR
A **1970s Swiss Haas Type Foundry cotton proof sheet & ISO 216 / ECMA-376 drafting specification manual**, printed on 120gsm warm archival proof paper with 1px lead-rule ledger grids, pica/point registration coordinates (`595 × 842 pt`), and oxidized iron-oxide ("rust") proofreader's registration ink.

## REJECTED
1. **R2 — Dark Precision / Cockpit (`#0B0C0E` near-black default, Linear/Raycast style)**: Rejected as the primary default theme because the user explicitly requested a calm, fast **Daylight / Paper light aesthetic** tailored to reading and editing `.docx` and `.pdf` sheets without dark-mode halation (a secondary Darkroom Litho Film dark mode is still provided via semantic CSS tokens and `prefers-color-scheme`).
2. **R3 — Bold Signal / Poster (oversized brutalist slabs & high-chroma color blocks)**: Rejected because loud poster slabs undermine the quiet typographic authority, precision measurement, and document-security trust required for an OpenXML word processor and PDF redaction/signature engine.

## PALETTE

### Light Mode — Daylight Proof Sheet (`[data-theme="light"]`, default)
- `--bg`: `#F7F6F2` — Uncoated 120gsm cotton proof vellum under north-window daylight.
- `--surface`: `#FDFCF9` — Pressed A4 archival sheet surface for interactive specimen plates.
- `--surface-subtle`: `#EFECE6` — Recessed marginalia strip and index header fill.
- `--fg`: `#16181D` — Carbon platen printing ink (`15.4:1` contrast against `--bg`).
- `--muted`: `#525866` — Lead-slug secondary technical callout (`6.4:1` contrast against `--bg`).
- `--border`: `#DCD8CE` — 1px hairline pica rule dividing ledger columns and rows.
- `--border-strong`: `#BDB7A8` — Structural registration frame rule.
- `--accent`: `#B83A1B` — Oxidized iron-oxide ("Rust") proofreader registration stamp (`5.3:1` contrast against `--bg` and `--surface`).
- `--accent-hover`: `#992E14` — Deepened pressed iron-oxide ink on hover/active state.
- `--accent-fg`: `#FAF8F5` — Reverse proof stock ink on solid `--accent` buttons (`5.6:1` contrast).
- `--cobalt`: `#1E4DB7` — Archival blueprint ink reserved strictly for `.docx` format badges and interactive focus rings.

### Dark Mode — Darkroom Litho Film (`[data-theme="dark"]`)
- `--bg`: `#111317` — Anodized darkroom inspection table.
- `--surface`: `#181B21` — Litho film inspection plate.
- `--surface-subtle`: `#20242C` — Recessed ruler trough and code specimen well.
- `--fg`: `#ECEAE4` — Warm silver-halide emulsion text (`14.8:1` contrast against `--bg`).
- `--muted`: `#9CA3B0` — Etched reticle secondary label (`6.7:1` contrast against `--bg`).
- `--border`: `#2A2F3A` — 1px darkroom grid rule.
- `--border-strong`: `#3D4452` — Structural plate frame rule.
- `--accent`: `#E86442` — Illuminated iron-oxide registration mark (`5.5:1` contrast against `--bg`).
- `--accent-hover`: `#F07A5B` — Active illuminated iron-oxide highlight.
- `--accent-fg`: `#111317` — High-contrast carbon label on `--accent` buttons (`6.1:1` contrast).
- `--cobalt`: `#6B9BFF` — Blueprint `.docx` format badge and keyboard focus ring in dark mode.

## TYPE
- **Display & Editorial Proof Headings (`--font-display`)**: `'Instrument Serif', 'Newsreader', Georgia, 'Times New Roman', serif` — Crisp editorial proof serif with tight display tracking (`-0.025em`) evoking metal display type on cotton proof sheets.
- **UI & Technical Body (`--font-sans`)**: `'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` — Clean geometric-humanist grotesque for effortless scanning at 15–17px.
- **Specification, CLI & Byte Offsets (`--font-mono`)**: `'JetBrains Mono', ui-monospace, 'Cascadia Code', Consolas, monospace` — Used with `font-variant-numeric: tabular-nums` for binary sizes (`3.3 MB`), SHA-256 checksums, CLI subcommands, and crate metrics.

## SCALE
Mobile-first fluid scale (`>= 1.6x` step ratio between body, section head, and hero display):
- `--text-xs`: `0.75rem` (`12px`) — Uppercase tracked (`0.08em`) registration labels and spec indices.
- `--text-sm`: `0.875rem` (`14px`) — Dense ledger rows, CLI flags, and secondary annotations.
- `--text-base`: `1rem` (`16px`, line-height `1.62`, max measure `65ch`) — Primary technical prose.
- `--text-lg`: `1.1875rem` (`19px`, line-height `1.5`) — Lead summary deck.
- `--text-xl`: `clamp(1.35rem, 2.2vw, 1.65rem)` — Sub-plate headings (`h3`).
- `--text-2xl`: `clamp(1.85rem, 3.4vw, 2.5rem)` — Section specification headings (`h2`, tracking `-0.02em`).
- `--text-hero`: `clamp(2.45rem, 5.2vw, 4.15rem)` — Asymmetric proof-sheet hero headline (`h1`, line-height `1.04`, tracking `-0.03em`).

## LAYOUT
- **Max Container**: `1240px` (`77.5rem`) framed by continuous vertical 1px registration hairlines on desktop (`min-width: 1024px`), with `1px` horizontal ledger rules separating numbered specification plates (`01` through `06`).
- **Grid**: Mobile-first single column (`320px–767px`), 2-column asymmetric split at `min-width: 768px`, and **7 / 5 asymmetric proof-sheet grid** at `min-width: 1024px`. Left-aligned copy throughout; zero centered-stack SaaS hero.
- **Rhythm**: `4px` base unit (`4 / 8 / 12 / 16 / 24 / 32 / 48 / 72 / 96px`), generous proof-margin breathing room (`>= 30%` open paper).

## RADIUS
- `--radius-xs`: `2px` — Registration badges and keycap tags (`Ctrl+K`, `Ctrl+\`).
- `--radius-sm`: `4px` — Buttons, inputs, and specimen tabs (crisp drafted corners, never pill-bloated).
- `--radius-md`: `6px` — Native screenshot proof frame and modal spotlight card.

## SHADOW
- **95% border-only**: Sections, crate ledgers, comparison matrices, and download rows use **zero drop shadows** — separated strictly by `1px solid var(--border)` hairline rules.
- **Single restrained physical paper shadow (`--shadow-sheet`)**: Applied only to the rendered native app sheet and floating tour popover (`0 12px 32px -12px rgba(22, 24, 29, 0.14)` in light mode).

## SIGNATURE
**Interactive Native Studio Switchboard & Live Binary Specimen Inspector**:
1. **4-Mode Real Pixel Studio Switchboard**: Switches instantaneously between real `1440×900` native `rustpad --render-preview` framebuffer captures (*Word `.docx` Studio*, *Vector `.pdf` Studio*, *50/50 Split + `Ctrl+K` Palette*, and *Home Template Hub*) with coordinate callouts, active keyboard shortcuts, and zero fake browser chrome.
2. **Live `.docx` / `.pdf` Binary Specimen Inspector & Crate Ledger**: Visitors can inspect real downloadable `.docx` and `.pdf` sample files generated by `rustpad` (fetching their live byte size, magic header bytes `50 4B 03 04` PKZIP / `25 50 44 46 2D 31 2E 37` `%PDF-1.7`, and internal structure right in the browser with skeleton loading + retryable error handling) alongside a searchable ledger of all **9 zero-dependency Rust workspace crates**.

## BANNED (Enforced = 0 occurrences)
- Zero gradient text (`background-clip: text`), zero radial hero glows, zero decorative background blobs.
- Zero emojis anywhere in UI, headings, bullets, badges, or footer.
- Zero fake 3-dot macOS browser/terminal chrome around screenshots.
- Zero 3-identical-rounded-feature-cards-with-drop-shadows.
- Zero `"Introducing..."` eyebrow pills, zero marketing superlatives (`blazingly fast`, `revolutionary`, `next-gen`, `seamless`), zero invented testimonials or fake download counts.
- Zero `max-width` desktop-first CSS media queries (100% mobile-first `min-width` queries).
- Zero `href="#"` or dead buttons.
