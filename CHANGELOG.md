# Changelog — RustPad

All notable additions to **RustPad** (authored by **Nour Yahyaoui** — [LinkedIn](https://www.linkedin.com/in/nour-yahyaoui-86987b36b) · [@Nour-yahyaoui](https://github.com/Nour-yahyaoui) · `nourryahyaoui@gmail.com`) follow our continuous release workflow (`Issue → PR → CI Matrix → GitHub Release`).

## [v2.3.0] - 2026-10-31 — Virtualized Large-Document & Multi-Screenshot `.docx` Engine (`30–100+ MB` Crash Fix)
### Fixed & Added
- **Eliminated 30 MB+ Multi-Page / Screenshot-Heavy `.docx` Crash, OOM & UI Freeze** (`crates/doc_model/src/lib.rs`, `crates/docx_io/src/{reader,writer}.rs`, `crates/power_tools/src/snapshots.rs`, `crates/rustpad_render/src/{jpeg_codec,png_codec,lib}.rs`, `crates/rustpad_zip/src/zip_archive.rs`, `crates/rustpad_window/src/win32.rs`, `app/src/{state,word_view,ui_paint}.rs`, `app/build.rs`, `#47`):
  - **Progressive Viewport-Only Image Decoding & `64 MiB` LRU Thumbnail Cache:** Replaced Frame-1 eager all-document RGBA decoding (which allocated `2–4.5 GB` of uncompressed RGBA buffers upfront when opening `100+` Retina/4K screenshots) with zero-allocation `probe_media_dimensions` (`IHDR`/`SOF0`/`BMP`/`GIF` header probing in microseconds), viewport-culled progressive streaming (`max 3` decodes/frame at `60Hz`), box-downsampled HiDPI viewport thumbnails (`<= 1440px` via `decode_any_image_thumbnail_rgba`), a hard **`64 MiB` byte-bounded LRU cache**, and `failed_image_hashes` tombstones so unsupported legacy media never retries on every frame.
  - **`SharedBytes` (`Arc<[u8]>`) Zero-Copy Media & CoW Time-Machine Snapshots:** Switched `ImageBlock::png_bytes` and `rgba_preview` from `Vec<u8>` to reference-counted `SharedBytes`, deduplicated identical archive media entries on load, stripped `passthrough_parts` from `UndoHistory` (`O(1)` media memory across 80 undo steps), and upgraded `SnapshotVault::capture` to zero-copy CoW in-memory snapshots for media-heavy documents (eliminating synchronous double-LZ77 `save_docx_bytes` + `zlib_compress` over 30+ MB of screenshots when opening a file).
  - **Cached Multi-Page Document Layout & Linear `O(W)` Word-Wrap:** Added FNV-1a layout fingerprinting (`ensure_cached_layout` returning `Rc<Vec<Vec<LaidOutBlock>>>`) + sub-page vertical block culling so scrolling, selection, and progressive figure streaming over 200+ pages perform `0.00 ms` re-layout, and upgraded paragraph/table word-wrapping from `O(W²)` string re-allocation to `O(W)` incremental width tracking.
  - **`1 GiB` ZIP Security Budget, `STORED` Media Fast-Path, `32,768px` Codec Cap & `16 MiB` Windows Stack:** Expanded `ArchiveSecurityLimits` (`32,768` entries, `256 MiB` entry, `1 GiB` unpacked budget, `2,000:1` XML compression ratio), skipped redundant `deflate_compress` on pre-compressed PNG/JPEG/WebP media in `ZipWriter::finish` (`~500×` faster saves), raised PNG/JPEG/BMP dimension limits from `8,192px` (`32 MP`) to `32,768px` (`64 MP`) for tall stitched screenshots, extracted `<w:drawing>` figures inside layout table cells, provisioned `/STACK:16777216` (`16 MiB` stack) on Windows MSVC, and added `std::panic::catch_unwind` boundaries around `open_path` and Win32 `wnd_proc`.

## [v2.2.0] - 2026-10-28 — Smooth `.docx` Image & Figure Studio + Effortless Table Creation Release
### Added
- **Pure-Rust `.docx` Image Codec Suite (`PNG` / `JPEG` / `BMP`), Full OOXML DrawingML & PDF Image XObjects** (`crates/rustpad_render/src/{png_codec,jpeg_codec,lib}.rs`, `crates/docx_io/src/{reader,writer,lib}.rs`, `crates/pdf_create/src/lib.rs`, `#39`):
  - Implemented a from-scratch ITU-T.81 Baseline, Extended Sequential & Progressive **JPEG decoder** (`SOF0`/`SOF1`/`SOF2`, DHT Huffman trees, 2D float IDCT, `YCbCr`/`CMYK`/Grayscale upsampling) alongside full RFC 2083 **PNG** (`1`/`2`/`4`/`8`/`16`-bit depths, `PLTE` indexed palettes + `tRNS` alpha, Adam7 de-interlacing) and **BMP/DIB** (`24`/`32`-bit + clipboard screenshots), unified under `decode_any_image_rgba` (`0` `crates.io` deps).
  - Upgraded `docx_io::reader` to traverse `<w:drawing>`, `<mc:AlternateContent>` (`<mc:Choice>` with `<mc:Fallback>` deduplication), legacy `<w:pict>`/`<v:imagedata>`, `<w:sdt>` content controls, normalized `../media/` relationship targets, and mixed text + inline image paragraphs — preserving `alignment` and figure `caption`.
  - Upgraded `docx_io::writer` to emit true media extensions (`imageN.jpg`/`.png`/`.bmp`), content types, and complete ECMA-376 `<pic:pic>` DrawingML (`<pic:nvPicPr>` + `<pic:spPr><a:xfrm>` + `<a:prstGeom prst="rect"/>`) so Word, LibreOffice, and Google Docs render RustPad-authored images natively.
  - Upgraded `pdf_create` to embed real ISO 32000 `/Subtype /Image` XObjects (`/DCTDecode` zero-recompression JPEG & `/FlateDecode` RGB PNG streams) with aligned `cm /ImN Do` placement and italicized captions.
- **Smooth `.docx` Image Insertion, Paste, Drag-and-Drop, Corner Resize & Contextual `IMAGE` Bar** (`crates/rustpad_window/`, `app/src/{commands,state,word_view,ui_paint,main}.rs`, `#40`):
  - Added `Image` ribbon button (`Ctrl+Shift+I`), inline empty-line `+ Image` chip, `/image <path>` & `/img` slash commands, native OS open-image dialogs (`GetOpenFileNameW` / `zenity` / `kdialog`) + workspace image picker modal, OS file drag-and-drop (`WM_DROPFILES`), and `Ctrl+V` screenshot/image clipboard paste (`CF_DIB` / `image/png`).
  - Added live click/hover figure selection, 1:1 mouse-locked aspect-preserving corner resize handles (`DragInteraction::ImageResize`), contextual `IMAGE` action bar (`25%` / `50%` / `75%` / `100%`, `Left` / `Center` / `Right`, `Caption`, `Replace`, `Delete`), and double-click / `+ Add figure caption…` inline modal.
- **Effortless Table Creation Studio: Visual `8×8` Grid Picker, Styled Presets, Slash/Pipe Shorthands & TSV Paste** (`crates/doc_model/src/lib.rs`, `app/src/{commands,state,word_view,ui_paint}.rs`, `index.html`, `src/`, `#36`, `#37`, `#38`):
  - Added interactive `8×8` visual hover/arrow-key grid popover (`Ctrl+Shift+T`), 5 1-click styled presets (`Simple Grid`, `Pricing / Invoice`, `Project Tracker`, `Comparison Matrix`, `Pros & Cons Split`), inline `+ Table` chip, `/table RxC` / `/invoice` / `| Col A | Col B |` auto-tables, Excel/Sheets TSV & Markdown table `Ctrl+V` paste, 5 table color palettes (`Linen`, `Cobalt`, `Sage`, `Terracotta`, `Minimal`), Zebra striping, Column Equalize, auto-replace header placeholders, and last-cell `Tab` auto-row insertion.

## [v2.1.0] - 2026-10-27 — Ultra-Smooth 60–120Hz Compositor & Flagship Human UX Release
### Added
- **60–120Hz Coalesced Native Compositor, Fast Scanline Fills & Pure-Rust PNG Figure Decoder** (`crates/rustpad_window/src/{win32,linux_x11}.rs`, `crates/rustpad_render/src/{canvas,png_codec}.rs`, `#31`):
  - Coalesced high-frequency Win32 `WM_MOUSEMOVE` / `WM_MOUSEWHEEL` redraw bursts into VSync-paced `WM_PAINT` frames (`needs_render` flag) and coalesced consecutive X11 `MotionNotify` bursts into 1 coordinate update per frame (`16ms` ~60Hz tick).
  - Accelerated `Surface::fill_rounded_rect` interior scanlines with contiguous `.fill(packed)` slice writes and added horizontal clip culling + inline row blitting in `Surface::draw_text`.
  - Implemented pure-Rust RFC 2083 `decode_png_rgba` (`IHDR`/`IDAT`, Sub/Up/Average/Paeth unfiltering, `64 MiB` safety cap) + bilinear `Surface::blit_rgba_scaled` with per-tab decoded RGBA caching for zero-allocation 60Hz scrolling over embedded `.docx` figures.
- **Live Text Range Selection, Substring `TextSpan` Styling & Floating Selection Mini-Toolbar** (`crates/doc_model/src/lib.rs`, `app/src/{state,word_view,commands}.rs`, `#32`):
  - Full `anchor..focus` range selection via zero-lag mouse drag (`WordLineHitBox`), double-click word selection (`word_range_at_char`), `Shift+Click`, `Shift+Arrow`/`Home`/`End`, `Ctrl+Left/Right` word jumps (`prev_word_char_idx` / `next_word_char_idx`), and `Ctrl+A` Select All.
  - Substring-accurate inline formatting (`Paragraph::transform_range`) + `paint_rich_spans_line` rendering mixed inline Bold/Serif/Mono/Size/Color/Highlight spans on a single line, plus selection-scoped Copy/Cut/Paste/Delete and AI Polish replacement.
  - **Warm Ivory Floating Selection Mini-Toolbar** (`B`, `I`, `U`, `Highlight`, `Ink`, `+Note`, `AI Polish`) blooming automatically above selected text.
- **60Hz Kinetic Spring Scroll, 1-Click Per-Clause Redline Review, Live Find Jump & Focus Mode** (`crates/doc_model/src/lib.rs`, `app/src/{state,word_view,ui_paint}.rs`, `#33`):
  - Critically-damped 60Hz `target_scroll_y` spring scrolling across Word and PDF tabs (while keeping scrollbar thumb dragging 1:1 instant).
  - 1-click per-clause `Accept #N` / `Reject #N` buttons (`Document::accept_revision` / `reject_revision`) in the Legal Redline Review Bar with SHA-256 `AuditTrail` logging.
  - Live `Ctrl+F` Find match counter + automatic selection jump on-sheet, and `Ctrl+Shift+F` **Distraction-Free Writer Focus Mode**.
- **UX-First Mobile Product Website & Interactive Smooth Sheet Sandbox** (`index.html`, `src/{main.ts,data/projectData.ts,styles/main.css}`, `#34`):
  - Added the `320px+` mobile-first **Interactive Smooth UX Sandbox** (`#ux-playground`) showcasing the Floating Selection Toolbar, substring `TextSpan` formatting, 1-click redline clause review, and Focus Mode live in the browser.

## [v2.0.0] - 2026-10-27 — Enterprise Offline Document & PDF Suite
### Added
- **Lossless OpenXML (`.docx`) Passthrough, ECMA-376 Track Changes & Threaded Review Comments** (`crates/doc_model/`, `crates/docx_io/`, `app/src/word_view.rs`, `#23`):
  - Byte-for-byte archive preservation (`passthrough_parts`) for corporate themes (`word/theme/*`), `customXml/*`, embedded media, and custom style parts.
  - Full `<w:ins>` and `<w:del><w:r><w:delText>` revision tracking with `word/settings.xml` `<w:trackRevisions/>`, live Track Changes typing (`Ctrl+Shift+E`), emerald/crimson redline rendering + left-margin change bars, `Accept All` / `Reject All`, and offline word-level contract comparison (`Document::diff_redline` / `rustpad redline`).
  - Standards-compliant `word/comments.xml` + `<w:commentRangeStart>` / `<w:commentRangeEnd>` / `<w:commentReference>` threaded review comments (`Ctrl+Shift+M`) and `EnterpriseContract` ("Enterprise MSA Redline") template.
- **Forensic PDF Redaction Sanitizer, PDF 1.5+ `/ObjStm` Unpacking, PDF/A-2b & SHA-256 Seals** (`crates/pdf_engine/`, `crates/pdf_create/`, `app/src/pdf_view.rs`, `#24`):
  - ISO 32000-1 §7.5.7 `/Type /ObjStm` compressed object stream unpacking + PNG row predictor (`Predictor >= 10`) unfiltering for modern Word/Acrobat/Chrome PDFs.
  - `PdfDocument::sanitize_redactions` physically purges intersecting underlying glyph runs and overlapping annotations from PDF content streams and scrubs document metadata (`rustpad sanitize`).
  - Archival **PDF/A-2b** (`pdfaid:part=2`, `conformance=B`) XMP metadata packets + `/Type /RustPadSeal` RFC 6234 SHA-256 tamper-evident sealing and CLI verification (`rustpad sign --pdfa`, `rustpad verify`).
- **Untrusted Parser Hardening & Pure-Rust RFC 6234 SHA-256** (`crates/rustpad_zip/`, `crates/docx_io/src/xml_lite.rs`, `#25`):
  - In-tree pure-Rust `sha256` / `sha256_hex` (`0` external crates).
  - Bounded DEFLATE/Zlib decompression (`deflate_decompress_bounded`), `ArchiveSecurityLimits` (`200:1` zip-bomb ratio & byte ceilings), strict `..` path-traversal rejection, and `MAX_XML_DEPTH = 128` XML recursion guard.
- **Enterprise MDM `policy.json` DLP Governance, Native WinHTTP TLS, Audit Chain & CycloneDX SBOM** (`crates/power_tools/src/{enterprise,ai_agent}.rs`, `#9`, `#26`):
  - Native Windows `winhttp.dll` (`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`) Schannel TLS transport honoring corporate PAC/WPAD proxies and Enterprise Root CAs without external `curl.exe` (`#9`).
  - System MDM `EnterprisePolicy` (`/etc/rustpad/policy.json`, `%ProgramData%\RustPad\policy.json`, `RUSTPAD_POLICY_PATH`) enforcing air-gapped cloud AI lockouts, security classification badges (`PUBLIC` / `INTERNAL` / `CONFIDENTIAL` / `RESTRICTED`), SHA-256 hash-chained `AuditTrail`, and `rustpad sbom` CycloneDX 1.5 JSON output.
- **Crash-Safe WAL Journal, Advisory Lockfiles, UIA/AT-SPI Accessibility Tree & OS Printing** (`crates/power_tools/src/enterprise.rs`, `crates/rustpad_window/src/lib.rs`, `#27`):
  - SHA-256-verified Write-Ahead Crash Recovery Journal (`WalJournal`), `.~<file>.rustpad.lock` SMB/NFS advisory lockfiles, RTL (`Arabic`/`Hebrew`) paragraph auto-alignment, `rustpad inspect --a11y` semantic tree export, and `Ctrl+P` native OS print spooler dispatch.
- **Enterprise Open-Source Governance Suite** (`SECURITY.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `docs/ENTERPRISE_POLICY.md`, `#28`): Complete threat model, 9-crate systems architecture blueprint, zero-dependency contributor charter, and MDM policy administration guide.

## [v1.3.0] - 2026-10-26
### Added
- **Pluggable Local & Cloud AI Agent Bridge** (`crates/power_tools/src/ai_agent.rs`, `app/src/{state,ui_paint,main}.rs`): Pure-Rust `TcpStream` HTTP/1.1 client for local air-gapped servers (`nour-code`, `Ollama` with `/api/tags` auto-discovery, `LM Studio`, `llama.cpp`) plus OS TLS bridge for `OpenAI`, `Groq`, `OpenRouter`, and `Anthropic Claude`, running on a non-blocking `std::thread` + `mpsc` worker (`Ctrl+J` GUI dock + `rustpad ai` CLI subcommand).
- **Interactive Multi-Line OpenXML Tables** (`crates/doc_model/src/lib.rs`, `app/src/{word_view,state,commands}.rs`): Word-wrapped multi-line table cells, in-cell blinking caret, contextual `TABLE` action sub-bar (`+Row Above/Below`, `+Col Left/Right`, `Header Bold`, `Shade`, `Delete`), on-canvas `+ Add Row` bar and `+` right column rail, draggable column dividers, and `Tab`/`Shift+Tab` cell traversal.
- **Architectural Paper & Linen Desktop Redesign & Ergonomics** (`docs/STYLE.md`, `crates/rustpad_render/`, `crates/rustpad_window/`, `app/src/`): Warm Paper & Linen color system, vector toolbar icons, floating Style/Font dropdown menus, asymmetric editorial Home Launchpad, drag-resizable/collapsible sidebars & split view, dynamic OS cursors (`IBeam`, `Hand`, `ResizeEW`), automatic OS HiDPI detection + UI scale steppers, and native OS clipboard (`Ctrl+C`/`X`/`V`).
- **Mobile-First Powerhouse Landing Page Redesign** (`index.html`, `src/styles/{tokens,main}.css`, `src/main.ts`, `src/data/projectData.ts`): Rebuilt from `320px` upward with a sticky top bar + animated drop-down Mobile Top Menu, zero horizontal overflow (`minmax(0, 1fr)`), interactive AI provider console, stacked mobile crate spec cards, and Carbon Obsidian / Architectural Paper dual theme.

### Fixed
- Fixed paragraph-start `Backspace` dropping text when merging into a zero-span paragraph (`#5`), dynamic `env!("CARGO_PKG_VERSION")` version sync across all 9 crates & About modal (`#6`), inverted `2,016 px` scroll jump & line-end caret click offset (`#10`), small HiDPI default font sizes (`#11`), table cell clipping (`#12`), and split-pane PDF clipping (`#13`).

## [v1.2.0] - 2026-10-26
### Added
- Automated multi-platform GitHub Actions release pipeline publishing standalone stripped Windows x86_64 (`rustpad-v1.2.0-windows-x86_64.exe` / `rustpad.exe`, `3.16 MB`) and Linux x86_64 (`rustpad-v1.2.0-linux-x86_64`, `3.18 MB`) executables with `SHA256SUMS.txt` verification hashes under `Nour-yahyaoui/rustpad`.
- Updated the **RustPad Landing Page** (`https://rustpad-studio.vercel.app`) and project docs with direct 1-click `v1.2.0` prebuilt binary download links, `16 Releases` ledger metrics, and author LinkedIn + Portfolio links.

## [v1.1.2] - 2026-10-26
### Added
- Windows `.exe` native PowerShell packaging (`scripts/build-windows-exe.ps1`), Linux MinGW-w64 cross-compile packaging (`scripts/build-windows-cross.sh`), and `docs/BUILD_WINDOWS_EXE.md`.

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
