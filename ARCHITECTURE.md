# RustPad v2.0 Enterprise — Systems Architecture

RustPad is a **pure-Rust, offline-first native desktop document and PDF suite** engineered with **zero third-party `crates.io` runtime dependencies**. It compiles into a single self-contained native executable (`rustpad` / `rustpad.exe`) for Windows (`Win32 GDI + WinHTTP`) and Linux (`X11`).

---

## 1. Nine-Crate Workspace Topology

```text
                        ┌───────────────────────────────────────────┐
                        │               app (`rustpad`)             │
                        │  Native Desktop Studio + Headless Ent CLI │
                        └──────┬──────────────┬──────────────┬──────┘
                               │              │              │
             ┌─────────────────▼──┐     ┌─────▼──────┐  ┌────▼─────────────┐
             │    power_tools     │     │rustpad_win │  │  rustpad_render  │
             │ • EnterprisePolicy │     │ • Win32 GDI│  │ • TrueType (.ttf)│
             │ • AuditTrail SHA256│     │ • Linux X11│  │ • Subpixel AA    │
             │ • WAL Crash Journal│     │ • Clipboard│  │ • 2D SDF Painter │
             │ • WinHTTP / TCP AI │     │ • OS Print │  │ • PNG Encoder    │
             └───┬────────┬───────┘     └────────────┘  └─────────┬────────┘
                 │        │                                       │
       ┌─────────▼──┐  ┌──▼──────────┐  ┌──────────────┐          │
       │  docx_io   │  │ pdf_engine  │  │  pdf_create  │          │
       │ • ECMA-376 │  │ • PDF 1.5+  │  │ • PDF 1.7    │          │
       │ • w:ins/del│  │   /ObjStm   │  │ • PDF/A-2b   │          │
       │ • comments │  │ • Sanitize  │  │ • Redlines   │          │
       │ • Passthru │  │ • Seal/Vrfy │  │ • XMP Packet │          │
       └─────┬──────┘  └──────┬──────┘  └──────┬───────┘          │
             │                │                │                  │
             └────────────────┼────────────────┴──────────────────┘
                              ▼
                   ┌────────────────────┐
                   │     doc_model      │
                   │ • Document AST     │
                   │ • RevisionMeta     │
                   │ • diff_redline     │
                   │ • UndoHistory      │
                   └──────────┬─────────┘
                              ▼
                   ┌────────────────────┐
                   │    rustpad_zip     │
                   │ • PKZIP Reader/Wrt │
                   │ • Bounded DEFLATE  │
                   │ • CRC32 / Adler32  │
                   │ • RFC 6234 SHA-256 │
                   └────────────────────┘
```

---

## 2. Subsystem Deep Dives

### 2.1 `rustpad_zip` — Hardened Archive & Cryptographic Primitives
- **PKZIP & RFC 1951 DEFLATE/Zlib:** Pure-Rust LZ77 + canonical Huffman compressor and bounded decompressor (`deflate_decompress_bounded`).
- **Zip-Bomb & Path-Traversal Defense:** `ArchiveSecurityLimits` enforces per-entry byte ceilings, aggregate archive byte ceilings, maximum compression ratios (`200:1` default), and strict `..` / absolute-path rejection.
- **RFC 6234 SHA-256:** In-tree 256-bit cryptographic digest (`sha256`, `sha256_hex`) powering PDF tamper-evident seals, hash-chained audit logs, and WAL crash-recovery frame verification.

