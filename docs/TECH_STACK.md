# Technology Stack Blueprint (100% Pure Rust Native Desktop)

> **Status:** Planning Phase (Zero Code — Architecture & Stack Lock)  
> **Architecture:** 100% Standalone Compiled Native Desktop App (**Zero Web Server · Zero HTML/CSS/JS · Zero WebView**)  
> **Author:** Nour Yahyaoui ([GitHub](https://github.com/Nour-yahyaoui) · [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com))

---

## 1. Core Decision: 100% Pure Rust Native Desktop (`Rust` + `Iced` + `wgpu`)

This project is a **true native desktop executable** (`.exe` / `.msi` on Windows, `.dmg` / `.app` on macOS, `.AppImage` / `.deb` on Linux) built **end-to-end in Rust** using a single `Cargo` workspace — with **no web server, no browser engine, no JavaScript, and no external runtime**.

### Why `Iced` + `wgpu` + `cosmic-text` (vs Other Native GUI Toolkits)

| Criteria | **`Iced` + `wgpu` (Pure Rust)** ✅ *Selected* | `Slint` (Rust + `.slint` DSL) | Qt 6 (C++ / QML) | Electron / WebView |
| :--- | :--- | :--- | :--- | :--- |
| **100% Open-Source License** | **100% MIT** (matches our MIT repo with zero restrictions) | GPLv3 / Commercial / Attribution rules | LGPLv3 / Commercial | MIT, uses browser engine |
| **Language & Toolchain** | **100% Pure Rust (`cargo` only)** | Custom `.slint` DSL + Rust | C++20 + CMake + QML | JS/TS + HTML/CSS + Node |
| **Cold Startup Time** | **< 120 ms** (instant native window) | < 120 ms | ~300 ms | 400 ms – 3,000 ms |
| **Idle Memory (RAM)** | **~30 – 45 MB** | ~30 – 45 MB | ~55 – 90 MB | 80 – 400 MB |
| **Standalone Binary Size** | **~7 – 12 MB `.exe`** (ideal for 50+ GitHub Releases!) | ~8 – 14 MB | ~35 – 60 MB | 10 – 220 MB |
| **Custom Document Canvas** | **First-class `Canvas` & `wgpu` custom primitives + `cosmic-text`** | Harder for complex WYSIWYG paginated canvases | Strong, slower C++ iteration | DOM-based |
| **Micro-Release Modularity** | **Pure Elm-style `State / Message / update / view` per feature** | Good for forms, rigid for rich doc editors | Heavy build setup | Multi-toolchain |

**Why `Iced` wins specifically for Word + PDF:**
1. **Powered by `wgpu` (DirectX 12 / Metal / Vulkan) + `tiny-skia` fallback:** Renders our **"Paper & Daylight"** light theme, crisp elevation shadows, vector icons, and zoomed pages at **120 FPS** on modern GPUs while still running smoothly on older integrated laptops.
2. **Built on `cosmic-text`:** Pure-Rust rich-text shaping (OpenType ligatures, bidirectional Latin/Arabic/RTL/CJK text, per-span font family/size/weight/color, cursor hit-testing) — the exact foundation needed for a native paginated word processor.
3. **100% Pure Rust (`cargo build --release`):** One language, one compiler, zero npm/Node/CMake headaches, and tiny standalone binaries for every GitHub Release.

---

## 2. Complete Pure-Rust Crate Stack

### A. Native GUI, Windowing & Smooth Light Theme
| Component | Rust Crate(s) | Role in the Desktop App |
| :--- | :--- | :--- |
| **GUI Framework** | **`iced` (0.13+)** | Elm-inspired reactive native desktop UI, custom widgets, split panes, scrollable viewports, keyboard shortcuts, and multi-window/tab state. |
| **GPU Rendering** | **`wgpu` + `tiny-skia`** | Hardware-accelerated DirectX 12 / Metal / Vulkan rendering with automatic CPU software fallback. |
| **Typography & Shaping** | **`cosmic-text` + `fontdb` + `swash`** | Discovers all installed OS fonts, shapes rich multi-style paragraphs, handles cursor/selection geometry, and renders razor-sharp glyphs. |
| **Vector Icons** | **`iced_svg` / `resvg` (Lucide SVGs)** | Crisp scalable icons bundled directly into the `.exe` binary. |
| **OS File Dialogs & Drag-Drop** | **`rfd` (Rusty File Dialogs)** | Native Windows Explorer, macOS Finder, and Linux GTK/Portal Open/Save dialogs. |
| **OS Clipboard** | **`arboard`** | Native copy & paste for plain text, rich text, and images across desktop apps. |

### B. Word Studio Engine (Create & Edit `.docx`)
| Component | Rust Crate(s) | Role in the Desktop App |
| :--- | :--- | :--- |
| **Document Model & Rope** | **`ropey` + Custom `doc_model` crate** | Fast text buffer + structured rich-document AST (Sections, Paragraphs, Headings, Tables, Lists, Images, Headers/Footers, Comments, Undo/Redo stack). |
| **Paginated Page Layout** | **Custom `page_layout` crate (`cosmic-text`)** | Breaks paragraphs, tables, and images dynamically across physical **A4 / US Letter / Legal** sheets with live margins, rulers, and `Page X of Y` numbering. |
| **`.docx` OpenXML I/O** | **`zip` + `quick-xml` + `serde`** | Streaming ECMA-376 `.docx` archive reader & writer — opens and saves Microsoft Word files in milliseconds with styles, tables, lists, and media. |
| **Image Decoding/Encoding** | **`image`** | Handles PNG, JPEG, WebP, BMP, and TIFF images embedded inside `.docx` and `.pdf` documents. |
| **Offline Spellcheck** | **`spellbook`** | 100% pure-Rust Hunspell-compatible spellchecker with wavy red underlines and instant suggestions. |

### C. PDF Studio Engine (Create & Edit `.pdf`)
| Component | Rust Crate(s) | Role in the Desktop App |
| :--- | :--- | :--- |
| **120 FPS PDF Rendering** | **`pdfium-render` / `hayro` + LRU Tile Cache** | High-DPI page rasterization, text-run bounding box extraction, links, bookmarks, and form field inspection with virtualized viewport caching. |
| **Create Vector PDFs** | **`krilla` + `pdf-writer`** *(Typst ecosystem)* | 100% pure-Rust generation of print-ready vector PDFs with subsetted fonts, crisp tables, vector shapes, clickable TOC/links, and bookmarks. |
| **Structural PDF Editing** | **`lopdf` + `flate2`** | Pure-Rust lossless **Merge, Split, Extract, Reorder, Rotate, Delete pages**, PDF stream compression, watermarks, metadata scrubbing, and passwords. |
| **Content & Visual Editing** | **Custom `pdf_edit` + `iced::widget::Canvas`** | Click-to-edit PDF text runs, add text boxes/images/shapes, highlights, sticky notes, AcroForm filling, Bezier ink **Signatures**, and true **Redaction**. |

### D. Power Tools, Concurrency & Local Persistence
| Component | Rust Crate(s) | Role in the Desktop App |
| :--- | :--- | :--- |
| **Bidirectional Converter** | **`converter` crate (`pulldown-cmark`, `krilla`, `lopdf`)** | Native offline `Word (.docx) ↔ PDF (.pdf)`, `Markdown ↔ Word`, and `Images ↔ PDF` conversion pipelines. |
| **Offline Neural OCR** | **`ocrs` + `rten`** *(100% Pure Rust OCR!)* | Pure-Rust optical character recognition — turns scanned PDFs and camera photos into editable `.docx` or searchable PDFs offline with zero C++ dependencies. |
| **Local Time-Machine History** | **`rusqlite` (`bundled`) + `zstd`** | Local SQLite database + Zstd-compressed automatic document snapshots, crash recovery, recent files, and user settings. |
| **Multi-Core Workers** | **`rayon` + `tokio`** | Runs batch conversions, PDF compression, OCR, and background page rendering across all CPU cores without ever freezing the 120 FPS UI thread. |

### E. Build, Testing & Micro-Release Packaging
| Tool | Choice | Purpose |
| :--- | :--- | :--- |
| **Build System** | **`Cargo` Workspace** | Single unified build/test tool (`cargo build`, `cargo test --workspace`, `cargo clippy`). |
| **Desktop Installers** | **`cargo-packager` / `cargo-bundle`** | Produces native Windows `.exe` & `.msi`, macOS `.app` & `.dmg`, and Linux `.AppImage` & `.deb` installers. |
| **CI & Auto-Releases** | **GitHub Actions + `Swatinem/rust-cache` + `gh release`** | Pushing any `v*.*.*` git tag compiles optimized cross-platform desktop binaries and publishes a GitHub Release automatically. |
