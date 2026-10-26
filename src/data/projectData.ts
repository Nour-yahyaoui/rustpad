export interface StudioMode {
  id: string;
  kicker: string;
  name: string;
  image: string;
  alt: string;
  caption: string;
  shortcut: string;
  sourceRef: string;
}

export interface CrateEntry {
  name: string;
  category: 'Core I/O' | 'Document & PDF' | 'Power Tools' | 'Native UI';
  loc: number;
  path: string;
  summary: string;
  tests: string;
}

export interface CliPreset {
  id: string;
  label: string;
  command: string;
  outputPreview: string;
  note: string;
}

export interface TourStep {
  step: number;
  targetSelector: string;
  title: string;
  body: string;
}

export const STUDIO_MODES: StudioMode[] = [
  {
    id: 'word',
    kicker: 'Mode 01 · .docx',
    name: 'Word Studio',
    image: '/screenshots/word-studio.png',
    alt: 'RustPad Word Studio showing an A4 sheet with headings, bullet list, 3-column table, interactive checklists, left document outline, and live word count metrics.',
    caption:
      'Paginated A4 / Letter / Legal ECMA-376 OpenXML editor with clickable heading outline, live reading-time metrics, tables, checklists, and inline Levenshtein spellcheck.',
    shortcut: 'Ctrl+N · Ctrl+S',
    sourceRef: 'app/src/word_view.rs',
  },
  {
    id: 'pdf',
    kicker: 'Mode 02 · .pdf',
    name: 'Vector PDF Studio',
    image: '/screenshots/pdf-studio.png',
    alt: 'RustPad PDF Studio displaying Technical Architecture Report with an emerald APPROVED vector stamp, amber text highlight, sticky note, benchmark table, and cursive ink signature.',
    caption:
      'ISO 32000 PDF 1.4–1.7 viewer and editor: click any line to edit text in place, place [APPROVED] vector stamps, draw polyline ink signatures, redact sensitive streams, merge, or compress.',
    shortcut: 'Ctrl+E · Ctrl+M',
    sourceRef: 'app/src/pdf_view.rs',
  },
  {
    id: 'split',
    kicker: 'Mode 03 · 50/50',
    name: 'Split + Ctrl+K',
    image: '/screenshots/split-command-palette.png',
    alt: 'RustPad 50/50 Split Workspace pairing Word Studio on the left with PDF Studio on the right while the Ctrl+K Command Palette filters PDF actions.',
    caption:
      'Side-by-side 50/50 reference workspace pairing an editable Word draft beside a live vector PDF sheet, topped by the instant fuzzy Command Palette across 49 actions.',
    shortcut: 'Ctrl+\\ · Ctrl+K',
    sourceRef: 'app/src/ui_paint.rs',
  },
  {
    id: 'home',
    kicker: 'Mode 04 · Hub',
    name: 'Template Hub',
    image: '/screenshots/home-hub.png',
    alt: 'RustPad Home Hub displaying six starter document templates and recent .docx and .pdf workspace files in the Paper and Daylight light theme.',
    caption:
      'Zero-friction launchpad with 6 built-in OpenXML templates (Blank, Welcome Guide, Modern Resume, Technical Report, Business Invoice, Executive Letter) and recent workspace files.',
    shortcut: 'Ctrl+O · Home',
    sourceRef: 'crates/doc_model/src/templates.rs',
  },
];

