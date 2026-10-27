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

export interface EnterpriseBenchmarkRow {
  capability: string;
  rustpad: string;
  cloudSaas: string;
  electronApps: string;
  legacySuites: string;
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

export const RELEASE_VERSION = 'v2.2.0';
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
    value: '60–120 Hz',
    label: 'Ultra-Smooth Native UX',
    detail: 'VSync coalesced · Spring scroll · Floating bar',
  },
  {
    value: '100% Offline',
    label: 'Air-Gapped Sovereign',
    detail: 'Zero cloud lock-in · MDM DLP policy.json',
  },
  {
    value: '0 Deps',
    label: '0 crates.io Runtime',
    detail: 'Auditable CycloneDX 1.5 SBOM built-in',
  },
  {
    value: '~3.9 MB',
    label: '< 90 ms Cold Boot',
    detail: 'Win32 GDI, X11 & PNG/JPEG/BMP figures',
  },
];

export const ENTERPRISE_BENCHMARKS: EnterpriseBenchmarkRow[] = [
  {
    capability: 'UI Smoothness, Latency & Human UX',
    rustpad: '60–120Hz VSync · Floating Selection Bar · Focus Mode',
    cloudSaas: 'Browser DOM Reflow & Network Jitter',
    electronApps: 'Multi-Process IPC & GPU Compositor Lag',
    legacySuites: '300+ Ribbon Icons & Modal Stutter',
  },
  {
    capability: '.docx Figures & Table Creation UX',
    rustpad: 'PNG/JPEG/BMP Codec · 8×8 Grid · /image & /table · TSV/Paste',
    cloudSaas: 'Cloud Upload Latency & Broken Offline Media',
    electronApps: 'Markdown-Only Links (No Real OOXML Figures)',
    legacySuites: 'Nested Modal Dialogs & Anchor Drift',
  },
  {
    capability: 'Air-Gapped Offline Operation',
    rustpad: '100% Native Offline · 0 Telemetry',
    cloudSaas: 'Cloud Account Mandatory',
    electronApps: 'Partial / Cached Web App',
    legacySuites: 'Offline + Activation Telemetry',
  },
  {
    capability: '3rd-Party Package Supply Chain',
    rustpad: '0 crates.io Deps · Built-in CycloneDX SBOM',
    cloudSaas: 'Opaque SaaS Multi-Tenant',
    electronApps: '500–1,100 npm/Chromium pkgs',
    legacySuites: 'Dozens of C/C++ shared codecs',
  },
  {
    capability: 'Legal Track Changes & Redline',
    rustpad: '1-Click Clause Accept/Reject + ECMA-376 <w:ins>/<w:del>',
    cloudSaas: 'Proprietary Suggestion Mode',
    electronApps: 'Rare / Markdown-only',
    legacySuites: 'Full OOXML Track Changes',
  },
  {
    capability: 'Forensic PDF Redaction & Seals',
    rustpad: 'Purges Glyph Streams + PDF/A-2b + SHA-256',
    cloudSaas: 'Requires Paid Add-ons',
    electronApps: 'Visual Black-Box Overlay Risk',
    legacySuites: 'Paid Acrobat Pro License',
  },
  {
    capability: 'Enterprise AI Data Loss Prevention',
    rustpad: 'MDM policy.json Localhost Lock + Selection AI Polish',
    cloudSaas: 'Vendor Cloud Egress',
    electronApps: 'Unrestricted API Keys',
    legacySuites: 'Cloud Copilot Egress',
  },
];

