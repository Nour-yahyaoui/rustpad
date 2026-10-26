export interface StudioSpecimen {
  id: string;
  index: string;
  label: string;
  shortLabel: string;
  badge: string;
  windowTitle: string;
  image: string;
  alt: string;
  summary: string;
  highlights: { tag: string; detail: string }[];
}

export interface CraftPillar {
  index: string;
  tag: string;
  title: string;
  summary: string;
  bullets: string[];
}

export interface AiProviderSpec {
  id: string;
  name: string;
  mode: 'LOCAL HTTP' | 'CLOUD HTTPS';
  endpoint: string;
  defaultModel: string;
  transport: string;
  cliExample: string;
  notes: string;
}

export interface CrateSpec {
  crateName: string;
  layer: string;
  standard: string;
  responsibility: string;
  keyModules: string;
}

export interface CliRecipe {
  label: string;
  badge: string;
  command: string;
  detail: string;
}

export interface TourStep {
  step: string;
  targetSelector: string;
  specimenId?: string;
  title: string;
  body: string;
}

export const RELEASE_VERSION = 'v1.3.0';
export const REPO_URL = 'https://github.com/Nour-yahyaoui/rustpad';
export const RELEASE_TAG_URL = `${REPO_URL}/releases/tag/${RELEASE_VERSION}`;

export const DOWNLOADS = {
  windowsExe: `${REPO_URL}/releases/download/${RELEASE_VERSION}/rustpad-${RELEASE_VERSION}-windows-x86_64.exe`,
  linuxBin: `${REPO_URL}/releases/download/${RELEASE_VERSION}/rustpad-${RELEASE_VERSION}-linux-x86_64`,
  checksums: `${REPO_URL}/releases/download/${RELEASE_VERSION}/SHA256SUMS.txt`,
  allReleases: `${REPO_URL}/releases`,
};

export const AUTHOR = {
  name: 'Nour Yahyaoui',
  role: 'Full-Stack & Systems Software Engineer',
  location: 'Tunis, Tunisia',
  github: 'https://github.com/Nour-yahyaoui',
  linkedin: 'https://www.linkedin.com/in/nour-yahyaoui-86987b36b',
  portfolio: 'https://nour-yahyaoui.vercel.app',
  email: 'nourryahyaoui@gmail.com',
  committersBadge: 'https://user-badge.committers.top/tunisia/Nour-yahyaoui.svg',
  committersUrl: 'https://committers.top/tunisia#Nour-yahyaoui',
};

export const HERO_METRICS = [
  {
    value: '< 90 ms',
    label: 'Cold Native Boot',
    detail: 'Direct Win32 GDI & X11 windowing',
  },
  {
    value: '~3.8 MB',
    label: 'Stripped Binary',
    detail: 'Zero Electron / WebView runtime',
  },
  {
    value: '9 Crates',
    label: '0 crates.io Deps',
    detail: 'Custom ZIP, TTF, DOCX & PDF stack',
  },
  {
    value: 'Local + API',
    label: 'Pluggable AI Dock',
    detail: 'nour-code, Ollama, Claude & OpenAI',
  },
];

