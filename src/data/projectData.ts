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

export const RELEASE_VERSION = 'v3.0.12';
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
    value: '3 Modes',
    label: 'Native Dark Mode',
    detail: 'Midnight · Hybrid Dark · Daylight + OS DWM',
  },
  {
    value: '60–120 Hz',
    label: 'Ultra-Smooth Native UX',
    detail: 'VSync coalesced · Spring scroll · Floating bar',
  },
  {
    value: '100+ MB',
    label: 'Async .docx Intelligence',
    detail: 'LazyZip64 · Repair · Diff/Merge · Redact',
  },
  {
    value: '0 Deps',
    label: '< 90 ms Offline Studio',
    detail: 'Win32 GDI, X11, Pure-Rust OCR & 19 CLI ops',
  },
];

export const ENTERPRISE_BENCHMARKS: EnterpriseBenchmarkRow[] = [
  {
    capability: 'Native Dark Mode & Print Fidelity',
    rustpad: 'Midnight + Hybrid Dark Desk + Daylight (Win32 DWM & X11 Titlebar)',
    cloudSaas: 'CSS Filter Inversion Artifacts',
    electronApps: 'Theme Flicker & Mutated Saved Colors',
    legacySuites: 'Inconsistent Legacy Dialog Chrome',
  },
  {
    capability: 'OOXML Validation, Repair & Privacy',
    rustpad: 'CRC32/XML Validator · Corrupt ZIP Salvage · Permanent .docx/.pdf Redact',
    cloudSaas: 'Refuses Corrupt Uploads · Cloud Indexing',
    electronApps: 'Lossy Markdown Import Drops XML/Notes',
    legacySuites: 'Opaque Binary Recovery & Hidden Metadata Leaks',
  },
  {
    capability: '30–100+ MB Multi-Screenshot .docx',
    rustpad: 'Async 32 MiB Worker · LazyZip64 · Pull XML · MCU Strip Codec',
    cloudSaas: 'Upload Ceilings & Browser Tab Hang',
    electronApps: 'Multi-GB RGBA Renderer OOM Crash',
    legacySuites: 'Multi-Second Freeze & Swap Thrash',
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
    capability: 'Inline Mathematical Formulas',
    rustpad: '83+ Native Inline Math Templates (Alt+=, /math, Multi-Span Sub/Superscript)',
    cloudSaas: 'Iframe LaTeX / Slow Web Equations',
    electronApps: 'External Copy-Paste or KaTeX Plugins',
    legacySuites: 'Heavy Modal Equation Builders',
  },
];