export const STUDIO_SPECIMENS: StudioSpecimen[] = [
  {
    id: 'word-ai',
    index: '01',
    label: 'Smooth Word + Redline',
    shortLabel: 'Smooth Word',
    badge: '60–120HZ + REDLINE',
    windowTitle: 'RustPad v2.2.0 — Enterprise_MSA_Redline.docx · Smooth UX & Table Studio',
    image: '/screenshots/word-studio.png',
    alt: 'RustPad v2.2.0 Word Studio showing 60–120Hz smooth sheet rendering, effortless interactive tables, ECMA-376 Track Changes legal redlines, and DLP-governed local AI dock.',
    summary:
      'Highlight any phrase with mouse-drag, double-click, or Shift+Arrow for a Warm Ivory floating selection bar (B/I/U/Highlight/AI Polish), glide with 60Hz spring scrolling, and review legal redlines clause-by-clause.',
    highlights: [
      {
        tag: 'Floating Selection Bar',
        detail: 'Zero-lag mouse drag, double-click word & Ctrl+Arrow jumps + substring TextSpan styling',
      },
      {
        tag: '1-Click Clause Review',
        detail: 'Accept #N / Reject #N per revision clause alongside ECMA-376 <w:ins> & <w:del>',
      },
      {
        tag: 'Focus Mode + AI',
        detail: 'Ctrl+Shift+F hides chrome into calm Linen & Paper; AI Polish rewrites just your selection',
      },
    ],
  },
  {
    id: 'pdf-studio',
    index: '02',
    label: 'Forensic PDF & Seal',
    shortLabel: 'Forensic PDF',
    badge: 'PDF/A-2B + SHA-256',
    windowTitle: 'RustPad v2.2.0 — Technical_Report.pdf · SHA-256 Sealed',
    image: '/screenshots/pdf-studio.png',
    alt: 'RustPad v2.2.0 Forensic PDF Studio with Sanitize Redactions, SHA-256 Tamper Seal, Bezier ink signature, stamp, and PDF 1.5+ ObjStm parser.',
    summary:
      'Glide through multi-page PDFs with 60Hz kinetic spring scrolling, physically purge redacted underlying glyph runs, export archival PDF/A-2b XMP packets, and embed verifiable SHA-256 tamper seals.',
    highlights: [
      {
        tag: 'Forensic Sanitize',
        detail: 'Physically deletes underlying text runs & metadata inside redaction rectangles',
      },
      {
        tag: 'SHA-256 Seal',
        detail: 'Embeds /Type /RustPadSeal cryptographic content digests verified via rustpad verify',
      },
      {
        tag: 'PDF 1.5+ ObjStm',
        detail: 'Unpacks compressed object streams & PNG predictors + Bezier ink signatures',
      },
    ],
  },
  {
    id: 'split-palette',
    index: '03',
    label: 'DLP & Audit Chain',
    shortLabel: 'Governance',
    badge: 'DLP + AUDIT',
    windowTitle: 'RustPad v2.2.0 — Dual Split Workspace + SHA-256 Governance Ledger',
    image: '/screenshots/split-command-palette.png',
    alt: 'RustPad v2.2.0 Split Workspace with Enterprise DLP Policy and Tamper-Evident SHA-256 Hash-Chained Audit Trail Inspector.',
    summary:
      'Drag-resize side-by-side Word & PDF panes at 60–120Hz with WM_PAINT/X11 motion coalescing while inspecting MDM DLP policy state and the tamper-evident SHA-256 audit chain.',
    highlights: [
      {
        tag: 'Zero-Lag Splitters',
        detail: 'Coalesced Win32 WM_PAINT & X11 MotionNotify keep splitter & table resize buttery smooth',
      },
      {
        tag: 'MDM policy.json',
        detail: '1-click Strict Air-Gapped mode blocks external cloud AI before DNS resolution',
      },
      {
        tag: '74 Studio Actions',
        detail: 'Ctrl+A Select All, Ctrl+Shift+F Focus Mode, Ctrl+F Live Match Jump, Ctrl+K Palette',
      },
    ],
  },
  {
    id: 'home-launchpad',
    index: '04',
    label: 'Editorial Hub',
    shortLabel: 'Launchpad',
    badge: '7 TEMPLATES',
    windowTitle: 'RustPad v2.2.0 — Warm Architectural Paper Launchpad & Starter Sheets',
    image: '/screenshots/home-hub.png',
    alt: 'RustPad v2.2.0 Editorial Home Launchpad with 7 starter templates including Enterprise MSA Legal Redline.',
    summary:
      'Cold-boots in under 90 ms into a calm Warm Architectural Paper & Linen launchpad with 7 offline templates, pure-Rust PNG figure decoding, WAL crash recovery, and SMB lockfile protection.',
    highlights: [
      {
        tag: '7 Starter Sheets',
        detail: 'Includes Enterprise MSA Redline, Technical Report, Invoice, Resume & Executive Letter',
      },
      {
        tag: 'Live PNG Figures',
        detail: 'Pure-Rust RFC 2083 PNG decoder + bilinear HiDPI scaler renders embedded figures on-sheet',
      },
      {
        tag: 'Crash-Safe WAL',
        detail: 'SHA-256 verified Write-Ahead Recovery frames + .~rustpad.lock shared-drive guards',
      },
    ],
  },
];