export const STUDIO_SPECIMENS: StudioSpecimen[] = [
  {
    id: 'word-ai',
    index: '01',
    label: 'Word + AI Studio',
    shortLabel: 'Word + AI',
    badge: 'DOCX + AI DOCK',
    windowTitle: 'RustPad v1.3.0 — Architecture_Spec_v1.3.docx · AI Studio Active',
    image: '/screenshots/word-studio.png',
    alt: 'RustPad v1.3.0 Word Studio showing Warm Paper & Linen chrome, contextual Table Studio bar, wrapped multi-line OpenXML table with +Add Row bar, and right-side Pluggable AI Agent Studio dock connected to nour-code.',
    summary:
      'Paginated A4/Letter Word (.docx) canvas paired with the contextual Table Studio ribbon and drag-resizable Pluggable AI Agent Dock running a local nour-code model.',
    highlights: [
      {
        tag: 'Wrapped .docx Tables',
        detail: 'Multi-line cell wrapping, in-cell blinking caret, +Add Row bar & +Col rail',
      },
      {
        tag: 'Pluggable AI Dock',
        detail: 'Switch between nour-code, Ollama, LM Studio, OpenAI & Claude (Ctrl+J)',
      },
      {
        tag: 'Paper & Linen UI',
        detail: '12.5 pt comfortable typography, vector icons & HiDPI UI scale steppers',
      },
    ],
  },
  {
    id: 'pdf-studio',
    index: '02',
    label: 'Vector PDF Sign',
    shortLabel: 'PDF Studio',
    badge: 'ISO 32000 PDF',
    windowTitle: 'RustPad v1.3.0 — Technical_Report.pdf · ISO 32000 Vector Stream',
    image: '/screenshots/pdf-studio.png',
    alt: 'RustPad v1.3.0 PDF Studio with drag-resizable page thumbnail rail, vector Bezier ink signature, APPROVED stamp, sticky note, and right PDF inspector.',
    summary:
      'Open, inspect, annotate, redact, merge, split, and sign ISO 32000-1 PDF documents with real Bezier ink strokes and bidirectional PDF-to-Word conversion.',
    highlights: [
      {
        tag: 'Bezier Ink Sign',
        detail: 'Draw vector signatures & place [APPROVED] stamps directly into PDF streams',
      },
      {
        tag: 'Resizable Rails',
        detail: 'Drag-resize or collapse Left Pages & Right PDF Inspector sidebars',
      },
      {
        tag: 'PDF <-> DOCX',
        detail: '1-click structural conversion from PDF pages into editable Word blocks',
      },
    ],
  },
  {
    id: 'split-palette',
    index: '03',
    label: 'Split + Ctrl+K',
    shortLabel: 'Split View',
    badge: '62 COMMANDS',
    windowTitle: 'RustPad v1.3.0 — Dual Split Workspace + Instant Command Palette',
    image: '/screenshots/split-command-palette.png',
    alt: 'RustPad v1.3.0 Split View showing Word Studio on the left, unclipped PDF Studio on the right with draggable terracotta divider, and Ctrl+K command palette.',
    summary:
      'Reference a PDF spec on the right while drafting a .docx report on the left. Drag the center splitter anywhere from 24% to 76% or fire any of 62 commands via Ctrl+K.',
    highlights: [
      {
        tag: 'Draggable Split',
        detail: 'Smooth SetCapture divider resize with auto-fitted, unclipped PDF sheet',
      },
      {
        tag: 'Ctrl+K Palette',
        detail: 'Fuzzy-search 62 File, Format, Table, PDF, View & AI Agent actions',
      },
      {
        tag: 'Keyboard-First',
        detail: 'Ctrl+\\ split, Ctrl+J AI dock, Ctrl+=/Ctrl+- UI scale, Ctrl+C/X/V clipboard',
      },
    ],
  },
  {
    id: 'home-launchpad',
    index: '04',
    label: 'Studio Launchpad',
    shortLabel: 'Launchpad',
    badge: 'EDITORIAL HUB',
    windowTitle: 'RustPad v1.3.0 — Asymmetric Editorial Home Launchpad',
    image: '/screenshots/home-hub.png',
    alt: 'RustPad v1.3.0 Asymmetric Editorial Home Launchpad showing quick creation buttons, hairline Recent Workspace Ledger, and six A4 paper miniature templates.',
    summary:
      'Asymmetric editorial launchpad replacing generic SaaS cards with hairline workspace ledgers, instant .docx/.pdf/.md creation, and miniature A4 starter sheets.',
    highlights: [
      {
        tag: '6 Starter Sheets',
        detail: 'Blank, Welcome Guide, Modern Resume, Technical Report, Invoice & Letter',
      },
      {
        tag: 'Recent Ledger',
        detail: 'Zero-cloud local workspace history with instant DOCX / PDF format badges',
      },
      {
        tag: 'Time-Machine',
        detail: 'DEFLATE-compressed in-session document snapshots with 1-click restore',
      },
    ],
  },
];

