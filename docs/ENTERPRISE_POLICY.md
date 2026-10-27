# Enterprise Deployment, MDM Policy & Compliance Guide

RustPad `v2.0` is built for regulated legal, financial, healthcare, defense, and engineering organizations that require **100% offline desktop sovereignty**, **air-gapped Data Loss Prevention (DLP)**, and **zero third-party open-source dependency sprawl**.

---

## 1. MDM / Group Policy Configuration (`policy.json`)

RustPad automatically loads an enterprise governance policy at startup using the following precedence order:
1. Path specified in environment variable `RUSTPAD_POLICY_PATH`
2. Workspace-local `.rustpad-policy.json`
3. **Linux system-wide MDM path:** `/etc/rustpad/policy.json` (deployable via Ansible, Puppet, Salt, or `.deb`/`.rpm` config management)
4. **Windows system-wide MDM path:** `%ProgramData%\RustPad\policy.json` (deployable via Microsoft Intune, GPO, or SCCM)

### Strict Air-Gapped Policy Example

Generate a strict air-gapped policy template directly from the binary:
```bash
rustpad policy --init-airgapped /etc/rustpad/policy.json
```

```json
{
  "organization": "Sovereign Legal & Engineering Corp",
  "air_gapped_mode": true,
  "allow_cloud_ai": false,
  "allowed_ai_hosts": ["127.0.0.1", "localhost", "::1"],
  "default_classification": "confidential",
  "require_redaction_sanitization": true,
  "audit_log_enabled": true
}
```

### Hybrid On-Premises VPC AI Gateway Example

Organizations running an internal vLLM / Ollama / TGI inference cluster inside a private VPC can allowlist only their internal inference hostname while keeping public SaaS AI (`api.openai.com`, `api.anthropic.com`, etc.) hard-blocked:

```json
{
  "organization": "Regulated Financial Services",
  "air_gapped_mode": false,
  "allow_cloud_ai": false,
  "allowed_ai_hosts": ["127.0.0.1", "localhost", "llm-gateway.internal.corp"],
  "default_classification": "internal",
  "require_redaction_sanitization": true,
  "audit_log_enabled": true
}
```

On Windows 10/11 workstations, HTTPS requests to `llm-gateway.internal.corp` automatically use native **`winhttp.dll`** (`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`), honoring corporate PAC/WPAD proxies and Enterprise Root CAs installed in the Windows Certificate Store.

---

## 2. Headless Compliance & Legal Ops CLI Pipelines

Every capability in the RustPad desktop GUI is exposed as a deterministic, scriptable CLI command suitable for CI/CD, e-discovery, and document-management pipelines:

```bash
# 1. Automated Legal Contract Redline (Offline Word-Level ECMA-376 <w:ins>/<w:del>)
rustpad redline Master_MSA_v1.docx Counterparty_Markup_v2.docx \
  -o MSA_Redline_Comparison.docx --author "Legal Ops"

# 2. Forensic PDF Redaction Sanitization (Purges underlying text streams + scrubs metadata)
rustpad sanitize Discovery_Exhibit_Marked.pdf -o Discovery_Exhibit_Sanitized.pdf

# 3. Archival PDF/A-2b Conformance + Cryptographic SHA-256 Tamper Seal
rustpad sign Discovery_Exhibit_Sanitized.pdf \
  -o Discovery_Exhibit_Final.pdf --signer "Compliance Officer" --pdfa

# 4. Tamper Verification (Returns non-zero exit code if any text/page byte was altered)
rustpad verify Discovery_Exhibit_Final.pdf

# 5. CycloneDX 1.5 Software Bill of Materials for CISO / Procurement Review
rustpad sbom -o rustpad-sbom.cdx.json

# 6. UIA / AT-SPI Semantic Accessibility Tree Export
rustpad inspect --a11y MSA_Redline_Comparison.docx
```

---

## 3. Tamper-Evident SHA-256 Audit Trail

When `"audit_log_enabled": true` is active, RustPad records security-relevant operations (`STUDIO_BOOT`, `DOC_OPENED`, `DOC_SAVED`, `TRACK_CHANGES_TOGGLED`, `REVISIONS_ACCEPTED_ALL`, `PDF_FORENSIC_SANITIZE`, `PDF_SHA256_SEALED`, `AI_DLP_BLOCKED`, `OS_PRINT_SPOOL`) into a **cryptographically hash-chained ledger**:

```text
entry_sha256 = SHA256(seq | timestamp_epoch_s | action | subject | actor | prev_sha256)
```

Users and compliance auditors can inspect and verify chain integrity live inside the desktop studio via the **Status Bar DLP Pill** (`Ctrl+K` -> `Open Enterprise DLP Policy & SHA-256 Audit Chain Inspector`).
