# RustPad (`rustpad`)

<p align="left">
  <img src="assets/icons/icon-256.png" alt="RustPad logo" width="84" height="84" />
</p>

**Create, edit, sign, redact, convert, and AI-assist Microsoft Word (`.docx`) and vector PDF (`.pdf`) documents offline-first in a single `~3.8 MB` pure-Rust desktop binary.**

[![Committers.top Tunisia Badge](https://user-badge.committers.top/tunisia/Nour-yahyaoui.svg)](https://committers.top/tunisia#Nour-yahyaoui)

- **Author**: Nour Yahyaoui — [LinkedIn](https://www.linkedin.com/in/nour-yahyaoui-86987b36b) · [GitHub](https://github.com/Nour-yahyaoui) · [Portfolio](https://nour-yahyaoui.vercel.app) · `nourryahyaoui@gmail.com`
- **Live Mobile-First Product Page**: [https://rustpad-studio.vercel.app](https://rustpad-studio.vercel.app)
- **Current Release (`v1.3.0`)**: [Windows x64 `.exe`](https://github.com/Nour-yahyaoui/rustpad/releases/download/v1.3.0/rustpad-v1.3.0-windows-x86_64.exe) · [Linux x64](https://github.com/Nour-yahyaoui/rustpad/releases/download/v1.3.0/rustpad-v1.3.0-linux-x86_64) · [SHA256SUMS.txt](https://github.com/Nour-yahyaoui/rustpad/releases/download/v1.3.0/SHA256SUMS.txt) · [All Releases](https://github.com/Nour-yahyaoui/rustpad/releases/tag/v1.3.0)
- **Stack**: 100% Pure Rust (`~19,500` LOC across 9 workspace crates, zero WebView, zero external `crates.io` runtime dependencies) + Mobile-First Vite TypeScript Product Showcase

![RustPad v1.3.0 Word Studio with Warm Architectural Paper & Linen chrome, contextual Table Studio bar, multi-line wrapped .docx table, and Pluggable AI Agent Studio Dock](docs/screenshots/word-studio.png)

---

## Why RustPad Exists

Desktop document tools routinely ship 180–400 MB Chromium wrappers or cloud-locked editors just to draft a `.docx` technical specification, adjust a table, or sign and merge a `.pdf`. **RustPad** compiles every subsystem—RFC 1951 DEFLATE/PKZIP compression, ECMA-376 OpenXML serialization, ISO 32000 PDF 1.7 vector streams, TrueType glyph rasterization, Win32/X11 native windowing, and a pluggable local/cloud AI HTTP bridge—into one standalone `~3.8 MB` executable with a calm **Warm Architectural Paper & Linen** interface.

---

## Quick Start

### Option A — Download Prebuilt Native Binary (`v1.3.0`, Zero Install)

- **Windows 10 / 11 (x86_64)**: Download [`rustpad-v1.3.0-windows-x86_64.exe`](https://github.com/Nour-yahyaoui/rustpad/releases/download/v1.3.0/rustpad-v1.3.0-windows-x86_64.exe) and double-click to launch.
- **Linux (x86_64)**: Download [`rustpad-v1.3.0-linux-x86_64`](https://github.com/Nour-yahyaoui/rustpad/releases/download/v1.3.0/rustpad-v1.3.0-linux-x86_64), then run `chmod +x rustpad-v1.3.0-linux-x86_64 && ./rustpad-v1.3.0-linux-x86_64`.
- **SHA-256 Checksums**: [`SHA256SUMS.txt`](https://github.com/Nour-yahyaoui/rustpad/releases/download/v1.3.0/SHA256SUMS.txt)

### Option B — Build & Run from Source (Linux, Windows, macOS)

Requires only a standard Rust toolchain (`rustc` + `cargo` 1.75+). Compiles offline with zero `crates.io` fetches:

```bash
git clone https://github.com/Nour-yahyaoui/rustpad.git
cd rustpad

# Run workspace unit tests across all 9 crates
cargo test --workspace

# Build stripped native release executable
cargo build --release -p rustpad

# Launch the native desktop studio
./target/release/rustpad
```

### Option C — Run Headless CLI & Local/Cloud AI Pipelines

```bash
# Run a local AI agent (nour-code / Ollama / LM Studio) on any .docx, .pdf, or .md file
./target/release/rustpad ai spec.docx --provider nour-code --task summarize -o summary.md
./target/release/rustpad ai spec.docx --provider ollama --model qwen2.5:7b --task sections -o spec_v2.docx

# Convert between .docx, .pdf, and .md offline
./target/release/rustpad convert examples/RustPad_Welcome_Guide.docx Welcome_Export.pdf

# Inspect OpenXML or PDF structure & word/page metrics
./target/release/rustpad inspect examples/RustPad_Welcome_Guide.docx
```

---

## Core Capabilities (`v1.3.0`)

### 1. Word Studio (`.docx`) & Interactive Wrapped Tables

![RustPad v1.3.0 Asymmetric Editorial Home Launchpad](docs/screenshots/home-hub.png)

- **Comfortable HiDPI Typography & Smooth Input**: `12.5 pt` default body text at `112%` paper zoom, automatic OS DPI detection (`GetDpiForWindow` / `Xft.dpi`), live `UI- / 100% / UI+` scale steppers (`Ctrl+=` / `Ctrl+-` / `Ctrl+0`), character-exact glyph hit-testing, 1:1 non-inverted mouse wheel scrolling, draggable scrollbar, and native OS clipboard (`Ctrl+C` / `Ctrl+X` / `Ctrl+V`).
- **First-Class Multi-Line Tables**: Cells word-wrap cleanly and grow row height automatically. Click inside any table cell to reveal the contextual **`TABLE` sub-bar** (`+ Row Below`, `+ Row Above`, `+ Col Right`, `+ Col Left`, `Header Bold`, `Cell Shade`, `Del Row`, `Del Col`), click the on-canvas **`+ Add Row`** bar or right **`+`** column rail, drag vertical column dividers (`ResizeEW`), or navigate cells with `Tab` / `Shift+Tab`.
- **Drag-Resizable Outline & Time-Machine Snapshots**: Drag-resize or collapse the left Document Outline rail (`148px–380px`), click any heading to jump to its exact layout offset, and capture DEFLATE-compressed version snapshots in session memory.

### 2. Pluggable Local & Cloud AI Agent Studio (`Ctrl+J` & `rustpad ai`)

- **Local-First Air-Gapped HTTP**: Connects over pure-Rust `std::net::TcpStream` HTTP/1.1 to **`nour-code`** (`127.0.0.1:8000`), **`Ollama`** (`127.0.0.1:11434` with live `/api/tags` model auto-discovery), **`LM Studio`** (`:1234`), or **`llama.cpp`**.
- **BYO-Key Cloud HTTPS**: Supports **OpenAI**, **Groq**, **OpenRouter**, and **Anthropic Claude** (`/v1/messages`) with session-only API key memory (keys are never persisted to disk).
- **Non-Blocking Worker & Safe Preview**: Executes on a background `std::thread` polled via `std::sync::mpsc` on `50 ms` ticks so typing stays responsive, isolates untrusted text inside `<document_context>`, strips `<think>` blocks, and previews output before **Insert Below** or **Replace**.

### 3. Vector PDF Studio (`.pdf`) & Resizable Dual-Pane Split View

![RustPad v1.3.0 PDF Studio with Bezier ink signature, stamp, sticky note, and resizable sidebars](docs/screenshots/pdf-studio.png)

- **Inline Edit, Sign & Redact**: Edit PDF text lines in place, draw freehand vector signatures (`Sign Ink`), stamp `[APPROVED]`, add sticky notes/highlights, or run true content-stream security redaction.
- **Drag-Resizable Split Workspace + `Ctrl+K` Palette**: Press `Ctrl+\` to view Word + PDF side-by-side with a draggable `24%–76%` center divider, or press `Ctrl+K` to search **69 studio commands**:

![RustPad v1.3.0 Split Workspace and Ctrl+K Command Palette](docs/screenshots/split-command-palette.png)

---

## CLI Reference

```text
USAGE:
  rustpad [FILE]                        Launch the native desktop studio (optionally opening FILE)
  rustpad ai <FILE> [OPTIONS]           Run a local (Ollama, nour-code, LM Studio) or cloud AI task
      --task <grammar|summarize|description|sections|ask>
      --provider <ollama|nour-code|lm-studio|openai|claude>
      --url <ENDPOINT_URL>              Override HTTP(S) chat/messages endpoint URL
      --model <MODEL_NAME>              Override model identifier (e.g. qwen2.5:7b, nour-code)
      --prompt <INSTRUCTION>            Optional focus instruction or custom question
      -o, --out <OUTPUT.docx|.md>       Write AI-augmented document or Markdown output to file
  rustpad convert <INPUT> <OUTPUT>      Convert between .docx, .pdf, and .md offline
  rustpad inspect <FILE>                Print page, word, character, or PDF object metrics
  rustpad ocr <FILE>                    Extract searchable text from a PDF or scanned image
  rustpad merge <OUT.pdf> <IN1.pdf>...  Merge multiple PDFs into a single vector PDF
  rustpad demo-files <OUT_DIR>          Generate showcase .docx, .pdf, and .md sample documents
  rustpad --render-preview <OUT_DIR>    Render 1440x900 PNG studio screenshots & 1200x630 OG card
```

---

## Keyboard Shortcuts

| Shortcut | Action | Scope |
|---|---|---|
| `Ctrl+K` | Open fuzzy Command Palette (69 studio actions) | Global |
| `Ctrl+J` | Toggle Pluggable AI Agent Studio Dock | Global |
| `Ctrl+\` | Toggle Drag-Resizable Side-by-Side Split Workspace | Global |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / Cut / Paste via OS & Studio Clipboard | Global |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Increase / Decrease / Reset Interface Scale (HiDPI) | Global |
| `Tab` / `Shift+Tab` | Next / Previous Table Cell (auto-adds row at end) | Word Tables |
| `Ctrl+N` / `Ctrl+O` / `Ctrl+S` | New Word Tab / Open File / Save Active File | Global |
| `Ctrl+E` | Export Word Document to Vector PDF (`.pdf`) | Word Studio |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo | Word Studio |

---

## Pure-Rust 9-Crate Workspace Architecture

| Crate | Path | Responsibility |
|---|---|---|
| `rustpad` (`app`) | `app/src/` | Multi-tab Word/PDF state, contextual Table ribbon, resizable AI dock, 69 commands & CLI |
| `power_tools` | `crates/power_tools/` | Pluggable local/cloud AI Agent Bridge (`ai_agent.rs`), `DOCX <-> PDF <-> MD` converters, spellcheck & snapshots |
| `doc_model` | `crates/doc_model/` | Structured Document AST, multi-line `TableBlock` row/col helpers, Undo/Redo & 6 templates |
| `docx_io` | `crates/docx_io/` | ECMA-376 OpenXML `.docx` reader & writer (paragraphs, styles, lists, tables, headers/footers) |
| `pdf_engine` | `crates/pdf_engine/` | PDF 1.4–1.7 parser, inline text edit, Bezier ink signatures, stamps, redaction, merge & split |
| `pdf_create` | `crates/pdf_create/` | ISO 32000 PDF 1.7 vector generator, 12 Type1 font slots, tables & watermarks |
| `rustpad_render` | `crates/rustpad_render/` | Pure-Rust TrueType (`.ttf`) rasterizer, `DaylightTheme`, SDF shapes, vector `UiIcon`s & PNG codec |
| `rustpad_window` | `crates/rustpad_window/` | Win32 GDI + `libX11` native loop, HiDPI scaling, dynamic OS cursors, mouse capture & clipboard |
| `rustpad_zip` | `crates/rustpad_zip/` | IEEE CRC-32, Adler-32, RFC 1951 DEFLATE/INFLATE & PKZIP archive engine |

---

## Author & License

Created and maintained by **Nour Yahyaoui**
- LinkedIn: [https://www.linkedin.com/in/nour-yahyaoui-86987b36b](https://www.linkedin.com/in/nour-yahyaoui-86987b36b)
- GitHub: [https://github.com/Nour-yahyaoui](https://github.com/Nour-yahyaoui)
- Portfolio: [https://nour-yahyaoui.vercel.app](https://nour-yahyaoui.vercel.app)
- Email: [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com)
- License: [MIT](LICENSE)
