# Documentation Inventory — RustPad (`Nour-yahyaoui/word`)

- **Project Type**: Dual surface — (1) Cross-platform Pure-Rust Native Desktop Application & CLI (`rustpad` v1.0.0) and (2) Root Vite Static Project Landing Page deployed on Vercel.
- **Primary Audience**: Developers, technical writers, students, and privacy-conscious desktop users who want a fast, offline `3.3 MB` Word (`.docx`) and PDF (`.pdf`) desktop suite without WebView bloat or cloud lock-in.
- **Author**: Nour Yahyaoui (`https://github.com/Nour-yahyaoui`, `nourryahyaoui@gmail.com`)
- **License**: MIT (`LICENSE`)

## Verified Entrypoints & Source Map (`file:line`)

| Crate / Component | Verified Path | Responsibility |
|---|---|---|
| Workspace Root | `Cargo.toml:1-26` | 9-member pure-Rust Cargo workspace (`version = "1.0.0"`, `lto = "thin"`, `strip = "symbols"`) |
| Desktop & CLI Entrypoint | `app/src/main.rs:26-79`, `app/src/main.rs:236-352` | Launches native GUI window (`NativeAppRunner`) or executes CLI subcommands (`convert`, `inspect`, `ocr`, `merge`, `demo-files`, `--render-preview`, `--version`, `--help`) |
| Command Registry (`Ctrl+K`) | `app/src/commands.rs:7-134` | 49 typed `CommandId` actions with keyboard shortcuts and fuzzy multi-token filtering (`filter_commands`) |
| Unified Studio State | `app/src/state.rs:95-210`, `app/src/state.rs:640-1927` | Multi-tab `StudioTab` (`HomeHub`, `Word`, `Pdf`), 50/50 Split Workspace, 7 modal dialogs, mouse/scroll/signature-stroke/keyboard dispatch |
| Paginated Word View | `app/src/word_view.rs:38-929` | Word formatting ribbon, clickable heading outline rail, live metrics card, paginated A4/Letter/Legal canvas, checklists, tables, inline spellcheck underlines |
| Interactive PDF Studio View | `app/src/pdf_view.rs:28-683` | PDF tool ribbon, page thumbnail rail, vector page sheet renderer, inline text editing, highlights, sticky notes, stamps, polyline ink signatures, redactions, inspector rail |
| Master Daylight Painter | `app/src/ui_paint.rs:24-1344` | Unified top header bar with RustPad logo & tabs, Home Hub template gallery, 50/50 Split Workspace, status bar, and modal overlays |
| DEFLATE / Zlib / PKZIP | `crates/rustpad_zip/src/lib.rs:1-24`, `crates/rustpad_zip/src/inflate.rs:1-293`, `crates/rustpad_zip/src/deflate.rs:1-183` | Zero-dependency IEEE CRC-32, Adler-32, RFC 1951 DEFLATE, RFC 1950 Zlib, and PKZIP reader/writer |
| Document AST & Templates | `crates/doc_model/src/lib.rs:1-723`, `crates/doc_model/src/templates.rs:1-360` | Rich `Document` model, `PageSetup` (A4/Letter/Legal), live metrics, Find & Replace, Undo/Redo ring buffer, and 6 starter templates |
| Microsoft Word `.docx` I/O | `crates/docx_io/src/reader.rs:1-502`, `crates/docx_io/src/writer.rs:1-433` | ECMA-376 OpenXML `.docx` parser and writer (`word/document.xml`, `docProps/core.xml`, `word/media/*`, headers/footers) |
| Vector PDF 1.7 Generator | `crates/pdf_create/src/lib.rs:36-759` | ISO 32000 PDF 1.7 layout engine, 12 Type1 font slots, full-line run coalescing, tables, watermarks, `/FlateDecode` streams |
| PDF Studio Edit Engine | `crates/pdf_engine/src/lib.rs:1-381`, `crates/pdf_engine/src/parser.rs:1-858`, `crates/pdf_engine/src/writer.rs:1-347` | PDF 1.4–1.7 parser, inline text editing, annotations, stamps, ink signatures, true text-stream redaction, merge, split, rotate, compress |
| Power Tools | `crates/power_tools/src/converter.rs:1-440`, `crates/power_tools/src/spellcheck.rs:1-210`, `crates/power_tools/src/ocr.rs:1-51`, `crates/power_tools/src/snapshots.rs:1-150` | Bidirectional `DOCX <-> PDF <-> Markdown` converter, offline Levenshtein spellchecker, offline OCR/scan extractor, Time-Machine snapshot vault |
| TrueType & 2D SDF Renderer | `crates/rustpad_render/src/ttf.rs:1-634`, `crates/rustpad_render/src/canvas.rs:1-383`, `crates/rustpad_render/src/theme.rs:1-83` | Pure-Rust `.ttf` glyph parser & 4×4 supersampled rasterizer, `DaylightTheme` tokens, SDF shapes, and PNG encoder |
| Native OS Windowing | `crates/rustpad_window/src/linux_x11.rs:1-628`, `crates/rustpad_window/src/win32.rs:1-458` | Dynamic `libX11.so.6` (Linux) and `Win32` (`user32`/`gdi32`/`comdlg32`) windowing + native OS Open/Save file dialogs |
| Release Packaging | `scripts/package-release.sh:1-34` | Runs workspace tests, builds stripped release binary (`3.3 MB`), bundles tarball, and generates `SHA256SUMS.txt` |

## Screenshots Available (Genuine Native Framebuffer Captures)
1. `docs/screenshots/word-studio.png` (`1440×900`) — Word Studio editing `Welcome to RustPad` with outline rail, live metrics, table, and interactive checklist.
2. `docs/screenshots/pdf-studio.png` (`1440×900`) — PDF Studio inspecting `Technical Architecture Report` with `[APPROVED]` vector stamp, sticky note, amber highlight, and blue ink signature.
3. `docs/screenshots/split-command-palette.png` (`1440×900`) — 50/50 Split Workspace (`Word` + `PDF`) with `Ctrl+K` Command Palette filtering `pdf` commands.
4. `docs/screenshots/home-hub.png` (`1440×900`) — Home Hub showing 6 starter templates and recent `.docx`/`.pdf` workspace files.
5. `docs/screenshots/og.png` (`1200×630`) — Open Graph social preview card showing side-by-side Word & PDF Studio.