export const CRAFT_PILLARS: CraftPillar[] = [
  {
    index: '01',
    tag: 'EFFORTLESS TABLE STUDIO · #36 #37 #38',
    title: 'Visual 8×8 Grid Picker, Styled Presets & Slash/TSV Tables',
    summary:
      'Create and style structured tables in one motion — with mouse hover, keyboard slash/pipe shorthands, or direct Excel / Google Sheets clipboard paste.',
    bullets: [
      'Interactive 8×8 visual hover/arrow-key grid (`Ctrl+Shift+T`) + 5 1-click styled presets',
      'Type `/table 4x5`, `/invoice`, or `| Col A | Col B |` + Enter, or `Ctrl+V` spreadsheet TSV',
      'Tab auto-adds rows, auto-replaces header placeholders, cycles 5 palettes & Zebra striping',
    ],
  },
  {
    index: '02',
    tag: 'SMOOTH .DOCX IMAGE STUDIO · #39 #40',
    title: 'Pure-Rust PNG/JPEG/BMP Decoders, Paste & Corner Resize',
    summary:
      'View and insert real-world .docx figures effortlessly — with zero crates.io image libraries, native screenshot paste, drag-and-drop, and aspect-locked handles.',
    bullets: [
      'From-scratch ITU-T.81 Baseline/Progressive JPEG, RFC 2083 PNG (palette/16-bit/Adam7) & BMP/DIB',
      'Parses `<w:drawing>`, `<mc:AlternateContent>` & legacy `<w:pict>`; writes full ECMA-376 `<pic:pic>`',
      '`Image` ribbon button (`Ctrl+Shift+I`), `/image`, `Ctrl+V` screenshot paste, 25–100% scale & captions',
    ],
  },
  {
    index: '03',
    tag: 'LEGAL OOXML & PDF FORENSICS · #23 #24 #33',
    title: 'Per-Clause Redlines, PNG Figures & PDF Sanitization',
    summary:
      'Built for real counterparty contract negotiation and archival compliance without losing corporate OOXML themes or leaving redacted PDF text underneath.',
    bullets: [
      '1-Click per-clause `Accept #N` / `Reject #N` + pure-Rust RFC 2083 PNG figure blitting',
      'Physical removal of intersecting PDF glyph runs + ISO 32000 `/ObjStm` parser',
      'Archival PDF/A-2b XMP conformance + RFC 6234 SHA-256 `/RustPadSeal` verification',
    ],
  },
  {
    index: '04',
    tag: 'SECURITY & DLP · #9 #25 #26',
    title: 'Zero-Crate Hardening, MDM DLP & WinHTTP TLS',
    summary:
      'Designed to pass CISO, defense, and financial procurement reviews with zero transitive crates.io packages and deterministic parser ceilings.',
    bullets: [
      'Bounded DEFLATE zip-bomb guards (200:1 max ratio), ZipSlip rejection & MAX_XML_DEPTH=128',
      'System MDM policy.json locks AI to localhost or allowlisted Sovereign VPC gateways',
      'Native Windows `winhttp.dll` Schannel TLS honoring corporate PAC proxies & Root CAs',
    ],
  },
];

