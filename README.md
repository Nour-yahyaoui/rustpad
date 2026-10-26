# RustPad (`rustpad`)

<p align="left">
  <img src="assets/icons/icon-256.png" alt="RustPad logo" width="84" height="84" />
</p>

**Create, edit, sign, redact, and convert Microsoft Word (`.docx`) and vector PDF (`.pdf`) documents offline in a single `3.3 MB` pure-Rust desktop binary.**

- **Author**: Nour Yahyaoui — [github.com/Nour-yahyaoui](https://github.com/Nour-yahyaoui) · `nourryahyaoui@gmail.com`
- **Current Release**: [`v1.1.0`](https://github.com/Nour-yahyaoui/word/releases) (Desktop Core `v1.0.0` · MIT License)
- **Stack**: 100% Pure Rust (`14,280` LOC across 9 workspace crates, zero WebView, zero external `crates.io` runtime dependencies) + Root Vite TypeScript Project Showcase

![RustPad Word Studio editing an A4 OpenXML document with Document Outline and Live Metrics](docs/screenshots/word-studio.png)

---

## Why RustPad Exists

Desktop document editors routinely ship 150–400 MB Chromium wrappers or multi-gigabyte installers just to write an A4 `.docx` report or sign and merge a `.pdf`. **RustPad** compiles every layer—from RFC 1951 DEFLATE/PKZIP compression and ECMA-376 OpenXML serialization to ISO 32000 PDF 1.7 layout, TrueType glyph rasterization, and native X11/Win32 window blitting—into a standalone `3.3 MB` native executable with a calm **Paper & Daylight** light theme.

---

## Quick Start

### Option A — Build & Run from Source (Linux, Windows, macOS)

Requires only a standard Rust toolchain (`rustc` + `cargo` 1.75+). Compiles offline with zero external network fetches:

```bash
git clone https://github.com/Nour-yahyaoui/word.git
cd word

# Run all 13 workspace unit tests across all 9 crates
cargo test --workspace

# Build the stripped ~3.3 MB release executable
cargo build --release -p rustpad

# Launch the native Paper & Daylight desktop studio
./target/release/rustpad
```

### Option B — Try Sample Deliverables & Headless CLI Immediately

```bash
# Generate 6 sample .docx, .pdf, and .md documents in ./examples
./target/release/rustpad demo-files examples

# Inspect OpenXML or PDF structure & word/page counts
./target/release/rustpad inspect examples/RustPad_Welcome_Guide.docx
./target/release/rustpad inspect examples/RustPad_Welcome_Guide.pdf

# Convert between .docx, .pdf, and .md offline
./target/release/rustpad convert examples/RustPad_Welcome_Guide.docx Welcome_Export.pdf
```

---

## Core Workflows

### 1. Authoring & Formatting Microsoft Word (`.docx`) Documents

Open **RustPad** to pick one of **6 built-in OpenXML starter templates** (*Blank Document*, *Welcome Guide*, *Modern Resume / CV*, *Technical Report*, *Business Invoice*, *Executive Letter*) or open any existing `.docx` / `.md` file (`Ctrl+O`).

- **Typography & Blocks**: Switch paragraph styles (*Body*, *Title*, *Heading 1–3*, *Quote*, *Code*), font families (*Sans*, *Serif*, *Monospace*), sizes (`A-` / `A+`), colors, amber highlights, alignments, bulleted/numbered lists, interactive checklists, and multi-column tables (`+ Table`).
- **Page Geometry**: Toggle **A4**, **US Letter**, or **Legal** stock with running headers, footers, and automatic `Page X of Y` numbering.
- **Navigation & Safety**: Jump to any section from the left **Document Outline**, monitor live word/character/reading-time counts, run inline offline **Levenshtein spellcheck**, and capture **Time-Machine version snapshots** (`History`).

### 2. Editing, Signing, Redacting & Merging PDF (`.pdf`) Files

Click **`-> PDF`** (`Ctrl+E`) in Word Studio to generate a vector PDF 1.7 document directly in a new tab, or open any PDF 1.4–1.7 file in **PDF Studio**:

![RustPad PDF Studio showing inline text editing, vector APPROVED stamp, sticky note, highlight, and polyline ink signature](docs/screenshots/pdf-studio.png)

- **Inline Text Editing**: With `Select / Edit` active, click any text line on the PDF canvas to edit its string content directly in place.
- **Ink Signatures, Stamps & Notes**: Open `Sign Ink` to draw a smooth freehand polyline signature on the interactive pad, place `[APPROVED]` vector stamps, add sticky notes (`SubType /Text`), or highlight passages.
- **True Security Redaction**: Select `Redact` and click sensitive passages—RustPad physically removes intersecting `PdfTextItem` strings from the underlying PDF content stream rather than merely painting a black rectangle on top (`crates/pdf_engine/src/lib.rs:206-236`).
- **Page Operations**: Rotate pages `90°`, insert blank sheets, split page ranges, merge multiple PDFs (`Ctrl+M`), add diagonal watermarks, compress streams with `/FlateDecode`, or convert the PDF back into an editable `.docx` (`-> Word .docx`).

### 3. Side-by-Side 50/50 Split Workspace & `Ctrl+K` Command Palette

Press **`Ctrl+\`** (or click **Split** in the top header) to place your Word draft on the left 50% and a companion PDF Studio sheet on the right 50%. Press **`Ctrl+K`** anywhere to fuzzy-search all **49 studio commands**:

![RustPad 50/50 Split Workspace with the Ctrl+K fuzzy Command Palette open](docs/screenshots/split-command-palette.png)

---

## CLI Reference

Verified output of `rustpad --help` (`app/src/main.rs:328-347`):

```text
USAGE:
  rustpad [FILE]                        Launch the native desktop studio (optionally opening FILE)
  rustpad convert <INPUT> <OUTPUT>      Convert between .docx, .pdf, and .md offline
  rustpad inspect <FILE>                Print page, word, character, or PDF object metrics
  rustpad ocr <FILE>                    Extract searchable text from a PDF or scanned image
  rustpad merge <OUT.pdf> <IN1.pdf>...  Merge multiple PDFs into a single vector PDF
  rustpad demo-files <OUT_DIR>          Generate showcase .docx, .pdf, and .md sample documents
  rustpad --render-preview <OUT_DIR>    Render 1440x900 PNG studio screenshots & 1200x630 OG card
  rustpad --version                     Print version information
  rustpad --help                        Print this help guide
```

---

## Keyboard Shortcuts

| Shortcut | Action | Scope |
|---|---|---|
| `Ctrl+K` | Open fuzzy Command Palette (49 actions) | Global |
| `Ctrl+\` | Toggle 50/50 Side-by-Side Split Workspace | Global |
| `Ctrl+N` | Create new Word (`.docx`) document tab | Global |
| `Ctrl+O` | Open `.docx`, `.pdf`, or `.md` file | Global |
| `Ctrl+S` / `Ctrl+Shift+S` | Save / Save As | Active Tab |
| `Ctrl+E` | Export Word document to Vector PDF (`.pdf`) | Word Studio |
| `Ctrl+M` | Merge another PDF into active PDF | PDF Studio |
| `Ctrl+B` / `Ctrl+I` / `Ctrl+U` | Toggle Bold / Italic / Underline | Word Studio |
| `Ctrl+F` | Find & Replace modal | Word Studio |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo | Word Studio |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Zoom In / Zoom Out / Reset 100% | Word & PDF |

---

## Pure-Rust 9-Crate Workspace Architecture

All 9 workspace crates reside in `Cargo.toml` with zero external runtime dependencies:

| Crate | Path | LOC | Responsibility |
|---|---|---:|---|
| `rustpad` (`app`) | `app/src/` | 5,384 | Multi-tab Word/PDF state, 49 commands, Split View, 7 modals, CLI & preview renderer |
| `pdf_engine` | `crates/pdf_engine/` | 1,586 | PDF 1.4–1.7 parser, inline text edit, ink signatures, stamps, redaction, merge, split, compress |
| `rustpad_window` | `crates/rustpad_window/` | 1,291 | Dynamic `dlopen("libX11.so.6")` Linux backend + Win32 `StretchDIBits` & native OS file dialogs |
| `docx_io` | `crates/docx_io/` | 1,275 | ECMA-376 OpenXML `.docx` reader & writer (paragraphs, styles, lists, tables, media, headers/footers) |
| `rustpad_render` | `crates/rustpad_render/` | 1,196 | Pure-Rust TrueType (`.ttf`) glyph parser, 4×4 supersampled rasterizer, `DaylightTheme`, SDF & PNG |
| `doc_model` | `crates/doc_model/` | 1,083 | Structured Document AST, A4/Letter/Legal page geometry, Undo/Redo, Find/Replace & 6 templates |
| `power_tools` | `crates/power_tools/` | 868 | `DOCX <-> PDF <-> Markdown` converters, offline Levenshtein spellcheck, OCR & Time-Machine vault |
| `rustpad_zip` | `crates/rustpad_zip/` | 812 | IEEE CRC-32, Adler-32, RFC 1951 DEFLATE, RFC 1950 Zlib & PKZIP archive reader/writer |
| `pdf_create` | `crates/pdf_create/` | 759 | ISO 32000 PDF 1.7 vector layout engine, 12 Type1 font slots, full-line run coalescing & watermarks |

Further architectural specs live in `docs/`:
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — Data-flow pipelines across Word, PDF, and rendering crates.
- [`docs/UI_UX_LIGHT_THEME.md`](docs/UI_UX_LIGHT_THEME.md) — Native desktop **Paper & Daylight** palette & layout rules.
- [`docs/STYLE.md`](docs/STYLE.md) — Web Landing Page Swiss Proof-Sheet visual specification.
- [`CHANGELOG.md`](CHANGELOG.md) — Complete micro-release history (`v0.1.0` -> `v1.1.0`).

---

## Project Showcase Website (Root Vite App)

The repository root also contains the static **RustPad Specification & Specimen Showcase** (deployed on Vercel):

```bash
npm ci
npm run dev      # Local dev server at http://localhost:5173
npm run build    # Production build into ./dist
```

---

## Troubleshooting & OS Notes

- **Headless / SSH Linux Environments**: Launching `./target/release/rustpad` without `DISPLAY` set prints a clear message explaining how to run X11 or use the headless CLI (`convert`, `inspect`, `ocr`, `merge`, `demo-files`, `--render-preview`).
- **Windows SmartScreen / macOS Gatekeeper on Unsigned Builds**: Community binaries compiled via `./scripts/package-release.sh` are not signed with a commercial EV certificate. Verify the file hash against `SHA256SUMS.txt` first; on Windows click **More info -> Run anyway**, or on macOS right-click **Open** (or build directly from source via `cargo build --release -p rustpad`).

---

## Author & License

Created and maintained by **Nour Yahyaoui**
- GitHub: [https://github.com/Nour-yahyaoui](https://github.com/Nour-yahyaoui)
- Email: [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com)
- License: [MIT](LICENSE)