export const CRAFT_PILLARS: CraftPillar[] = [
  {
    index: '01',
    tag: 'TABLES · ISSUE #12',
    title: 'Interactive Multi-Line OpenXML Tables',
    summary:
      'Tables are first-class layout citizens rather than rigid single-line boxes. Cells word-wrap cleanly, expand row height automatically, and expose direct canvas controls.',
    bullets: [
      'On-canvas "+ Add Row" bottom bar and vertical "+" right-edge column rail',
      'Contextual TABLE sub-bar (+Row Above/Below, +Col Left/Right, Header Bold, Shade, Delete)',
      'Draggable column borders with ResizeEW cursor + Tab / Shift+Tab cell navigation',
    ],
  },
  {
    index: '02',
    tag: 'SMOOTHNESS · ISSUE #10',
    title: 'Character-Exact Hit-Testing & 1:1 Scroll',
    summary:
      'Every mouse click resolves against true TrueType glyph advances across wrapped lines and table cells, paired with flicker-free GDI/X11 frame pacing.',
    bullets: [
      'Click any character inside wrapped paragraphs or table cells to place the caret',
      'Fixed 56 px/notch non-inverted wheel scrolling + interactive draggable scrollbar',
      'Automatic scroll-to-caret viewport tracking while typing, Enter-splitting, or arrowing',
    ],
  },
  {
    index: '03',
    tag: 'ERGONOMICS · ISSUE #11',
    title: 'HiDPI Auto-Scale & Native OS Clipboard',
    summary:
      'Designed for long engineering and writing sessions on 1080p, 1440p, and 4K displays without squinting or losing clipboard interoperability.',
    bullets: [
      'Auto-detects Win32 GetDpiForWindow & Linux Xft.dpi + live UI- / 100% / UI+ steppers',
      '12.5 pt body typography at 112% default paper zoom on Warm Paper (#FFFFFF / #EAE6DC)',
      'Native Win32 CF_UNICODETEXT & Linux xclip/xsel/wl-copy Ctrl+C, Ctrl+X, Ctrl+V',
    ],
  },
  {
    index: '04',
    tag: 'STUDIO CHROME · ISSUE #13',
    title: 'Drag-Resizable Rails & Architectural UI',
    summary:
      'Replaced noisy text-box ribbons with custom anti-aliased vector icons, floating typography menus, and smooth drag-resizable sidebars.',
    bullets: [
      'Drag-resize or collapse Word Outline, PDF Pages, PDF Inspector, Split View & AI Dock',
      'Dynamic OS cursors: IBeam over text, Hand on controls, ResizeEW on splitters',
      'Floating Paragraph Style & Font Family popovers with live typographic previews',
    ],
  },
];

export const AI_PROVIDERS: AiProviderSpec[] = [
  {
    id: 'nour-code',
    name: 'nour-code (Local)',
    mode: 'LOCAL HTTP',
    endpoint: 'http://127.0.0.1:8000/v1/chat/completions',
    defaultModel: 'nour-code',
    transport: 'Pure-Rust std::net::TcpStream HTTP/1.1 (0 external deps)',
    cliExample:
      'rustpad ai spec.docx --provider nour-code --task sections --prompt "Add latency benchmarks" -o spec_v2.docx',
    notes:
      'Connects directly to a local nour-code / vLLM / FastAPI OpenAI-compatible server on localhost with zero API keys and 100% air-gapped document privacy.',
  },
  {
    id: 'ollama',
    name: 'Ollama (Local)',
    mode: 'LOCAL HTTP',
    endpoint: 'http://127.0.0.1:11434/v1/chat/completions',
    defaultModel: 'qwen2.5:7b',
    transport: 'Pure-Rust TcpStream + /api/tags live model auto-discovery',
    cliExample:
      'rustpad ai report.pdf --provider ollama --model qwen2.5:7b --task summarize -o summary.md',
    notes:
      'Click "Detect Local" inside the AI Studio Dock to query 127.0.0.1:11434/api/tags and populate installed local models as 1-click pills.',
  },
  {
    id: 'lm-studio',
    name: 'LM Studio / llama.cpp',
    mode: 'LOCAL HTTP',
    endpoint: 'http://127.0.0.1:1234/v1/chat/completions',
    defaultModel: 'local-model',
    transport: 'Pure-Rust TcpStream HTTP/1.1 + chunked transfer decoder',
    cliExample:
      'rustpad ai draft.docx --provider lm-studio --task grammar -o draft_polished.docx',
    notes:
      'Works out of the box with LM Studio local server (:1234) and llama-server --port 8080; strips <think>...</think> reasoning blocks automatically.',
  },
  {
    id: 'openai',
    name: 'OpenAI / Groq / OpenRouter',
    mode: 'CLOUD HTTPS',
    endpoint: 'https://api.openai.com/v1/chat/completions',
    defaultModel: 'gpt-4o-mini',
    transport: 'Background worker thread via OS TLS bridge (RUSTPAD_AI_KEY / OPENAI_API_KEY)',
    cliExample:
      'OPENAI_API_KEY=sk-... rustpad ai proposal.docx --provider openai --task description',
    notes:
      'Supports any HTTPS OpenAI-compatible endpoint (OpenAI, Groq, OpenRouter, Together, Mistral). API keys remain in session memory and are never written to disk.',
  },
  {
    id: 'claude',
    name: 'Anthropic Claude',
    mode: 'CLOUD HTTPS',
    endpoint: 'https://api.anthropic.com/v1/messages',
    defaultModel: 'claude-3-5-sonnet-latest',
    transport: 'Native Anthropic Messages v1 payload (x-api-key + anthropic-version)',
    cliExample:
      'ANTHROPIC_API_KEY=sk-ant-... rustpad ai contract.pdf --provider claude --task ask --prompt "List obligations"',
    notes:
      'Formats native Anthropic Messages JSON with isolated <document_context> system prompts and parses Markdown responses into Word AST blocks.',
  },
];