export const AI_PROVIDERS: AiProviderSpec[] = [
  {
    id: 'nour-code',
    name: 'nour-code / vLLM (Air-Gapped)',
    mode: 'LOCAL HTTP',
    endpoint: 'http://127.0.0.1:8000/v1/chat/completions',
    defaultModel: 'nour-code',
    transport: 'Pure-Rust std::net::TcpStream + MDM EnterprisePolicy DLP check',
    cliExample:
      'rustpad ai MSA_Redline.docx --provider nour-code --task summarize -o counsel_brief.md',
    notes:
      'Permitted even under Strict Air-Gapped MDM policy (`air_gapped_mode: true`). Zero external DNS lookups, zero API keys, and 100% on-device sovereignty.',
  },
  {
    id: 'ollama',
    name: 'Ollama (Local Workstation)',
    mode: 'LOCAL HTTP',
    endpoint: 'http://127.0.0.1:11434/v1/chat/completions',
    defaultModel: 'qwen2.5:7b',
    transport: 'Pure-Rust TcpStream + /api/tags live model auto-discovery',
    cliExample:
      'rustpad ai report.pdf --provider ollama --model qwen2.5:7b --task sections -o report_v2.docx',
    notes:
      'Click "Detect Local" inside the AI Studio Dock to query 127.0.0.1:11434/api/tags and populate installed sovereign local models as 1-click pills.',
  },
  {
    id: 'lm-studio',
    name: 'LM Studio / llama.cpp',
    mode: 'LOCAL HTTP',
    endpoint: 'http://127.0.0.1:1234/v1/chat/completions',
    defaultModel: 'local-model',
    transport: 'Pure-Rust TcpStream HTTP/1.1 + chunked transfer decoder',
    cliExample:
      'rustpad ai contract.docx --provider lm-studio --task grammar -o contract_clean.docx',
    notes:
      'Works offline with LM Studio (:1234) and llama-server; automatically strips `<think>...</think>` reasoning traces before inserting OOXML blocks.',
  },
  {
    id: 'openai',
    name: 'OpenAI / Sovereign VPC HTTPS',
    mode: 'CLOUD HTTPS',
    endpoint: 'https://api.openai.com/v1/chat/completions',
    defaultModel: 'gpt-4o-mini',
    transport: 'Native Win32 WinHTTP Schannel TLS (Enterprise Root CA + Proxy) / Policy-Governed',
    cliExample:
      'OPENAI_API_KEY=sk-... rustpad ai proposal.docx --provider openai --task description',
    notes:
      'Uses native `winhttp.dll` (`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`) on Windows for Zscaler/Netskope TLS inspection compatibility — and is automatically blocked when MDM `allow_cloud_ai: false` is set.',
  },
  {
    id: 'claude',
    name: 'Anthropic Claude (Policy-Governed)',
    mode: 'CLOUD HTTPS',
    endpoint: 'https://api.anthropic.com/v1/messages',
    defaultModel: 'claude-3-5-sonnet-latest',
    transport: 'Native WinHTTP Schannel / OS TLS + SHA-256 Audit Trail logging',
    cliExample:
      'ANTHROPIC_API_KEY=sk-ant-... rustpad ai contract.pdf --provider claude --task ask --prompt "Flag indemnity caps"',
    notes:
      'Dispatches isolated `<document_context>` prompts when permitted by `allowed_ai_hosts` in `policy.json`, keeping API keys strictly in volatile session RAM.',
  },
];

