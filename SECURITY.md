# Security Policy & Enterprise Threat Model

**Project:** RustPad Enterprise Suite (`v2.2.0`)  
**Maintainer:** Nour Yahyaoui (<nourryahyaoui@gmail.com>)  
**Repository:** <https://github.com/Nour-yahyaoui/rustpad>

---

## 1. Supported Versions

| Version | Status | Supply-Chain Posture | Security Advisories |
| :--- | :--- | :--- | :--- |
| **`2.2.x`** | **Active LTS** | `0` third-party `crates.io` runtime dependencies | Bounded PNG/JPEG/BMP image decoders (`64 MiB`), full parser hardening, DLP policy, forensic PDF sanitize & SHA-256 seals |
| `2.0.x` – `2.1.x` | Supported | `0` third-party `crates.io` runtime dependencies | Full parser hardening, DLP policy, forensic PDF sanitize & SHA-256 seals |
| `1.3.x` | Maintenance | `0` third-party `crates.io` runtime dependencies | Upgrade to `2.0.0` recommended for zip-bomb & XML depth limits |
| `< 1.3.0` | End of Life | — | Unsupported |

---

## 2. Zero-Dependency Supply-Chain Architecture

Enterprise desktop document editors routinely pull **300–900 transitive open-source packages** (C/C++ codec bindings, Electron/Chromium runtimes, third-party HTTP/TLS stacks, XML parsers, and ZIP extractors). Every transitive dependency represents an upstream supply-chain vector.

RustPad `v2.0` eliminates third-party package supply-chain risk at the root:
- **`Cargo.lock` contains `0` external `crates.io` dependencies.** Every line of PKZIP, RFC 1951 DEFLATE/Inflate, RFC 6234 SHA-256, ECMA-376 OpenXML, ISO 32000 PDF 1.7 / PDF/A-2b, TrueType `.ttf` glyph rasterization, Win32 GDI/WinHTTP, and Linux X11 protocol framing lives in-tree across 9 auditable workspace crates.
- **Machine-Readable SBOM:** Security teams can generate a standards-compliant **CycloneDX 1.5 JSON Software Bill of Materials** offline at any time:
  ```bash
  rustpad sbom -o rustpad-v2.0.0.cdx.json
  ```

---

## 3.Untrusted Document Parser Defenses (`v2.0` Hardening)

Opening untrusted `.docx` and `.pdf` attachments is one of the highest-risk workflows in legal, financial, and government environments. RustPad enforces deterministic memory and recursion ceilings:

| Vector | Defense Mechanism | Crate / Module | Default Ceiling |
| :--- | :--- | :--- | :--- |
| **DEFLATE / Zlib Zip Bombs** | `deflate_decompress_bounded` & `ArchiveSecurityLimits` abort decompression mid-stream when output bytes or compression ratio exceed thresholds | `crates/rustpad_zip` | `64 MiB` / entry · `256 MiB` total · `200:1` max ratio |
| **ZIP Path Traversal (`ZipSlip`)** | Rejects archive entries containing `..` parent segments, leading `/`, drive prefixes, or NUL bytes | `crates/rustpad_zip` | Strict zero-tolerance |
| **Billion-Laughs / Deep XML Nesting** | `parse_xml` enforces `MAX_XML_DEPTH = 128` to prevent stack exhaustion on adversarial `word/document.xml` payloads | `crates/docx_io` | `128` nested elements |
| **Residual Redacted PDF Text Leaks** | `PdfDocument::sanitize_redactions` physically purges intersecting underlying glyph runs and annotations from PDF content streams instead of drawing black rectangles over selectable text | `crates/pdf_engine` | Physical stream purge + metadata scrub |
| **Post-Signature PDF Tampering** | `/Type /RustPadSeal` embeds a canonical RFC 6234 SHA-256 content & geometry digest verified via `rustpad verify <file.pdf>` | `crates/pdf_engine` | `256-bit` SHA-256 |

---

## 4. Air-Gapped Operation & Enterprise AI Data Loss Prevention (DLP)

- **100% Offline Core:** Launching `rustpad`, editing `.docx` / `.pdf` files, running legal redlines, sanitizing PDFs, and printing make **zero network calls** — no telemetry, no analytics beacons, no license pings, and no auto-updater background daemons.
- **MDM `policy.json` AI Lockout:** IT administrators can bind `/etc/rustpad/policy.json` (Linux) or `%ProgramData%\RustPad\policy.json` (Windows) with `"air_gapped_mode": true` and `"allow_cloud_ai": false`. Any attempt to invoke a non-loopback AI endpoint is blocked **before DNS resolution or socket creation** and logged to the SHA-256 hash-chained `AuditTrail`.
- **Corporate TLS Proxy & Root CA Trust:** On Windows workstations where outbound HTTPS is allowed through enterprise inspection proxies (Zscaler, Netskope, Cloudflare Gateway), RustPad uses native **`winhttp.dll` (`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`)**, delegating TLS 1.2/1.3 validation directly to Windows Schannel and the OS Enterprise Root Certificate Store.

---

## 5. Reporting a Vulnerability

Please **do not** open public GitHub issues for suspected parser memory-safety bugs, DLP bypasses, or redaction leaks.

Send a private disclosure to **Nour Yahyaoui** at **<nourryahyaoui@gmail.com>** with:
1. RustPad version (`rustpad --version`) and OS/architecture.
2. Minimal reproduction payload or CLI command (`rustpad inspect`, `rustpad sanitize`, etc.).
3. Expected vs. observed security boundary behavior.

We acknowledge vulnerability reports within **48 hours** and publish patched binaries + SHA-256 checksums via GitHub Releases once verified.