export const CRATE_LEDGER: CrateSpec[] = [
  {
    crateName: 'rustpad (app)',
    layer: 'Desktop & CLI Shell',
    standard: 'Win32 + X11 + CLI',
    responsibility:
      'Multi-tab Word/PDF studio, contextual table ribbon, resizable AI dock, 62-command palette & headless CLI.',
    keyModules: 'app/src/{main,state,word_view,pdf_view,ui_paint,commands}.rs',
  },
  {
    crateName: 'power_tools',
    layer: 'AI & Workflows',
    standard: 'HTTP/1.1 + Converters',
    responsibility:
      'Pluggable local/cloud AI Agent Bridge, DOCX <-> PDF <-> Markdown converters, offline spellcheck & DEFLATE snapshots.',
    keyModules: 'crates/power_tools/src/{ai_agent,converter,spellcheck,snapshots,ocr}.rs',
  },
  {
    crateName: 'doc_model',
    layer: 'Document AST',
    standard: 'Rich Block Tree',
    responsibility:
      'Paragraphs, styled spans, wrapped multi-line TableBlock helpers, lists, checklists, undo/redo & 6 templates.',
    keyModules: 'crates/doc_model/src/{lib,templates}.rs',
  },
  {
    crateName: 'docx_io',
    layer: 'Word Codec',
    standard: 'ECMA-376 OpenXML',
    responsibility:
      'Zero-dependency XML pull/push reader & writer for word/document.xml, styles, numbering, relationships & tables.',
    keyModules: 'crates/docx_io/src/{reader,writer,xml_lite}.rs',
  },
  {
    crateName: 'pdf_create',
    layer: 'Vector PDF Writer',
    standard: 'ISO 32000-1.7',
    responsibility:
      'Paginated vector PDF generator with headers, footers, geometric rules, tables, watermarks & xref tables.',
    keyModules: 'crates/pdf_create/src/lib.rs',
  },
  {
    crateName: 'pdf_engine',
    layer: 'PDF Parser & Editor',
    standard: 'ISO 32000 Streams',
    responsibility:
      'Content-stream parser, inline text editing, Bezier ink signatures, stamps, redactions, page merge/split & compression.',
    keyModules: 'crates/pdf_engine/src/{parser,writer,lib}.rs',
  },
  {
    crateName: 'rustpad_render',
    layer: '2D Framebuffer',
    standard: 'TrueType + SDF',
    responsibility:
      'Pure-Rust .ttf glyph rasterizer, SDF rounded rects, anti-aliased vector UI icons, warm shadows & PNG codec.',
    keyModules: 'crates/rustpad_render/src/{ttf,canvas,theme,png_codec}.rs',
  },
  {
    crateName: 'rustpad_window',
    layer: 'OS Windowing FFI',
    standard: 'Win32 GDI + libX11',
    responsibility:
      'Zero-crate native window loop, HiDPI detection, dynamic OS cursors, mouse capture & OS clipboard integration.',
    keyModules: 'crates/rustpad_window/src/{win32,linux_x11,lib}.rs',
  },
  {
    crateName: 'rustpad_zip',
    layer: 'Compression Core',
    standard: 'RFC 1951 + PKZIP',
    responsibility:
      'From-scratch DEFLATE compressor, INFLATE decompressor, CRC-32/Adler-32 & ZIP archive engine powering .docx & PDF.',
    keyModules: 'crates/rustpad_zip/src/{deflate,inflate,zip_archive,checksum}.rs',
  },
];

