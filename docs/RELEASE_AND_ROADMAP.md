# Micro-Release Strategy & Step-by-Step Roadmap

> **Core Workflow Rule:** *Every small thing added = 1 Focused Commit + 1 SemVer Git Tag + 1 GitHub Release.*  
> **Toolchain:** 100% Pure Rust (`Cargo` Workspace → Standalone `.exe` / `.dmg` / `.AppImage` Releases)  
> **Author:** Nour Yahyaoui ([GitHub](https://github.com/Nour-yahyaoui) · [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com))

---

## 1. The "Living Repository" Release Workflow

Instead of disappearing for months to write a giant monolith, this project grows **visibly, step by step**, where every small module or tool immediately ships its own milestone:

```text
1 Small Feature Slice  ──►  cargo test  ──►  Conventional Commit  ──►  Git Tag (v0.x.y)  ──►  GitHub Release + Standalone Binary
```

### Why Pure Rust Makes Frequent Releases Shine
- **Tiny ~7–12 MB Native Executables:** Every GitHub Release attaches a crisp, lightweight standalone `.exe` / `.dmg` / `.AppImage` — no 200 MB browser bundle bloat.
- **Crate-Level Isolation:** Working on `crates/pdf_edit` (e.g., adding *Split PDF*) cannot break `crates/docx_io` because Cargo isolates crate boundaries and runs fast unit tests per crate.

### Commit & Versioning Convention
- **Commits:** Conventional Commits (`docs(plan): ...`, `feat(shell): ...`, `feat(word): ...`, `feat(pdf): ...`, `fix(docx): ...`)
- **Tags & Releases:**
  - `v0.0.x` — **Phase 0:** Planning, Architecture Blueprints & Release Pipeline
  - `v0.1.x` — **Phase 1:** Native `Iced + wgpu` Window Shell & Smooth Daylight Theme
  - `v0.2.x` — **Phase 2:** Paginated Word Studio (`cosmic-text` + A4/Letter Canvas + Rich Formatting)
  - `v0.3.x` — **Phase 3:** Native `.docx` Engine (`docx_io`) & Pure-Rust Vector PDF Creation (`pdf_create`)
  - `v0.4.x` — **Phase 4:** PDF Studio — 120 FPS Viewer & Lossless Page Tools (Merge, Split, Rotate, Compress)
  - `v0.5.x` — **Phase 5:** PDF Content Editing, Annotations, Forms, Signatures & Redaction
  - `v0.6.x` — **Phase 6:** Power Tools (`PDF ↔ DOCX` Converter, Pure-Rust Offline OCR, Split View, Time-Machine Snapshots)
  - `v1.0.0` — **Stable v1.0.0 Flagship Desktop Release** (Windows, macOS, Linux)

---

## 2. Complete 48-Step Micro-Release Roadmap

Each release below is intentionally small, self-contained, and verifiable on its own:

### Phase 0 — Planning & Foundation (`v0.0.x`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.0.1` | `README` + `LICENSE` | Project vision, Pure-Rust direction, MIT license & author metadata (Nour Yahyaoui) |
| `v0.0.2` | `docs/` Architecture | `TECH_STACK.md`, `ARCHITECTURE.md` & `PROJECT_STRUCTURE.md` Cargo workspace blueprint |
| `v0.0.3` | `docs/` UX & Roadmap | `UI_UX_LIGHT_THEME.md` ("Paper & Daylight" spec) & `RELEASE_AND_ROADMAP.md` (48-step plan) |
| `v0.0.4` | `.github/` | GitHub Issue templates, `CHANGELOG.md` & automated tag-to-release GitHub Actions workflow |

### Phase 1 — Native `Iced` Shell & Smooth Light Theme (`v0.1.x`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.1.0` | `app` | Minimal Cargo workspace + `Iced` (`wgpu`) native desktop window booting in `< 120 ms` |
| `v0.1.1` | `app::theme` | **Paper & Daylight** native `iced::Theme` palette, bundled `Inter` font & custom button/card styles |
| `v0.1.2` | `app::shell::titlebar` | Unified multi-tab document bar with `DOCX` (Cobalt) & `PDF` (Coral) tab badges |
| `v0.1.3` | `app::views::home_hub` | Calm 1-click Home Launch Hub (Hero Quick Actions + Recent Local Files panel) |
| `v0.1.4` | `app::registry` | Central Rust `CommandRegistry`, keyboard shortcut router & `Ctrl+K` Command Palette |
| `v0.1.5` | `app::shell` | Collapsible Left Outline/Thumbnails Rail, Bottom Status Bar & smooth `50%–200%` Zoom pill |

### Phase 2 — Word Creation & Paginated Editing (`v0.2.x`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.2.0` | `crates/doc_model` | Structured rich-document AST, atomic edit commands & instant Undo/Redo stack (`Ctrl+Z`/`Ctrl+Y`) |
| `v0.2.1` | `crates/page_layout` | `cosmic-text` A4/Letter floating white paper sheet on Alabaster desk with blinking cursor & typing |
| `v0.2.2` | `word_studio` | Top Contextual Bar: OS Font Family picker, Font Size, Bold, Italic, Underline & Strikethrough |
| `v0.2.3` | `word_studio` | Text color picker, soft highlight swatches, Left/Center/Right/Justify alignment & line spacing |
| `v0.2.4` | `word_studio` | Document Styles gallery (`Title`, `Heading 1–4`, `Normal`, `Quote`) + live Left Outline jump tree |
| `v0.2.5` | `word_studio` | Bulleted lists, numbered lists, interactive checklists & `Tab` / `Shift+Tab` indentation |
| `v0.2.6` | `crates/page_layout` | Automatic multi-page flow, manual Page Breaks (`Ctrl+Enter`), A4/Letter/Legal sizes & live margins |
| `v0.2.7` | `word_studio` | **Tables Engine** — grid picker, cell editing, row/column insert/delete, borders & cell shading |
| `v0.2.8` | `word_studio` | Native Image insertion (`rfd` + drag-and-drop), corner resize handles, alignment & captions |
| `v0.2.9` | `word_studio` | Running Headers & Footers, `Page X of Y` numbering, live Word Count, Find/Replace & `spellbook` spellcheck |

### Phase 3 — Native `.docx` Engine & Vector PDF Creation (`v0.3.x`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.3.0` | `crates/docx_io` | Pure-Rust `.docx` writer — save formatted documents to standard Microsoft Word `.docx` files |
| `v0.3.1` | `crates/docx_io` | Pure-Rust `.docx` reader — open existing `.docx` files with headings, paragraphs & inline styles |
| `v0.3.2` | `crates/docx_io` | Full `.docx` round-trip fidelity for tables, embedded images, lists, headers/footers & page margins |
| `v0.3.3` | `assets/templates` | Built-in Starter Templates gallery (Clean Resume, Academic Paper, Formal Letter, Business Invoice) |
| `v0.3.4` | `crates/pdf_create` | Pure-Rust **Vector PDF Creator** (`krilla` / `pdf-writer`) — export any Word doc to crisp vector `.pdf` |
| `v0.3.5` | `crates/pdf_create` | **Blank PDF Studio Creator** — build new multi-page PDFs from scratch with text, shapes & images |
| `v0.3.6` | `word_studio` | Margin Comments thread + native OS Print integration (`Ctrl+P`) |

### Phase 4 — PDF Studio: 120 FPS Viewing & Structural Page Tools (`v0.4.x`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.4.0` | `crates/pdf_render` | Virtualized 120 FPS high-DPI PDF viewer (`wgpu` tile cache) inside unified app tabs |
| `v0.4.1` | `pdf_studio` | Crisp PDF text selection, copy to clipboard (`arboard`), clickable links & PDF Bookmarks tree |
| `v0.4.2` | `pdf_studio` | Left Page Thumbnail Organizer — drag-and-drop reorder pages, 1-click rotate 90° & delete page |
| `v0.4.3` | `crates/pdf_edit` | **Merge PDFs Tool (`lopdf`)** — combine multiple PDFs and custom page ranges in 2 clicks |
| `v0.4.4` | `crates/pdf_edit` | **Split & Extract PDF Tool** — split by range, every N pages, or extract selected thumbnails |
| `v0.4.5` | `crates/pdf_edit` | **Compress PDF Tool** — stream & image optimization with instant before/after file-size preview |
| `v0.4.6` | `crates/pdf_edit` | Custom PDF **Watermarks**, **Page Numbering**, and page background tinting |

### Phase 5 — PDF Content Editing, Annotations & Signatures (`v0.5.x`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.5.0` | `pdf_studio` | PDF Highlighter, Underline, Strikethrough & Sticky Notes saved as standard PDF annotations |
| `v0.5.1` | `pdf_studio` | Smooth freehand ink pen, vector rectangles/arrows/callouts & image stamping on PDF pages |
| `v0.5.2` | `crates/pdf_edit` | **Inline PDF Text Block Editor** — click existing PDF text lines to edit wording, font size & color |
| `v0.5.3` | `crates/pdf_edit` | Interactive **PDF Form Filler** (text fields, checkboxes, radios, dropdowns) + Flatten Form |
| `v0.5.4` | `signature_pad` | **Signature Studio** — draw smooth Bezier ink signatures, type calligraphic names, or stamp images |
| `v0.5.5` | `crates/pdf_edit` | True **PDF Redaction** (permanently scrubs underlying text/image streams), Passwords & Metadata Cleaner |

### Phase 6 — Power Tools & v1.0.0 Flagship Release (`v0.6.x` → `v1.0.0`)
| Tag | Module / Crate | Deliverable |
| :--- | :--- | :--- |
| `v0.6.0` | `crates/converter` | **PDF → Editable Word (`.docx`) Converter** reconstructing headings, paragraphs, tables & images |
| `v0.6.1` | `crates/converter` | **Markdown ↔ `.docx`** (`pulldown-cmark`) & **Images (`PNG/JPG/WebP`) ↔ `.pdf`** converters |
| `v0.6.2` | `split_pane` | **Side-by-Side Split Workspace** — read/highlight a PDF on the left while writing a `.docx` on the right |
| `v0.6.3` | `snapshot_store` | **Local Time-Machine Snapshots** (`rusqlite` + `zstd` crash recovery & 1-click visual version rewind) |
| `v0.6.4` | `crates/ocr_engine` | **Pure-Rust Offline OCR (`ocrs`)** — convert scanned PDFs & photos into editable `.docx` or searchable PDF |
| `v0.6.5` | `batch_studio` | Multi-core `rayon` **Batch File Studio** (convert, compress, or watermark dozens of files in parallel) |
| `v0.9.0` | Release Candidate | OS `.docx`/`.pdf` double-click file associations, installer polish & 1,000-page stress benchmarks |
| `v1.0.0` | Stable Release 🎉 | **Official v1.0.0 Native Desktop Release** (Windows `.exe`/`.msi`, macOS `.dmg`, Linux `.AppImage`/`.deb`) |
