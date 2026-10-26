# Planned Project Structure (Pure Rust Cargo Workspace)

> **Status:** Planning Blueprint Only — **No Rust code or `Cargo.toml` files are created yet.**  
> **Toolchain:** 100% Pure Rust (`cargo`) — Zero `node_modules`, Zero `package.json`, Zero Web Files  
> **Author:** Nour Yahyaoui ([GitHub](https://github.com/Nour-yahyaoui) · [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com))

---

## 1. Repository Layout Overview

When coding starts (incrementally across tagged releases), the repository will be organized as an idiomatic **Pure-Rust Cargo Workspace** separating the **Desktop UI (`app/`)** from clean, testable **Engine Crates (`crates/*`)**:

```text
word/
├── README.md                          # Project vision, Pure-Rust stack summary & author info
├── LICENSE                            # MIT Open-Source License (Nour Yahyaoui)
├── CHANGELOG.md                       # Release-by-release history (v0.0.1 → v1.0.0)
│
├── docs/                              # Complete Planning Blueprints (Current Phase)
│   ├── TECH_STACK.md                  # Pure-Rust GUI (Iced + wgpu) & engine crates
│   ├── ARCHITECTURE.md                # System diagram, Word/PDF pipelines & memory budget
│   ├── PROJECT_STRUCTURE.md           # Annotated Cargo workspace layout (this file)
│   ├── UI_UX_LIGHT_THEME.md           # "Paper & Daylight" Iced theme tokens & UX wireframes
│   └── RELEASE_AND_ROADMAP.md         # Commit → Tag → Release workflow & 48-step roadmap
│
├── .github/                           # [Planned] GitHub CI & Automated Binary Releases
│   ├── ISSUE_TEMPLATE/                # Bug report & feature request forms
│   └── workflows/
│       ├── ci.yml                     # cargo fmt, cargo clippy, cargo test --workspace
│       └── release.yml                # Builds .exe / .msi / .dmg / .AppImage on v*.*.* tag
│
├── assets/                            # [Planned] Embedded static desktop assets
│   ├── fonts/                         # Bundled Inter & JetBrains Mono OpenType fonts
│   ├── icons/                         # App icons (.ico, .icns, .png) & Lucide SVG set
│   └── templates/                     # Built-in Resume, Report, Letter & Invoice .docx/.pdf
│
├── Cargo.toml                         # [Planned] Root Cargo Workspace manifest
│
├── app/                               # [Planned] Main Native Desktop Binary Crate (Iced + wgpu)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                    # Native desktop entry point (zero console window on Win)
│       ├── app.rs                     # Root Iced Application state, update() & subscriptions
│       ├── registry.rs                # Central CommandRegistry & keyboard shortcut router
│       │
│       ├── theme/                     # Smooth, Fast "Paper & Daylight" Native Theme
│       │   ├── palette.rs             # Exact RGB/Hex color tokens (Alabaster, Paper, Cobalt, Coral)
│       │   ├── styles.rs              # Custom Iced button, toolbar, card, tab & shadow styles
│       │   └── typography.rs          # UI font loader & scale constants
│       │
│       ├── shell/                     # Window Chrome & Workspace Containers
│       │   ├── titlebar.rs            # Unified multi-tab bar (.docx & .pdf tabs)
│       │   ├── contextual_bar.rs      # Clean progressive top toolbar (3-click max UX)
│       │   ├── command_palette.rs     # Instant Ctrl+K universal tool launcher
│       │   ├── split_pane.rs          # Side-by-side PDF + Word dual workspace
│       │   └── status_bar.rs          # Local snapshot badge, word count, page X/Y & zoom pill
│       │
│       └── views/                     # Feature Views (1 submodule ≈ 1+ micro-releases!)
│           ├── home_hub.rs            # 1-click Quick Actions, Templates & Recent Files
│           ├── word_studio/           # Interactive Paginated Word Editor UI
│           │   ├── sheet_canvas.rs    # wgpu/Canvas physical A4/Letter sheets, rulers & cursor
│           │   ├── format_tools.rs    # Font, size, B/I/U, colors, alignment, spacing, lists
│           │   ├── table_overlay.rs   # Table grid picker, cell resize & contextual table pill
│           │   ├── media_handles.rs   # Image drag-drop, corner resize & captions
│           │   └── outline_rail.rs    # Live document headings navigator & Find/Replace bar
│           │
│           ├── pdf_studio/            # Interactive PDF Creation & Editing Studio UI
│           │   ├── pdf_viewport.rs    # Virtualized 120 FPS page renderer & text selection
│           │   ├── thumbnail_rail.rs  # Drag-reorder, rotate, extract & delete page thumbnails
│           │   ├── text_editor.rs     # Click-to-edit PDF text blocks & add new text boxes
│           │   ├── annotations.rs     # Highlighter, underline, sticky notes, shapes & redaction
│           │   ├── signature_pad.rs   # Smooth Bezier ink signature drawer, typed & image stamps
│           │   └── page_dialogs.rs    # 2-click Merge PDFs, Split PDF, Compress & Watermark UI
│           │
│           └── power_tools/           # Cross-Format Power UI
│               ├── converter_modal.rs # Word ↔ PDF, Markdown ↔ Word, Images ↔ PDF UI
│               ├── ocr_dialog.rs      # Offline scanned PDF → Editable Word / Searchable PDF
│               ├── snapshot_drawer.rs # Visual Time-Machine local version restore timeline
│               └── batch_studio.rs    # Multi-file batch compress, convert & watermark queue
│
├── crates/                            # [Planned] Pure-Rust Domain & File Engine Crates
│   ├── doc_model/                     # Structured Document AST, Styles, Tables & Undo/Redo
│   ├── page_layout/                   # cosmic-text A4/Letter pagination, margins & hit-testing
│   ├── docx_io/                       # Fast ECMA-376 OpenXML (.docx) reader & writer
│   ├── pdf_render/                    # High-DPI PDF page rasterizer, text bounds & tile cache
│   ├── pdf_create/                    # Pure-Rust vector PDF builder (krilla / pdf-writer)
│   ├── pdf_edit/                      # lopdf merge, split, rotate, text edit, sign, redact, shrink
│   ├── converter/                     # Bidirectional DOCX ↔ PDF, Markdown & Image pipelines
│   ├── ocr_engine/                    # Pure-Rust offline OCR worker (ocrs + rten)
│   └── snapshot_store/                # Embedded SQLite (rusqlite) + Zstd local history & prefs
│
└── tests/                             # [Planned] Integration & Round-Trip Fidelity Fixtures
    ├── fixtures/                      # Sample .docx and .pdf test files
    └── roundtrip/                     # Automated DOCX/PDF load → edit → save verification
```

---

## 2. Crate Dependency & Modularity Rules

To guarantee fast incremental `cargo` builds and effortless micro-releases:

1. **Engine Crates Have Zero GUI Dependencies:**  
   `crates/*` never depend on `iced` or `wgpu`. Every engine crate (`docx_io`, `pdf_edit`, `converter`, `page_layout`) can be unit-tested headlessly in milliseconds via `cargo test -p <crate>`.
2. **Strict One-Way Data Flow:**  
   `app/` depends on `crates/*`, never the reverse. `crates/doc_model` sits at the base as the lightweight shared document representation used by `docx_io`, `page_layout`, `pdf_create`, and `converter`.
3. **Centralized Daylight Theme Tokens:**  
   All widgets in `app/src/views/*` pull colors, spacing, border radii, and subtle paper shadows strictly from `app/src/theme/` so the Smooth Light Theme remains consistent across every tool.
4. **1 Feature Slice = 1 Clean Commit, Tag & Release:**  
   Each module above maps directly to a dedicated version tag in [`docs/RELEASE_AND_ROADMAP.md`](RELEASE_AND_ROADMAP.md).