export const CRATE_LEDGER: CrateSpec[] = [
  {
    crateName: 'rustpad (app)',
    layer: 'Enterprise Studio & CLI',
    standard: 'Win32 + X11 + 13 CLI Ops',
    responsibility:
      'Native desktop Word/PDF studio, Legal Redline bar, DLP Governance modal & 13-command enterprise CLI.',
    keyModules: 'app/src/{main,state,word_view,pdf_view,ui_paint,commands}.rs',
  },
  {
    crateName: 'power_tools',
    layer: 'DLP, WAL, SBOM & AI',
    standard: 'CycloneDX 1.5 + WinHTTP',
    responsibility:
      'MDM policy.json DLP engine, SHA-256 AuditTrail, Crash WAL, Lockfiles, WinHTTP/TCP AI & CycloneDX SBOM.',
    keyModules: 'crates/power_tools/src/{enterprise,ai_agent,converter,spellcheck,ocr}.rs',
  },
  {
    crateName: 'doc_model',
    layer: 'OpenXML AST & Redline',
    standard: 'Track Changes + LCS Diff',
    responsibility:
      'Document AST, RevisionMeta (<w:ins>/<w:del>), word-level diff_redline, Classification, RTL & Undo.',
    keyModules: 'crates/doc_model/src/{lib,templates}.rs',
  },
  {
    crateName: 'docx_io',
    layer: 'Lossless Word & Media Codec',
    standard: 'ECMA-376 OpenXML + DrawingML',
    responsibility:
      'Bounded XML parser, <w:drawing>/<mc:AlternateContent>/<w:pict> figures, <w:ins>/<w:del> & passthrough.',
    keyModules: 'crates/docx_io/src/{reader,writer,xml_lite}.rs',
  },
  {
    crateName: 'pdf_engine',
    layer: 'Forensic PDF & Seals',
    standard: 'ISO 32000 ObjStm + SHA-256',
    responsibility:
      'PDF 1.5+ /ObjStm & PNG predictor unpacker, forensic sanitize_redactions, /RustPadSeal signer & verifier.',
    keyModules: 'crates/pdf_engine/src/{parser,writer,lib}.rs',
  },
  {
    crateName: 'pdf_create',
    layer: 'PDF 1.7 & PDF/A-2b',
    standard: 'ISO 19005-2 + Image XObjects',
    responsibility:
      'Paginated vector PDF & PDF/A-2b generator with /DCTDecode & /FlateDecode image figures, tables & redlines.',
    keyModules: 'crates/pdf_create/src/lib.rs',
  },
  {
    crateName: 'rustpad_zip',
    layer: 'Hardened ZIP & Crypto',
    standard: 'PKZIP + DEFLATE + SHA-256',
    responsibility:
      'From-scratch DEFLATE/Zlib, bounded zip-bomb & ZipSlip guards, CRC32, Adler32 & RFC 6234 SHA-256.',
    keyModules: 'crates/rustpad_zip/src/{checksum,inflate,deflate,zip_archive}.rs',
  },
  {
    crateName: 'rustpad_render',
    layer: '60–120Hz 2D & Image Codecs',
    standard: 'TrueType + PNG + JPEG + BMP',
    responsibility:
      'Scanline fills, TrueType rasterizer, RFC 2083 PNG, ITU-T.81 JPEG & BMP/DIB decoders + bilinear scaler.',
    keyModules: 'crates/rustpad_render/src/{ttf,canvas,theme,png_codec,jpeg_codec}.rs',
  },
  {
    crateName: 'rustpad_window',
    layer: 'VSync Windowing & Print',
    standard: 'Win32 GDI + libX11 + CUPS',
    responsibility:
      '16ms 60Hz tick, Win32 WM_PAINT & X11 MotionNotify coalescing, HiDPI, clipboard & OS Print Spooler.',
    keyModules: 'crates/rustpad_window/src/{win32,linux_x11,lib}.rs',
  },
];

