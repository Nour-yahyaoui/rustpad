# System Architecture Blueprint (Pure Rust Native Desktop)

> **Status:** Planning Phase (Zero Code)  
> **Runtime Model:** Single Standalone Native Desktop Executable (`Rust` + `Iced` + `wgpu`) — Zero Web Server, Zero Browser/WebView  
> **Author:** Nour Yahyaoui ([GitHub](https://github.com/Nour-yahyaoui) · [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com))

---

## 1. Architectural Pillars

1. **100% Native Compiled Desktop Process:**  
   The app compiles directly to native machine code (`.exe` on Windows, Mach-O `.app` on macOS, ELF on Linux). Double-clicking the icon or a `.docx`/`.pdf` file opens a GPU-accelerated desktop window in **under 120 ms**.
2. **Cargo Multi-Crate Modularity (Built for Micro-Releases):**  
   Every domain engine (`doc_model`, `page_layout`, `docx_io`, `pdf_render`, `pdf_create`, `pdf_edit`, `converter`, `ocr_engine`, `snapshot_store`) is its own isolated Rust library crate. Adding a small feature touches only one crate + one UI module, making our **Commit → Tag → Release** workflow clean and regression-free.
3. **Unidirectional Elm Architecture (`State → Message → update → Task → view`):**  
   `Iced` enforces clean state transitions. Heavy file I/O, PDF rendering, compression, and OCR run asynchronously inside `iced::Task` / `rayon` worker pools and stream progress messages back to the 120 FPS UI thread.
4. **100% Offline & Local-First:**  
   Zero servers, zero network ports, zero cloud accounts, zero telemetry. Everything runs locally on the user's machine.

---

## 2. Layered System Diagram

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                 NATIVE DESKTOP EXECUTABLE (100% Pure Rust)                   │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  UI PRESENTATION LAYER (iced + wgpu + cosmic-text — 120 FPS Daylight)  │  │
│  │  • Window Shell & Unified Tab Bar    • Adaptive Contextual Top Bar     │  │
│  │  • Ctrl+K Command Palette Overlay    • Split-Screen Dual Pane Host     │  │
│  └───────────────┬────────────────────────────────────────┬───────────────┘  │
│                  │                                        │                  │
│                  ▼                                        ▼                  │
│  ┌───────────────────────────────────┐    ┌───────────────────────────────┐  │
│  │   WORD CANVAS WIDGET (.docx)      │    │   PDF STUDIO WIDGET (.pdf)    │  │
│  │ • Paginated A4/Letter wgpu Sheets │    │ • Virtualized Tile Viewport   │  │
│  │ • cosmic-text Rich Paragraph Runs │    │ • Text-Block Click-to-Edit    │  │
│  │ • Interactive Tables, Lists, Media│    │ • Highlights, Ink & Signatures│  │
│  │ • Headers, Footers & Rulers       │    │ • Thumbnail Drag-Drop Rail    │  │
│  └───────────────┬───────────────────┘    └───────────────┬───────────────┘  │
│                  │                                        │                  │
│                  └───────────────────┬────────────────────┘                  │
│                                      ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │         CENTRAL RUST COMMAND REGISTRY & ICED MESSAGE DISPATCHER        │  │
│  └───────────────────────────────────┬────────────────────────────────────┘  │
│                                      │ Async iced::Task + rayon Thread Pool  │
│                                      ▼                                       │
│  ┌──────────────────┐  ┌──────────────────────┐  ┌────────────────────────┐  │
│  │  WORD CRATES     │  │     PDF CRATES       │  │   POWER TOOL CRATES    │  │
│  │ • doc_model      │  │ • pdf_render         │  │ • converter (DOCX↔PDF) │  │
│  │ • page_layout    │  │ • pdf_create (krilla)│  │ • ocr_engine (ocrs)    │  │
│  │ • docx_io (zip + │  │ • pdf_edit (lopdf)   │  │ • spellcheck           │  │
│  │   quick-xml)     │  │   merge/split/sign   │  │ • batch_processor      │  │
│  └────────┬─────────┘  └──────────┬───────────┘  └───────────┬────────────┘  │
│           └───────────────────────┼──────────────────────────┘               │
│                                   ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │         LOCAL STORAGE CRATE (snapshot_store: rusqlite + zstd)          │  │
│  │  • Time-Machine Document Snapshots  • Crash Journal  • Recent Files    │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Native Engine Pipelines

### 3.1 Word Engine Pipeline (`docx_io` ↔ `doc_model` ↔ `page_layout`)

```text
.docx File ──► docx_io (zip + quick-xml) ──► doc_model (Rich AST + Undo Stack)
                                                      │
                                                      ▼
                                            page_layout (cosmic-text)
                                            Breaks AST into physical A4/Letter
                                            sheets, lines, table cells & margins
                                                      │
                                                      ▼
                                            WordCanvas Widget (iced + wgpu)
```

- **Document Model (`crates/doc_model`):**
  - Stores structured `Section` → `Block` (`Paragraph`, `Heading`, `Table`, `List`, `ImageBlock`, `PageBreak`) → `Inline` (`StyledTextRun`, `Hyperlink`, `InlineImage`, `CommentAnchor`).
  - Every edit is an atomic `EditCommand` pushed onto an Undo/Redo ring buffer so `Ctrl+Z` / `Ctrl+Y` is instant and rock-solid.
- **Real Pagination (`crates/page_layout`):**
  - Uses `cosmic-text` to measure exact shaped glyph widths and line heights against the chosen page geometry (e.g., A4 `210 × 297 mm` minus top/bottom/left/right margins).
  - Automatically flows overflowing lines, table rows, and images onto Page 2, Page 3, ..., injecting running headers, footers, and page numbers (`Page X of Y`).
- **Native `.docx` Read/Write (`crates/docx_io`):**
  - Streams `word/document.xml`, `word/styles.xml`, `word/numbering.xml`, relationships, headers/footers, and `word/media/*` directly from/to disk in milliseconds.

### 3.2 PDF Studio Pipeline (`pdf_render` + `pdf_create` + `pdf_edit`)

1. **Create PDF (`crates/pdf_create`):**
   - Converts any `doc_model::Document` (or Blank PDF Canvas / Image set) directly into a crisp **vector PDF** via pure-Rust `krilla` / `pdf-writer`, embedding subsetted OpenType fonts, vector table borders, hyperlinks, and PDF outline bookmarks.
2. **View & Inspect PDF (`crates/pdf_render`):**
   - Renders visible pages (+ a 2-page scroll buffer) into GPU textures via `wgpu`, keeping memory flat (`< 80 MB`) even when scrolling smoothly through a **1,000-page PDF at 120 FPS**.
   - Extracts selectable text runs, character bounding boxes, hyperlinks, and AcroForm widgets for interactive selection and editing.
3. **Edit PDF Content & Structure (`crates/pdf_edit`):**
   - **On-Page Content Editing:** Click existing PDF text blocks to modify text/size/color, insert new text boxes/images/shapes, draw smooth Bezier ink **Signatures**, highlight/annotate, fill forms, or apply permanent **Redactions** (scrubbing underlying content stream operators).
   - **Structural Page Operations (`lopdf`):** Instant lossless **Merge PDFs**, **Split PDF**, **Extract/Delete/Rotate/Reorder pages**, **Compress streams/images**, **Add Watermarks**, and **Password Encrypt/Unlock**.

### 3.3 Power Tools Pipeline (`converter`, `ocr_engine`, `snapshot_store`)

- **`DOCX ↔ PDF` Smart Converter (`crates/converter`):**
  - *Word → PDF:* Direct `doc_model` → `pdf_create` vector export.
  - *PDF → Editable Word:* Analyzes PDF text-run coordinates, font weights/sizes, column alignment, rule lines, and embedded bitmaps to reconstruct real paragraphs, headings, tables, and images inside a clean `.docx` file.
- **Pure-Rust Offline OCR (`crates/ocr_engine`):**
  - Powered by `ocrs` + `rten` (pure Rust neural text detection + recognition). Converts scanned PDF pages or photos of paper documents into either a **Searchable PDF** (invisible text layer aligned over scan) or an **Editable `.docx`**.
- **Time-Machine Local Snapshots (`crates/snapshot_store`):**
  - Every few minutes (and before destructive operations), a Zstd-compressed snapshot is saved into a local `rusqlite` store so users can visually scrub back in time and recover any prior version offline.

---

## 4. Central Rust Command Registry (Why Micro-Releases Stay Effortless)

Every tool in the desktop app implements a clean Rust trait/descriptor:

- **Command ID:** e.g., `"word.bold"`, `"word.insert_table"`, `"pdf.merge"`, `"pdf.sign"`, `"tools.ocr"`
- **Label, Icon & Shortcut:** Shown automatically in Toolbars, Menus, Tooltips, and the `Ctrl+K` Command Palette.
- **Context Filter:** `Global`, `WordActive`, or `PdfActive`.
- **Execution:** Dispatches a strongly-typed `AppMessage` to the corresponding crate/module.

Because tools are decoupled via `CommandRegistry`, each micro-release (`v0.2.3`, `v0.4.2`, `v0.5.4`, ...) simply drops in one new self-contained Rust module, registers its command, passes `cargo test`, and ships!

---

## 5. Performance & Resource Targets

| Metric | Target Budget | How Pure Rust Achieves It |
| :--- | :--- | :--- |
| **Cold Startup to Window** | **< 120 ms** | Native machine code binary, zero VM/JIT/browser initialization. |
| **UI & Scroll Frame Rate** | **120 FPS** | `wgpu` GPU instanced quads/glyphs + virtualized page rendering. |
| **Idle Memory Usage** | **30 – 45 MB** | Zero garbage collector, zero Chromium/WebView process overhead. |
| **500-Page File Open Time** | **< 150 ms to Page 1** | Streaming ZIP/PDF index read; renders visible page 1 immediately while background `rayon` threads index remaining pages. |
| **Release Binary Size** | **~7 – 12 MB** | `lto = "fat"`, `codegen-units = 1`, `strip = true` in release `Cargo.toml`. |