export const STUDIO_SPECIMENS: StudioSpecimen[] = [
  {
    id: 'word-studio',
    index: '01',
    label: 'v3.0 Word + Dark Mode',
    shortLabel: 'Word v3.0',
    badge: 'DARK MODE + REDLINE',
    windowTitle: 'RustPad v3.0 — Enterprise_MSA_Redline.docx · Dark Mode, Inline Math & Table Studio',
    image: '/screenshots/word-studio.png',
    alt: 'RustPad v3.0 Word Studio showing Native Dark Mode, 60–120Hz smooth sheet rendering, async streaming 100+ MB large-doc engine, inline math formulas, footnotes/endnotes, and ECMA-376 Track Changes legal redlines.',
    summary:
      'Switch seamlessly between Daylight, Midnight Dark Mode, and Hybrid Dark Desk (Ctrl+Shift+D) with zero-mutation render-time ink adaptation, inspect OOXML security & hidden text (Ctrl+Shift+J), and review legal redlines in 1 click.',
    highlights: [
      {
        tag: '3-Mode Dark Studio',
        detail: 'Daylight, Midnight & Hybrid Dark Desk + native Win11 DWMWA_USE_IMMERSIVE_DARK_MODE titlebars',
      },
      {
        tag: 'OOXML Intelligence',
        detail: 'Footnotes, endnotes, hyperlinks, section breaks, corruption repair & hidden-text inspector',
      },
      {
        tag: 'Diff, Merge & Redact',
        detail: 'Char-level redlines, 3-way conflict-aware smart .docx merge & permanent PII/metadata scrubbing',
      },
    ],
  },
  {
    id: 'pdf-studio',
    index: '02',
    label: 'Forensic PDF & Seal',
    shortLabel: 'Forensic PDF',
    badge: 'PDF/A-2B + SHA-256',
    windowTitle: 'RustPad v3.0.0 — Technical_Report.pdf · SHA-256 Sealed',
    image: '/screenshots/pdf-studio.png',
    alt: 'RustPad v3.0.0 Forensic PDF Studio with Sanitize Redactions, SHA-256 Tamper Seal, Bezier ink signature, stamp, and PDF 1.5+ ObjStm parser.',
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
    label: 'Inspector & DLP Audit',
    shortLabel: 'Governance',
    badge: 'INSPECTOR + DLP',
    windowTitle: 'RustPad v3.0.0 — Dual Split Workspace + Document Intelligence & DLP Ledger',
    image: '/screenshots/split-command-palette.png',
    alt: 'RustPad v3.0.0 Split Workspace with Document Security Inspector, Enterprise DLP Policy, and SHA-256 Hash-Chained Audit Trail.',
    summary:
      'Drag-resize side-by-side Word & PDF panes at 60–120Hz while auditing hidden <w:vanish/> runs, embedded macros/OLE, broken hyperlinks, duplicate paragraphs, and SHA-256 DLP events.',
    highlights: [
      {
        tag: 'Security Inspector',
        detail: 'Flags hidden runs, white-on-white stealth text, remote template injection, VBA & OLE',
      },
      {
        tag: 'MDM policy.json',
        detail: '1-click Strict Air-Gapped mode enforces 100% offline workstation compliance',
      },
      {
        tag: '98 Studio Actions',
        detail: 'Ctrl+Shift+D Dark Mode, Alt+= Math Library, Ctrl+Shift+J Inspector, Ctrl+K Palette',
      },
    ],
  },
  {
    id: 'home-launchpad',
    index: '04',
    label: 'Editorial Hub',
    shortLabel: 'Launchpad',
    badge: '7 TEMPLATES',
    windowTitle: 'RustPad v3.0.0 — Architectural Studio Launchpad & Starter Sheets',
    image: '/screenshots/home-hub.png',
    alt: 'RustPad v3.0.0 Editorial Home Launchpad with 7 starter templates including Enterprise MSA Legal Redline.',
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
    tag: 'LARGE-DOC & FIGURE ENGINE · #39 #40 #47',
    title: 'Virtualized 100+ MB .docx Streaming, Paste & Corner Resize',
    summary:
      'Open 30–100+ MB multi-page screenshot reports without OOM crashes — powered by header dimension probing, progressive viewport thumbnails, and zero-copy SharedBytes CoW.',
    bullets: [
      'Progressive viewport-only HiDPI streaming (max 3/frame, <=1440px) inside a strict 64 MiB LRU cache',
      'SharedBytes (`Arc<[u8]>`) zero-copy undo/snapshots + STORED ZIP fast-path (~500× faster saves)',
      'Pure-Rust JPEG/PNG/BMP (`Ctrl+Shift+I`, `/image`, `Ctrl+V` paste, 32,768px stitched captures)',
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
    tag: 'INLINE MATH & SECURITY · #9 #25 #66',
    title: '83+ Inline Math Templates, Zero-Crate Hardening & MDM DLP',
    summary:
      'Author classical multi-span inline mathematics inside prose and pass CISO procurement reviews with zero transitive crates.io packages.',
    bullets: [
      '83+ inline math templates (`Alt+=`, `Ctrl+Shift+L`, `/math`) across Core, Algebra, Calculus, Linear Algebra & Probability',
      'Bounded DEFLATE zip-bomb guards (200:1 max ratio), ZipSlip rejection & MAX_XML_DEPTH=128',
      'System MDM policy.json offline air-gap governance + SHA-256 hash-chained AuditTrail',
    ],
  },
];

export const CRATE_LEDGER: CrateSpec[] = [
  {
    crateName: 'rustpad (app)',
    layer: 'Enterprise Studio & CLI',
    standard: 'Win32 + X11 + 18 CLI Ops',
    responsibility:
      'Native desktop Word/PDF studio, Inline Math Library, Legal Redline bar, DLP Governance & 18-command CLI.',
    keyModules: 'app/src/{main,state,word_view,pdf_view,ui_paint,commands}.rs',
  },
  {
    crateName: 'power_tools',
    layer: 'DLP, WAL, Redact & SBOM',
    standard: 'CycloneDX 1.5 + SHA-256',
    responsibility:
      'MDM policy.json DLP engine, .docx PII Redactor, SHA-256 AuditTrail, Crash WAL, OCR & CycloneDX SBOM.',
    keyModules: 'crates/power_tools/src/{enterprise,security,converter,spellcheck,ocr}.rs',
  },
  {
    crateName: 'doc_model',
    layer: 'OpenXML AST, Math & Diff',
    standard: 'Track Changes + Inline Math',
    responsibility:
      'Document AST, 83+ Inline Math templates, RevisionMeta (<w:ins>/<w:del>), smart merge & Undo.',
    keyModules: 'crates/doc_model/src/{lib,math_library,templates}.rs',
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
    label: 'Validate, Repair Corrupt .docx & Inspect Security / Hidden Layers',
    badge: 'OOXML v3.0',
    command:
      'rustpad validate Contract.docx && rustpad inspect --security Contract.docx && rustpad repair Truncated.docx -o Recovered.docx',
    detail:
      'Verifies PKZIP CRC32 & ECMA-376 relationships, audits hidden <w:vanish/> / remote template risks, and salvages truncated .docx archives.',
  },
  {
    label: 'Character-Level Legal Redline, Structural Diff & 3-Way Smart Merge',
    badge: 'DIFF / MERGE',
    command:
      'rustpad compare Base.docx Rev.docx && rustpad redline Base.docx Rev.docx -o Redline.docx --char-level && rustpad merge Out.docx Base.docx CounselA.docx CounselB.docx',
    detail:
      'Computes char-level & formatting diffs, emits ECMA-376 <w:ins>/<w:del> redlines, and runs 3-way conflict-aware smart .docx merges.',
  },
  {
    label: 'Permanent .docx & .pdf Redaction + Strict Metadata & Hidden Scrubbing',
    badge: 'PRIVACY',
    command:
      'rustpad redact Deal.docx -o Deal_Redacted.docx --pii --term "Project Falcon" && rustpad clean-meta Deal_Redacted.docx -o Deal_Clean.docx',
    detail:
      'Permanently scrubs PII/terms across body, tables, headers, comments & Track Changes history via 0600 atomic writes.',
  },
  {
    label: 'Batch Directory Processing, HTML5/PDF/A-2b Export & CycloneDX SBOM',
    badge: 'BATCH + SBOM',
    command:
      'rustpad batch pdf ./contracts -o ./exports && rustpad convert Guide.docx Guide.html && rustpad sbom -o rustpad-v3.0.0.cdx.json',
    detail:
      'Converts entire .docx folders offline, exports Dark-Mode-ready HTML5 & PDF/A-2b, and emits a 0-dependency CycloneDX 1.5 SBOM.',
  },
];

export const SCOPE_MATRIX = {
  builtFor: [
    'Writers, legal counsel & engineers demanding Native Dark Mode (Midnight / Hybrid / Daylight) & 60–120Hz responsiveness',
    'Air-gapped legal, defense, healthcare & financial teams requiring 100% offline desktop sovereignty with 0 telemetry',
    'Lossless OOXML validation, corrupt .docx repair, char-level redlining, 3-way smart merge, footnotes/endnotes & permanent PII redaction',
    'Forensic PDF stream sanitization, PDF 1.5+ /ObjStm parsing, PDF/A-2b export, SHA-256 seals, pure-Rust OCR & batch CLI',
  ],
  nonGoals: [
    'Legacy VBA macro execution (`vbaProject.bin` is preserved passively or scrubbed via Privacy Cleaner, never executed)',
    'Proprietary cloud collaboration accounts, telemetry beacons, or 300 MB Electron/Chromium installers',
    'Hardware smartcard PKCS#11 X.509 certificate chains (v3.0 uses deterministic zero-dependency SHA-256 `/RustPadSeal` manifests)',
  ],
};

export const TOUR_STEPS: TourStep[] = [
  {
    step: '01 / 05',
    targetSelector: '#ux-playground',
    title: 'UX Is the #1 Feature — Smooth Sheet, Dark Mode & Table Studio',
    body: 'Toggle Dark / Light theme in the top bar, click + Create Table ▾ to test the live visual hover grid, or style text with the Floating Bar.',
  },
  {
    step: '02 / 05',
    targetSelector: '#enterprise-matrix',
    title: 'RustPad v3.0 Native Rust vs. Cloud & Electron Suites',
    body: 'Compare RustPad v3.0 across Native Dark Mode, OOXML validation/repair, 100+ MB .docx streaming, smart merge, and forensic redaction.',
  },
  {
    step: '03 / 05',
    targetSelector: '#studio',
    specimenId: 'word-studio',
    title: 'Live v3.0 Word, Inspector, Forensic PDF & DLP Switchboard',
    body: 'Inspect all 4 v3.0 desktop views: Dark-Mode-Capable Word Studio, Forensic PDF + SHA-256 Seal, Document Security Inspector, and Editorial Hub.',
  },
  {
    step: '04 / 05',
    targetSelector: '#architecture',
    title: '9 Pure-Rust Crates & Zero External Dependencies',
    body: 'Every PKZIP, DEFLATE, XML, TrueType, PNG/JPEG, Inline Math, and PDF 1.7 primitive lives in-tree — verifiable via `rustpad sbom`.',
  },
  {
    step: '05 / 05',
    targetSelector: '#cli',
    title: '18-Command Headless OOXML, Privacy, Forensic & Batch CLI',
    body: 'Automate `rustpad validate`, `repair`, `compare`, `merge`, `redact --pii`, `clean-meta`, `batch`, `sign --pdfa`, and `sbom` in CI.',
  },
];