export const CLI_RECIPES: CliRecipe[] = [
  {
    label: 'Offline Legal Contract Redline (ECMA-376 <w:ins> / <w:del>)',
    badge: 'REDLINE',
    command:
      'rustpad redline Base_MSA_v1.docx Counterparty_v2.docx -o MSA_Redline.docx --author "General Counsel"',
    detail:
      'Computes word-level LCS revisions offline and emits a standards-compliant OpenXML .docx with tracked insertions and deletions.',
  },
  {
    label: 'Forensic PDF Redaction Sanitize + PDF/A-2b SHA-256 Seal',
    badge: 'FORENSICS',
    command:
      'rustpad sanitize Exhibit.pdf -o Clean.pdf && rustpad sign Clean.pdf -o Sealed.pdf --pdfa && rustpad verify Sealed.pdf',
    detail:
      'Physically purges underlying glyph runs inside redaction boxes, embeds PDF/A-2b XMP metadata, and seals content with RFC 6234 SHA-256.',
  },
  {
    label: 'CycloneDX 1.5 SBOM & Strict Air-Gapped MDM Policy',
    badge: 'CISO / DLP',
    command:
      'rustpad sbom -o rustpad-v2.2.0.cdx.json && rustpad policy --init-airgapped .rustpad-policy.json',
    detail:
      'Generates a machine-readable CycloneDX 1.5 bill of materials proving 0 crates.io deps and locks AI sockets strictly to 127.0.0.1.',
  },
  {
    label: 'DLP-Governed Local AI & Accessibility Tree Audit',
    badge: 'AI + A11Y',
    command:
      'rustpad ai MSA_Redline.docx --provider nour-code --task summarize -o brief.md && rustpad inspect --a11y MSA_Redline.docx',
    detail:
      'Runs air-gapped local inference over pure-Rust HTTP and exports the semantic UIA/AT-SPI accessibility landmark tree as JSON.',
  },
];

export const SCOPE_MATRIX = {
  builtFor: [
    'Writers, legal counsel & engineers who demand ultra-smooth 60–120Hz responsiveness, instant range selection & calm Warm Paper UX',
    'Air-gapped legal, defense, healthcare & financial teams requiring 100% offline desktop sovereignty with 0 telemetry',
    'Counterparty .docx contract redlining (1-click per-clause Accept/Reject), lossless corporate archive passthrough & PNG figures',
    'Forensic PDF stream sanitization, PDF 1.5+ /ObjStm parsing, PDF/A-2b archival export, SHA-256 seals & CycloneDX SBOM',
  ],
  nonGoals: [
    'Legacy VBA macro execution (`vbaProject.bin` is preserved passively on passthrough, never executed — eliminating macro malware)',
    'Proprietary cloud collaboration accounts, telemetry beacons, or 300 MB Electron/Chromium installers',
    'Hardware smartcard PKCS#11 X.509 certificate chains (v2.2 uses deterministic zero-dependency SHA-256 `/RustPadSeal` manifests)',
  ],
};

export const TOUR_STEPS: TourStep[] = [
  {
    step: '01 / 05',
    targetSelector: '#ux-playground',
    title: 'UX Is the #1 Feature — Smooth Sheet & Table Creator Sandbox',
    body: 'Click + Create Table ▾ to test the live visual hover grid & 1-click Invoice/Tracker presets, edit cells with Tab, or style text with the Floating Ivory Bar.',
  },
  {
    step: '02 / 05',
    targetSelector: '#enterprise-matrix',
    title: '60–120Hz Native Rust vs. Cloud & Electron Suites',
    body: 'Compare RustPad v2.2 across UI latency, effortless table authoring, offline DLP sovereignty, legal redlining, and forensic PDF sanitization.',
  },
  {
    step: '03 / 05',
    targetSelector: '#studio',
    specimenId: 'word-ai',
    title: 'Live Redline, Forensic PDF & DLP Switchboard',
    body: 'Inspect all 4 v2.2 desktop views: Smooth Word + Table Studio, Forensic PDF + SHA-256 Seal, Split Governance Audit, and Editorial Hub.',
  },
  {
    step: '04 / 05',
    targetSelector: '#ai-studio',
    title: 'Selection AI Polish & Air-Gapped MDM DLP',
    body: 'Highlight any sentence and click AI Polish to rewrite just that phrase via local Ollama / nour-code or corporate WinHTTP.',
  },
  {
    step: '05 / 05',
    targetSelector: '#cli',
    title: 'Headless Legal, Forensic & CISO CLI',
    body: 'Automate `rustpad redline`, `rustpad sanitize`, `rustpad sign --pdfa`, `rustpad verify`, and `rustpad sbom` directly in CI.',
  },
];