export const WORKSPACE_CRATES: CrateEntry[] = [
  {
    name: 'rustpad (app)',
    category: 'Native UI',
    loc: 5384,
    path: 'app/src/{main,state,word_view,pdf_view,ui_paint,commands}.rs',
    summary:
      'Unified desktop binary & CLI orchestrating multi-tab Word/PDF state, 49 Command Palette actions, 50/50 split view, modals, and headless screenshot rendering.',
    tests: 'Pass (render + command dispatch)',
  },
  {
    name: 'rustpad_render',
    category: 'Native UI',
    loc: 1196,
    path: 'crates/rustpad_render/src/{ttf,canvas,theme,png_codec}.rs',
    summary:
      'Pure-Rust TrueType (.ttf) SFNT table parser, 4x4 supersampled glyph rasterizer, DaylightTheme tokens, SDF rounded primitives, and lossless PNG encoder.',
    tests: 'Pass (TTF + SDF -> PNG)',
  },
  {
    name: 'rustpad_window',
    category: 'Native UI',
    loc: 1291,
    path: 'crates/rustpad_window/src/{linux_x11,win32,lib}.rs',
    summary:
      'Runtime dlopen("libX11.so.6") Linux backend with _NET_WM_ICON taskbar icon and Win32 GDI StretchDIBits + native OS file dialogs without C header build deps.',
    tests: 'Pass (event & key mapping)',
  },
  {
    name: 'doc_model',
    category: 'Document & PDF',
    loc: 1083,
    path: 'crates/doc_model/src/{lib,templates}.rs',
    summary:
      'Structured Document AST (paragraphs, spans, tables, embedded images, A4/Letter/Legal PageSetup), Undo/Redo ring buffer, Find/Replace, and 6 starter templates.',
    tests: 'Pass (2 suites: stats + undo/redo)',
  },
  {
    name: 'docx_io',
    category: 'Core I/O',
    loc: 1275,
    path: 'crates/docx_io/src/{reader,writer,xml_lite}.rs',
    summary:
      'ECMA-376 OpenXML (.docx) archive reader & serializer handling word/document.xml, styles, numbering, relationships, embedded media, and Dublin Core metadata.',
    tests: 'Pass (round-trip OpenXML)',
  },
  {
    name: 'pdf_create',
    category: 'Document & PDF',
    loc: 759,
    path: 'crates/pdf_create/src/lib.rs',
    summary:
      'ISO 32000 PDF 1.7 vector layout generator with 12 WinAnsi Type1 font slots, full-line run coalescing, tables, running headers/footers, watermarks, and FlateDecode.',
    tests: 'Pass (vector PDF 1.7 validation)',
  },
  {
    name: 'pdf_engine',
    category: 'Document & PDF',
    loc: 1586,
    path: 'crates/pdf_engine/src/{parser,writer,lib}.rs',
    summary:
      'PDF 1.4–1.7 object parser and editor: inline text editing, highlights, sticky notes, stamps, polyline ink signatures, true text-stream redaction, merge, split, compress.',
    tests: 'Pass (parse/edit/sign/redact/merge)',
  },
  {
    name: 'power_tools',
    category: 'Power Tools',
    loc: 868,
    path: 'crates/power_tools/src/{converter,spellcheck,ocr,snapshots}.rs',
    summary:
      'Bidirectional DOCX <-> PDF <-> Markdown converters, offline Levenshtein spellchecker, offline scan/PDF OCR extractor, and compressed Time-Machine snapshot vault.',
    tests: 'Pass (DOCX/PDF/MD + snapshots)',
  },
  {
    name: 'rustpad_zip',
    category: 'Core I/O',
    loc: 812,
    path: 'crates/rustpad_zip/src/{inflate,deflate,zip_archive,checksum}.rs',
    summary:
      'From-scratch IEEE CRC-32, Adler-32, RFC 1951 DEFLATE (Store, Fixed & Dynamic Huffman), RFC 1950 Zlib, and PKZIP archive reader/writer.',
    tests: 'Pass (4 suites + Python zlib interop)',
  },
];

export const CLI_PRESETS: CliPreset[] = [
  {
    id: 'build-run',
    label: 'Build & Launch GUI',
    command: 'cargo build --release -p rustpad && ./target/release/rustpad',
    outputPreview:
      'Finished `release` profile [optimized] · Binary: target/release/rustpad (3.3 MB, stripped)',
    note: 'Compiles all 9 pure-Rust crates offline with zero external crates.io network fetches and opens the native Daylight desktop window.',
  },
  {
    id: 'convert',
    label: 'Convert .docx -> .pdf',
    command:
      'rustpad convert examples/RustPad_Welcome_Guide.docx output/Welcome_Print.pdf',
    outputPreview: 'Converted "examples/RustPad_Welcome_Guide.docx" -> "output/Welcome_Print.pdf"',
    note: 'Converts interchangeably across .docx, .pdf, and .md completely offline from scripts or CI pipelines.',
  },
  {
    id: 'inspect',
    label: 'Inspect Document Metrics',
    command: 'rustpad inspect examples/RustPad_Welcome_Guide.docx',
    outputPreview:
      'Format: ECMA-376 Word (.docx) | Paper: A4 (210 x 297 mm) | Words: 187 | Tables: 1 | Outline H1+: 4',
    note: 'Prints structured OpenXML or ISO 32000 PDF page, word, character, table, and annotation counts.',
  },
  {
    id: 'merge',
    label: 'Merge Vector PDFs',
    command:
      'rustpad merge combined_packet.pdf examples/RustPad_Welcome_Guide.pdf examples/Business_Invoice_Sample.pdf',
    outputPreview: 'Merged 2 PDF files into "combined_packet.pdf" (2 pages, FlateDecode compressed)',
    note: 'Combines multiple PDF documents while preserving vector text items, annotations, and page geometry.',
  },
  {
    id: 'ocr',
    label: 'Offline Text / OCR Extract',
    command: 'rustpad ocr examples/RustPad_Welcome_Guide.pdf',
    outputPreview:
      'Welcome to RustPad — Fast · Lightweight · 100% Native Rust · Designed by Nour Yahyaoui ...',
    note: 'Extracts searchable plain text from PDF content streams or rasterized scan pages without cloud uploads.',
  },
];

export const TOUR_STEPS: TourStep[] = [
  {
    step: 1,
    targetSelector: '#studio-switchboard',
    title: 'Inspect all 4 Native Studio Modes',
    body: 'Switch between genuine 1440x900 framebuffer captures of Word Studio (.docx), Vector PDF Studio (.pdf), 50/50 Split Workspace, and the Template Hub.',
  },
  {
    step: 2,
    targetSelector: '#specimens',
    title: 'Verify Real .docx & .pdf Byte Headers',
    body: 'Select any sample document generated by RustPad to inspect its live PKZIP (50 4B 03 04) or %PDF-1.7 binary header in your browser and download the file.',
  },
  {
    step: 3,
    targetSelector: '#cli',
    title: 'Run the 3.3 MB Binary or CLI',
    body: 'Copy verified Cargo build commands, headless conversion pipelines, and SHA-256 release checksums from the CLI & Release Verification Bench.',
  },
];
