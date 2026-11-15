# Security Policy & Enterprise Threat Model

**Project:** RustPad Studio (`v3.0.13`)
**Maintainer:** Nour Yahyaoui (<nourryahyaoui@gmail.com>)  
**Repository:** <https://github.com/Nour-yahyaoui/rustpad>

---

## 1. Supported Versions

| Version | Status | Notes |
| :--- | :--- | :--- |
| **`3.0.13`** | Current documented release | Zero external `crates.io` dependencies; see parser and PDF security boundaries below |
| `< 3.0.13` | Older release | Update to the current release and review its release-specific notes |
---

## 2. Zero-Dependency Supply-Chain Architecture

Enterprise desktop document editors routinely pull **300–900 transitive open-source packages** (C/C++ codec bindings, Electron/Chromium runtimes, third-party HTTP/TLS stacks, XML parsers, and ZIP extractors). Every transitive dependency represents an upstream supply-chain vector.

RustPad `v3.0.13` has no external `crates.io` dependencies in its Rust workspace (this does not eliminate all software or operating-system risk):
- **`Cargo.lock` contains `0` external `crates.io` dependencies.** Every line of PKZIP, RFC 1951 DEFLATE/Inflate, RFC 6234 SHA-256, ECMA-376 OpenXML, ISO 32000 PDF 1.7 / PDF/A-2b, TrueType `.ttf` glyph rasterization, Win32 GDI/WinHTTP, and Linux X11 protocol framing lives in-tree across 9 auditable workspace crates.
- **Machine-Readable SBOM:** Security teams can generate a standards-compliant **CycloneDX 1.5 JSON Software Bill of Materials** offline at any time:
  ```bash
  rustpad sbom -o rustpad-v3.0.13.cdx.json
  ```

---

## 3. Untrusted Document Parser Defenses

Opening untrusted `.docx` and `.pdf` attachments is one of the highest-risk workflows in legal, financial, and government environments. RustPad enforces deterministic memory and recursion ceilings:

| Vector | Defense Mechanism | Crate / Module | Default Ceiling |
| :--- | :--- | :--- | :--- |
| **DEFLATE / Zlib Zip Bombs** | `deflate_decompress_bounded` & `ArchiveSecurityLimits` abort decompression mid-stream when output bytes or compression ratio exceed thresholds | `crates/rustpad_zip` | `64 MiB` / entry · `256 MiB` total · `200:1` max ratio |
| **ZIP Path Traversal (`ZipSlip`)** | Rejects archive entries containing `..` parent segments, leading `/`, drive prefixes, or NUL bytes | `crates/rustpad_zip` | Strict zero-tolerance |
| **Billion-Laughs / Deep XML Nesting** | `parse_xml` enforces `MAX_XML_DEPTH = 128` to prevent stack exhaustion on adversarial `word/document.xml` payloads | `crates/docx_io` | `128` nested elements |
| **PDF redaction scope** | Removes intersecting modeled text items and overlapping modeled annotations; the parser does not model arbitrary images, form XObjects, or every PDF layer/filter | `crates/pdf_engine` | Limited model-only cleanup; not a universal forensic sanitizer |
| **PDF integrity digest** | `/Type /RustPadSeal` contains an unkeyed SHA-256 digest of selected modeled content. Verification detects mismatches against the embedded digest but does not authenticate the signer; an editor can recompute it | `crates/pdf_engine` | Integrity check only; not a digital signature |

---

## 4. Offline Behavior and Trust Boundaries

- **No bundled AI agent in v3.0.13:** the AI modules and their HTTP/WinHTTP bridges were removed. Core editing and parsing are local. This is not a blanket guarantee that OS services (for example, network printing) never use network facilities.
- **Do not use `/RustPadSeal` as proof of identity or consent.** The seal is an unkeyed checksum with a display-only signer label. For identity, non-repudiation, or legal signature validation, use a standards-based certificate-backed PDF digital signature and validate the certificate chain separately.
- **Do not treat PDF modeled-text redaction as complete for arbitrary PDFs.** Inspect the output with a full-featured forensic PDF tool; raster images, form XObjects, and unsupported layers are outside the current page model.
- **Arabic/RTL note:** RTL detection and alignment do not establish full Unicode bidi or Arabic contextual-shaping support. Validate the rendered/exported document in the intended viewer.

## 5. Windows Publisher Identity, PE Resources & SmartScreen / AV Guidance

In the `v3.0.13` codebase, `app/build.rs` embeds native Win32 PE `.rsrc` metadata directly into `rustpad.exe` with zero external crates:
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
