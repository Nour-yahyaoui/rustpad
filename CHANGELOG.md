# Changelog — RustPad

All notable additions to **RustPad** (authored by **Nour Yahyaoui** — [@Nour-yahyaoui](https://github.com/Nour-yahyaoui), `nourryahyaoui@gmail.com`) follow our continuous micro-release workflow (`Commit → Tag → GitHub Release`).

## [v1.1.1] - 2026-10-26
### Added
- Deployed production **RustPad** landing page & live binary specimen showcase to Vercel at [`https://rustpad-studio.vercel.app`](https://rustpad-studio.vercel.app) and linked live URL in repository documentation.

## [v1.1.0] - 2026-10-26
### Added
- **RustPad Project Landing Page & Live Specimen Showcase** (`index.html`, `src/`, `public/`): Swiss Typesetter's Proof Sheet × Technical Datasheet web showcase (`docs/STYLE.md`) featuring the 4-Mode Native Studio Switchboard, Live `.docx`/`.pdf` Binary Header & Hex Specimen Inspector with downloadable deliverables, searchable 9-Crate Pure-Rust Architecture Ledger, interactive CLI Command Builder, zero-FOUC Daylight/Darkroom theme toggle, 3-step Spotlight Tour, and full Open Graph / JSON-LD SEO metadata.
- **Documentation & UX Verification Suite**: `docs/STYLE.md`, `docs/UI-REFORMAT-INVENTORY.md`, `docs/UI-REFORMAT-REPORT.md`, `docs/DOCS-INVENTORY.md`, and comprehensive usage-first `README.md`.

## [v1.0.0] - 2026-10-26
### Added
- **RustPad v1.0.0 Production Milestone**: First complete 100% pure-Rust native desktop release uniting Microsoft Word (`.docx`) authoring, vector PDF (`.pdf`) creation & editing, bidirectional converters, offline OCR & spellcheck, Side-by-Side Split Workspace, and standalone CLI (`rustpad`).
- Release packaging (`scripts/package-release.sh`) producing stripped standalone binary (`~3.3 MB`), compressed bundle, SHA-256 verification checksums, and showcase `.docx`/`.pdf` sample files.

## [v0.9.5] - 2026-10-26
### Added
- `app` (`rustpad`): Complete interactive native desktop studio & CLI integrating Home Hub (6 starter templates + recent workspace documents), paginated Word (`.docx`) Studio, vector PDF (`.pdf`) Studio (inline text block editing, highlights, sticky notes, stamps, freehand ink signature pad, true redaction, rotate/merge/split/compress), Side-by-Side Split Workspace (`Ctrl+\`), `Ctrl+K` fuzzy Command Palette across 49 actions, Time-Machine Snapshot drawer, CLI subcommands (`convert`, `inspect`, `ocr`, `merge`, `demo-files`, `--render-preview`), and full-line PDF text-stream coalescing.

## [v0.9.0] - 2026-10-26
### Added
- `crates/rustpad_window`: Zero-dependency cross-platform native desktop windowing & OS file dialog backend supporting Linux (`libX11.so.6` dynamic loader with `_NET_WM_ICON` 64×64 RustPad taskbar icon), Windows (`Win32` HiDPI `StretchDIBits` + Explorer Open/Save dialogs), keyboard shortcuts, scroll wheel, and mouse events.

## [v0.8.0] - 2026-10-26
### Added
- `crates/rustpad_render`: Pure-Rust TrueType (`.ttf`) SFNT font parser & 4×4 supersampled anti-aliased glyph atlas, **"Paper & Daylight" Light Theme** design tokens (`DaylightTheme`), SDF anti-aliased rounded rectangles, floating paper elevation shadows, embedded RustPad logo blitter, and lossless PNG encoder.

## [v0.7.0] - 2026-10-26
### Added
- `crates/power_tools`: Bidirectional `Word (.docx) ↔ PDF (.pdf)` and `Markdown ↔ Word` converters, offline Levenshtein spellchecker (`SpellChecker`), offline OCR / PDF scan extractor, and compressed local Time-Machine version vault (`SnapshotVault`).

## [v0.6.0] - 2026-10-26
### Added
- `crates/pdf_engine`: Pure-Rust PDF 1.4–1.7 parser, visual page model, and full PDF Studio editing engine supporting inline text-block editing, adding text boxes, highlights, sticky notes (`SubType /Text`), stamps, smooth polyline ink Signatures, true underlying-text Redaction, page rotate/reorder/delete, multi-PDF Merge, page-range Split/Extract, Watermarks, and `/FlateDecode` PDF Compression.

## [v0.5.1] - 2026-10-26
### Fixed
- `crates/pdf_create`: Encode WinAnsi bullet glyph (`0x95`) using standard PDF octal escape (`\225`) for clean compilation and PDF viewer fidelity.

## [v0.5.0] - 2026-10-26
### Added
- `crates/pdf_create`: Pure-Rust ISO 32000 Vector PDF 1.7 document generator supporting multi-page layout, 12 WinAnsi Type1 font variants, colors, highlights, tables, lists, headers/footers, `Page X of Y`, watermarks, and `/FlateDecode` Zlib stream compression.

## [v0.4.0] - 2026-10-26
### Added
- `crates/docx_io`: Pure-Rust ECMA-376 Microsoft Word (`.docx`) OpenXML serializer & reader supporting paragraphs, headings, inline styles, colors, highlights, lists, tables, embedded images, page dimensions/margins, running headers/footers, and Dublin Core metadata.

## [v0.3.0] - 2026-10-26
### Added
- `crates/doc_model`: Structured rich Document AST (`Document`, `Paragraph`, `TableBlock`, `ImageBlock`, `PageSetup` A4/Letter/Legal), span styles, live word/reading-time metrics, outline extraction, Find & Replace, Undo/Redo history ring buffer, and 6 Starter Templates (*Blank*, *Welcome Guide*, *Modern Resume*, *Technical Report*, *Business Invoice*, *Executive Letter*).

## [v0.2.0] - 2026-10-26
### Added
- Root Pure-Rust Cargo Workspace (`Cargo.toml`) configured for thin-LTO optimized standalone desktop binaries.
- `crates/rustpad_zip`: Pure-Rust IEEE CRC-32, Adler-32, RFC 1951 DEFLATE (Store, Fixed & Dynamic Huffman) / RFC 1950 Zlib compressor & decompressor, and PKZIP archive reader/writer powering `.docx` and `.pdf` engines.

## [v0.1.0] - 2026-10-26
### Added
- Official **RustPad** architecture, Pure-Rust stack, Paper & Daylight UI/UX, and roadmap blueprints (`docs/*`).
- Custom **RustPad** desktop logo & multi-resolution icon suite (`assets/icons/rustpad-logo.png`, `rustpad.ico`, `icon-256.png`, `icon-64.png`, embedded RGBA bitmaps).
- Bundled open-source TrueType font suite (`Sans-Regular`, `Sans-Bold`, `Serif-Regular`, `Mono-Regular`) ensuring crisp typography offline on every OS.
- MIT Open-Source `LICENSE` and `.gitignore`.
