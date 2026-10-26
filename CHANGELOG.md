# Changelog — RustPad

All notable additions to **RustPad** (authored by **Nour Yahyaoui** — [@Nour-yahyaoui](https://github.com/Nour-yahyaoui), `nourryahyaoui@gmail.com`) follow our continuous micro-release workflow (`Commit → Tag → GitHub Release`).

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