export const CLI_RECIPES: CliRecipe[] = [
  {
    label: 'Run Local AI Agent on a .docx or .pdf',
    badge: 'AI CLI',
    command:
      'rustpad ai report.docx --provider nour-code --task summarize -o executive_summary.md',
    detail:
      'Streams document context to Ollama, nour-code, LM Studio, OpenAI, or Claude and writes clean Markdown or appends structured .docx blocks.',
  },
  {
    label: 'Convert Word (.docx) <-> Vector PDF <-> Markdown',
    badge: 'CONVERT',
    command: 'rustpad convert Architecture_Spec.docx Architecture_Spec.pdf',
    detail:
      'Deterministic offline conversion across .docx, .pdf, and .md in milliseconds — built for CI pipelines and batch scripts.',
  },
  {
    label: 'Merge & Inspect Multi-File PDF Packets',
    badge: 'PDF OPS',
    command: 'rustpad merge Release_Packet.pdf Cover.pdf Spec.pdf Signed_Appendix.pdf',
    detail:
      'Combines multiple ISO 32000 PDFs into a compressed vector packet without ghostscript, Java, or cloud upload.',
  },
  {
    label: 'Build Native Binary from Zero-Dependency Rust Source',
    badge: 'SOURCE',
    command: 'git clone https://github.com/Nour-yahyaoui/rustpad.git && cd rustpad && cargo run --release',
    detail:
      'Compiles all 9 workspace crates cleanly with stable rustc — zero crates.io downloads required.',
  },
];

export const SCOPE_MATRIX = {
  builtFor: [
    'Instant (<90 ms) daily drafting of clean .docx specs, resumes, letters, and technical reports',
    'Interactive multi-line tables, headings, lists, checklists, undo/redo, and local version snapshots',
    'Vector PDF export, page merge/split/reorder, Bezier ink signatures, stamps, and redactions',
    'Air-gapped local AI workflows (Ollama, nour-code, LM Studio) + optional BYO-key OpenAI/Claude',
  ],
  nonGoals: [
    'Complex multi-author Word Track Changes, VBA macros, or OLE embedded spreadsheets',
    'Arbitrary encrypted XFA PDF forms or CMYK prepress color-separation pipelines',
    'Forced cloud accounts, background telemetry daemons, or 250 MB Chromium installers',
  ],
};

export const TOUR_STEPS: TourStep[] = [
  {
    step: '01 / 05',
    targetSelector: '#hero',
    title: 'Zero-WebView Native Rust Studio',
    body: 'RustPad v1.3.0 packs a Word (.docx) processor, ISO 32000 PDF editor, and pluggable AI studio into a ~3.8 MB native binary.',
  },
  {
    step: '02 / 05',
    targetSelector: '#studio',
    specimenId: 'word-ai',
    title: 'Live Studio Switchboard',
    body: 'Tap through all 4 native studio views — Word + AI Studio, Vector PDF Sign, Split View + Ctrl+K, and the Editorial Launchpad.',
  },
  {
    step: '03 / 05',
    targetSelector: '#craft',
    title: 'Interactive Tables & Smooth Ergonomics',
    body: 'v1.3.0 adds wrapped multi-line cells, +Add Row / +Col rails, draggable column dividers, character-exact clicks, and HiDPI scaling.',
  },
  {
    step: '04 / 05',
    targetSelector: '#ai-studio',
    title: 'Local & Cloud AI Agent Bridge',
    body: 'Connect Ollama, nour-code, or LM Studio over pure-Rust HTTP, or plug in OpenAI / Claude keys for grammar, summaries, and new sections.',
  },
  {
    step: '05 / 05',
    targetSelector: '#architecture',
    title: '9 Pure-Rust Crates, Zero Bloat',
    body: 'Inspect the 9-crate workspace from custom RFC 1951 DEFLATE/ZIP and TrueType rasterization up to ECMA-376 OpenXML and ISO 32000 PDF.',
  },
];
