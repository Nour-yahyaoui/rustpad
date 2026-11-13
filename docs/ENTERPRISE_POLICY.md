# Enterprise Deployment and Governance Notes — RustPad `v3.0.13`

This page describes the governance features present in the current source. It is not a certification, network-firewall guarantee, or substitute for an organization's document-handling controls.

## 1. Workstation policy

RustPad reads a policy file from the first available location:

1. `RUSTPAD_POLICY_PATH`
2. `.rustpad-policy.json` in the working directory
3. `/etc/rustpad/policy.json` on Linux
4. `%ProgramData%\RustPad\policy.json` on Windows

Generate a policy template with:

```bash
rustpad policy --init-airgapped ./rustpad-policy.json
```

The current schema contains organization, `air_gapped_mode`, default classification, `require_redaction_sanitization`, and `audit_log_enabled` fields. For example:

```json
{
  "organization": "Example Organization",
  "air_gapped_mode": true,
  "default_classification": "confidential",
  "require_redaction_sanitization": true,
  "audit_log_enabled": true
}
```

The AI-agent integration and its endpoint allowlist/network controls described in older documentation were removed before v3.0.13. The policy setting is application-level governance; it is **not** a host firewall and does not prove that OS services (such as network printing) are offline.

## 2. Useful CLI workflows

```bash
# Validate or attempt recovery of a DOCX package
rustpad validate Contract.docx
rustpad repair Damaged.docx -o Recovered.docx

# Redact a DOCX; scrub hidden/revision content as configured by the command
rustpad redact Contract.docx -o Contract_Redacted.docx --term "Project Falcon" --pii
rustpad clean-meta Contract_Redacted.docx -o Contract_Clean.docx

# PDF sanitation applies only to redaction marks and content represented by RustPad's PDF model.
# It does not inspect arbitrary raster images, form XObjects, or every PDF layer/filter.
rustpad sanitize Marked.pdf -o Sanitized.pdf

# `/RustPadSeal` is an unkeyed modeled-content digest, not a certificate-backed signature.
rustpad sign Sanitized.pdf -o Digest_Checked.pdf --signer "Records Officer" --pdfa
rustpad verify Digest_Checked.pdf

# Generate a CycloneDX SBOM and inspect document information
rustpad sbom -o rustpad-v3.0.13.cdx.json
rustpad inspect --security Contract.docx
```

Use a full-featured forensic PDF tool to independently validate files before sensitive disclosure. Use a standards-based certificate-backed PDF digital signature when signer identity or legal signature validation is required; RustPad's signer string is not authenticated.

## 3. Audit trail

When `audit_log_enabled` is enabled, the application records selected events in a SHA-256 hash-chained audit trail. The chain can reveal changes relative to a trusted copy, but the hash chain is unkeyed and does not by itself establish who produced or approved the log. Store and protect audit records according to organizational policy.

## 4. Further security review

See [Security Policy](../SECURITY.md) and the [priority-by-priority review](SECURITY_REVIEW.md) for parser limits, PDF trust boundaries, test status, and known RTL limitations.
