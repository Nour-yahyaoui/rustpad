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

## 5. Windows Publisher Identity, PE Resources & SmartScreen / AV Guidance

Starting in `v2.2.0`, `app/build.rs` embeds native Win32 PE `.rsrc` metadata directly into `rustpad.exe` with zero external crates:
- **Embedded Multi-Frame App Icon (`RT_GROUP_ICON` ID `1`):** Crisp `16×16`, `32×32`, `48×48`, `64×64` 32-bpp DIB frames + `256×256` lossless PNG for File Explorer, Titlebar, Alt+Tab, and Windows 11 Taskbar.
- **Publisher & Version Metadata (`VS_VERSION_INFO`):**
  - `CompanyName`: `Nour Yahyaoui - RustPad Studio`
  - `ProductName`: `RustPad Desktop Studio`
  - `FileDescription`: `RustPad - Offline Enterprise Word (.docx) & PDF Desktop Studio`
  - `OriginalFilename`: `rustpad.exe`
- **Least-Privilege UAC & HiDPI Manifest (`RT_MANIFEST` ID `1`):** Explicitly declares `requestedExecutionLevel="asInvoker"` (`uiAccess="false"`), Windows 10/11 compatibility GUIDs, and `PerMonitorV2` DPI awareness.

### Why Windows Defender SmartScreen May Still Show *"Unknown Publisher"* on Fresh Releases
Windows distinguishes between **PE File Properties (`VS_VERSION_INFO`)** — visible under *Right-Click -> Properties -> Details* — and **Authenticode X.509 Code Signing** issued by a commercial Certificate Authority (DigiCert, Sectigo, GlobalSign, or Azure Trusted Signing, typically $200–$500/yr):
1. **Verify Release Integrity:** Always verify downloaded binaries against `SHA256SUMS.txt` attached to each GitHub Release:
   ```powershell
   Get-FileHash .\rustpad.exe -Algorithm SHA256
   ```
2. **Authenticode Signing with a `.pem`, `.pfx`, or Cloud HSM Certificate (`scripts/sign-windows.ps1`):**
   - **Never commit `.pem`, `.key`, or `.pfx` files to Git** (they are protected in `.gitignore`).
   - **Combined `.pem` (contains both `CERTIFICATE` and `PRIVATE KEY`):**
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath .\rustpad.exe -PemPath .\your-cert.pem
     ```
   - **Public `.pem` certificate + separate private key file:**
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath .\rustpad.exe -PemPath .\your-cert.pem -KeyPath .\private.key
     ```
   - **Public `.pem` from a Cloud HSM / USB Token CA (e.g., Certum SimplySign, SSL.com, DigiCert):** Log in to SimplySign Desktop (or plug in your USB token) and run with `-PemPath .\your-cert.pem` — the script binds the public `.pem` to your hardware/cloud key in `Cert:\CurrentUser\My` and timestamps via RFC 3161 (`http://timestamp.digicert.com`).
   - **Linux / macOS / WSL cross-signing with `osslsigncode`:**
     ```bash
     osslsigncode sign -certs your-cert.pem -key private.key \
       -n "RustPad Desktop Studio" -i "https://rustpad-studio.vercel.app" \
       -h sha256 -ts http://timestamp.digicert.com \
       -in rustpad.exe -out rustpad-signed.exe
     ```
3. **Open-Source Public CA Signing & Defender Whitelisting:** Maintainers can connect GitHub Actions to [SignPath Foundation](https://about.signpath.io/product/open-source) (free OSS Authenticode signing) and submit new release hashes to [Microsoft Security Intelligence (WDSI)](https://www.microsoft.com/en-us/wdsi/filesubmission) to clear ML false-positive heuristics within 24 hours.

---

## 6. Reporting a Vulnerability

Please **do not** open public GitHub issues for suspected parser memory-safety bugs, DLP bypasses, or redaction leaks.

Send a private disclosure to **Nour Yahyaoui** at **<nourryahyaoui@gmail.com>** with:
1. RustPad version (`rustpad --version`) and OS/architecture.
2. Minimal reproduction payload or CLI command (`rustpad inspect`, `rustpad sanitize`, etc.).
3. Expected vs. observed security boundary behavior.

We acknowledge vulnerability reports within **48 hours** and publish patched binaries + SHA-256 checksums via GitHub Releases once verified.
