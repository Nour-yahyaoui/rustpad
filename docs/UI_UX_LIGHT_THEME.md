# UI/UX & Smooth Light Theme Blueprint ("Paper & Daylight")

> **Design Philosophy:** *"As calm and readable as real paper on a sunlit desk, instant as a native Rust executable, and powerful without toolbar clutter."*  
> **Rendering Engine:** Native GPU-Accelerated `Iced` + `wgpu` + `cosmic-text` (120 FPS, Crisp HiDPI)  
> **Author:** Nour Yahyaoui ([GitHub](https://github.com/Nour-yahyaoui) · [nourryahyaoui@gmail.com](mailto:nourryahyaoui@gmail.com))

---

## 1. Core UX Principles (Easy to Use from Second One)

Legacy desktop office suites overwhelm users with **300+ tiny icons crammed into multi-row Ribbons**, heavy modal dialogs, and slow startup splashes. Our native Rust desktop UX follows five clear rules:

1. **Calm, Zero-Glare "Paper & Daylight" Light Theme:**  
   Instead of blinding `#FFFFFF` across every panel (which causes eye fatigue and blurs toolbars into pages), the **workspace desk** uses a soft cool alabaster (`#F4F6F9`), letting the **crisp white document sheet (`#FFFFFF`)** float naturally with a subtle GPU-rendered elevation shadow.
2. **Progressive Contextual Toolbar (No Ribbon Overload):**  
   - The main top bar shows **only ~10–12 everyday actions** with clear icons and breathing room.
   - Contextual pills (*Table Controls*, *Image Resize*, *Signature Ink Bar*) appear smoothly **only when you click a table, image, or signature**, and vanish when you return to typing.
   - Every tool in the entire program is reachable in **2 keystrokes via `Ctrl+K`** (Command Palette).
3. **The 3-Click Maximum Rule:**  
   Merging PDFs, splitting pages, signing a contract, converting a Word file to PDF, or inserting a styled table must take **at most 3 clicks** from anywhere in the app.
4. **Unified Multi-Tab & Side-by-Side Split View:**  
   Open `.docx` Word files and `.pdf` files as tabs inside the **same native window**, or split the window 50/50 to read/highlight a PDF on the left while writing a Word document on the right.
5. **Instant `< 120 ms` Launch & 120 FPS Feel:**  
   Zero splash screen delay. Every hover, dropdown, scroll, and page zoom renders directly on the GPU via `wgpu`.

---

## 2. Native `iced::Theme` Color Palette Tokens (`palette.rs`)

| Rust Token Constant | Hex Value | Visual Role in `Iced` |
| :--- | :--- | :--- |
| `DESK_ALABASTER` | `#F4F6F9` | Calm workspace backdrop behind floating A4/Letter sheets |
| `PAPER_WHITE` | `#FFFFFF` | Physical document pages, popover cards & modals |
| `CHROME_SURFACE` | `#FAFBFC` | Clean header bar, tab strip, status bar & side rails |
| `SURFACE_HOVER` | `#EEF2F6` | Subtle tactile hover highlight on buttons & list items |
| `BORDER_HAIRLINE` | `#E2E8F0` | Crisp `1px` dividers separating chrome from the desk canvas |
| `BORDER_PAPER` | `#DCE3EC` | Subtle physical page edge border |
| `INK_PRIMARY` | `#0F172A` | Deep Slate primary UI & body ink (high contrast without harsh `#000` glare) |
| `INK_SECONDARY` | `#475569` | Toolbar icons, secondary labels & inspector controls |
| `INK_MUTED` | `#94A3B8` | Rulers, placeholders & subtle shortcut hints (`Ctrl+K`) |
| `ACCENT_WORD` | `#2563EB` | **Word Mode Accent** — Calm Royal Cobalt (`.docx` badge, active Word tools, focus ring) |
| `ACCENT_WORD_SOFT` | `#EFF6FF` | Soft Cobalt fill for active pills & text selection |
| `ACCENT_PDF` | `#E11D48` | **PDF Mode Accent** — Warm Coral Crimson (`.pdf` badge & active PDF Studio tools) |
| `ACCENT_PDF_SOFT` | `#FFF1F2` | Soft Coral fill for selected PDF tools |
| `STATUS_SAVED` | `#059669` | Emerald dot confirming local snapshot/save completion |

---

## 3. Typography & Crisp Native Rendering

- **Bundled UI Font:** **`Inter`** embedded directly in `assets/fonts/` so the app looks identical and razor-sharp on Windows, macOS, and Linux out of the box.
- **Document Fonts:** Instant discovery of **all installed OS fonts** via `fontdb` + bundled clean open-source defaults (`Inter`, `Source Serif 4`, metric-compatible `Carlito`/`Caladea` for Calibri/Cambria `.docx` fidelity).
- **Metrics & Rulers Font:** **`JetBrains Mono`** (`12px`) for zoom percentages (`100%`), page dimensions, and `Page 3 of 12` counters.
- **Icons:** Clean `18px` Lucide vector SVGs rendered via `iced_svg` — airy, crisp on 4K/Retina displays, and easy to recognize at a glance.

---

## 4. Native Window Wireframes

### A. Unified Editor Window (Word or PDF Tab Active)

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│ [≡]  [W] Project_Plan.docx ×  │  [P] Signed_Contract.pdf ×  [+]   [🔍 Ctrl+K] — □ ×│
├──────────────────────────────────────────────────────────────────────────────────┤
│  ↶ ↷ │ Heading 1 ▾ │ Inter ▾  11 ▾ │ B  I  U  S  A▾ │ ≡  ≣  •≡ │ ⊞ Table  🖼  │ [Export PDF ▾] │
├──────────────┬────────────────────────────────────────────────────┬──────────────┤
│              │                 #F4F6F9 Desk Canvas                │              │
│  OUTLINE /   │        ┌──────────────────────────────────┐        │  INSPECTOR   │
│  PAGES       │        │          #FFFFFF Paper           │        │  (Hidden by  │
│              │        │                                  │        │   default —  │
│  1. Summary  │        │   Native Pure-Rust Document      │        │   slides in  │
│  2. Goals    │        │   Studio                         │        │   only when  │
│  3. Budget   │        │                                  │        │   needed!)   │
│              │        │   Crisp wgpu + cosmic-text       │        │              │
│  ──────────  │        │   paginated sheets with real     │        │  • Size: A4  │
│  [Snapshots] │        │   margins, tables, headers,      │        │  • Margins   │
│   • Just now │        │   and 1-click PDF creation...    │        │  • Spacing   │
│   • 15m ago  │        │                                  │        │              │
│              │        └──────────────────────────────────┘        │              │
├──────────────┴────────────────────────────────────────────────────┴──────────────┤
│ ● Saved locally (Snapshot #8)     │ 1,280 words · 5 min read │  Page 1 of 3   [-] 100% [+] │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### B. Calm 1-Click Home Launch Hub

When launched from the Desktop/Start Menu with no file open:
- **4 Hero Action Cards (1 Click):**
  1. **New Word Document** (`.docx` blank paginated sheet)
  2. **Open & Edit PDF** (edit text, highlight, fill forms, sign)
  3. **Merge & Split PDF** (drag-and-drop page studio)
  4. **Convert / OCR** (`Word ↔ PDF`, `Scanned PDF → Editable Word`)
- **Starter Template Strip:** Clean Resume, Formal Letter, Academic Report, Business Invoice, Blank PDF.
- **Recent Local Documents:** Fast searchable list with `DOCX`/`PDF` badges, file path, last modified time, and pinned favorites.
