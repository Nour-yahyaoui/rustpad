# Security and Documentation Review — RustPad `v3.0.13`

Review date: 2026-11-13. This is a scoped source/documentation review, not a formal security audit. The Rust toolchain was unavailable in the review environment, so Rust tests could not be executed.

| Priority | Review item | Result / remaining boundary |
| :--- | :--- | :--- |
| P0 | PDF redaction and hidden/image layers | Hardened redaction geometry against non-finite/zero-sized input; sanitizer removes overlapping modeled text, notes, highlights, stamps, and ink signatures, drops page-wide watermarks, and scrubs modeled title/author metadata. The current PDF model does not represent arbitrary raster images, form XObjects, or every PDF layer/filter, so this is **not** complete forensic redaction for arbitrary PDFs. CLI and docs now warn explicitly. |
| P0 | Distinguish SHA-256 seal from digital signature | Clarified in code comments, CLI help/output, README, and `SECURITY.md`: `/RustPadSeal` is an unkeyed digest over selected modeled content. It can detect a mismatch but does not authenticate the displayed signer and can be recomputed by someone who edits the PDF. |
| P1 | Corrupted DOCX/PDF test coverage | Added malformed/truncated-input no-panic regression tests for DOCX, PDF, and ZIP readers, including damaged ZIP header lengths. Existing DOCX repair and ZIP traversal/bomb tests remain. Execution is pending because `cargo`/`rustc` are not installed in this environment. |
| P1 | `unsafe` and ZIP operations | Source search found no `unsafe` blocks in `crates/rustpad_zip`, `crates/docx_io`, or `crates/pdf_engine`; added ZIP malformed-length/truncation tests. This is not a complete fuzzing, formal bounds-proof, or audit of unsafe code elsewhere (notably native windowing/FFI). |
| P2 | Version documentation | Root README and current security/build/inventory metadata now identify `v3.0.13`; old release history remains historical rather than being rewritten. Planning documents are marked as historical drafts. |
| P2 | Arabic/RTL rendering and shaping | Not implemented in this change. Current RTL detection/right alignment must not be read as full UAX #9 bidi or Arabic contextual shaping. README and security policy now state the limitation. |

## Important safe-use notes

- Do not rely on RustPad's current PDF redaction model alone for sensitive disclosure. Verify output with a forensic PDF tool that inspects image content, forms, annotations, and hidden layers.
- Do not treat the SHA-256 `/RustPadSeal` as a legal or identity signature. Use a certificate-backed PDF digital-signature implementation and verify its trust chain when signer authentication is required.
- Run `cargo test --workspace` in a Rust-enabled environment before merging these changes.