### 2.2 `doc_model` & `docx_io` — Lossless ECMA-376 OpenXML & Legal Redlining
- **Lossless Archive Passthrough (`passthrough_parts`):** Unknown ZIP entries inside corporate `.docx` archives (`word/theme/theme1.xml`, `word/styles.xml`, `customXml/*`, embedded media, macros/vbaProject where present) are preserved byte-for-byte across load/edit/save cycles instead of being stripped.
- **ECMA-376 Track Changes (`<w:ins>` / `<w:del>`):** `TextSpan` carries optional `RevisionMeta { id, kind, author, date_iso }`. `docx_io` reads and emits `<w:ins>` and `<w:del><w:r><w:delText>` runs alongside `word/settings.xml` (`<w:trackRevisions/>`).
- **Threaded Review Comments (`word/comments.xml`):** Anchors `<w:commentRangeStart>`, `<w:commentRangeEnd>`, and `<w:commentReference>` to paragraph blocks and round-trips `word/comments.xml` + OPC relationships.
- **Offline Word-Level Legal Redline Engine (`Document::diff_redline`):** Computes LCS word-level diffs paragraph-by-paragraph between base and counterparty `.docx` revisions to produce standards-compliant redlined contracts offline.

### 2.3 `pdf_engine` & `pdf_create` — ISO 32000 PDF 1.5+, Forensic Sanitization & PDF/A-2b
- **PDF 1.5+ Compressed Object Streams (`/Type /ObjStm`) & PNG Predictors:** `scan_and_expand_objects` unpacks Flate-compressed object streams and reverses `/Predictor >= 10` PNG row filters emitted by Microsoft Word, LibreOffice, Chrome, and macOS Quartz.
- **Forensic Redaction Sanitizer (`sanitize_redactions`):** Unlike superficial black-rectangle overlays that leave underlying text copy-pasteable, RustPad physically removes intersecting `PdfTextItem` runs and overlapping annotations from page content streams and scrubs document metadata.
- **Archival PDF/A-2b & Cryptographic Seals:** Embeds `pdfaid:part=2`, `pdfaid:conformance=B` XMP metadata packets and `/Type /RustPadSeal` SHA-256 content manifests verifiable via `rustpad verify`.

### 2.4 `power_tools` — Enterprise DLP Policy, Audit Chain, WAL & Native WinHTTP AI
- **MDM Governance (`EnterprisePolicy`):** Parses JSON policies from `RUSTPAD_POLICY_PATH`, `/etc/rustpad/policy.json`, or `%ProgramData%\RustPad\policy.json` to enforce air-gapped cloud AI lockouts and default security classifications.
- **Native Windows Schannel TLS (`winhttp_tls`):** Binds directly to `winhttp.dll` (`extern "system"`) on Windows with `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` so corporate WPAD/PAC proxies and Enterprise Root CAs work out of the box without external binaries.
- **Crash-Safe WAL (`WalJournal`) & Advisory Locks (`AdvisoryFileLock`):** Frames dirty `.docx` state with `RUSTPAD-WAL-V2|<sha256>|<len>` headers for deterministic post-crash recovery and writes `.~<file>.rustpad.lock` markers for shared SMB/NFS drives.

---

## 3. Honest Architectural Trade-Offs (v2.0 Scope)

| Capability | RustPad v2.0 Enterprise | LibreOffice / MS Word | Trade-Off Rationale |
| :--- | :--- | :--- | :--- |
| **Binary Size & Deps** | **~2.5 MB · `0` `crates.io` deps** | 300 MB – 3 GB · hundreds of shared libs | Instant cold start, trivial security audit, portable `.exe` |
| **Track Changes & Comments** | `<w:ins>`, `<w:del>`, `word/comments.xml`, word-level redline diff | Full complex field codes, nested table cell merge revisions | Covers 95% of legal & engineering contract redlining without 2M LOC bloat |
| **PDF Redaction & Sealing** | Physical text-stream purge, PDF/A-2b XMP, SHA-256 `/RustPadSeal` | PKCS#7 / PAdES hardware smartcard X.509 chains | Deterministic SHA-256 tamper detection with zero OpenSSL/NSS dependency |
| **Complex Script Shaping** | Automatic RTL paragraph alignment (`Arabic`/`Hebrew`) + TrueType cmap | Full HarfBuzz contextual ligature & CJK vertical shaping | Keeps glyph rasterizer pure-Rust & zero-dependency |
