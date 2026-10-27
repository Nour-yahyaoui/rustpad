# Contributing to RustPad Enterprise

Thank you for contributing to **RustPad**! RustPad is engineered as a serious, long-term open-source alternative to bloated desktop document & PDF suites, governed by strict systems-engineering invariants.

---

## 1. Non-Negotiable Architectural Charter

1. **Zero External Runtime Dependencies (`Cargo.lock` Invariant):**
   - Do **not** add `crates.io` dependencies to `Cargo.toml` in any workspace crate.
   - OS interaction must use narrow, well-documented `extern "system"` / `extern "C"` FFI boundaries isolated inside `crates/rustpad_window` or `crates/power_tools` (e.g. `user32`, `gdi32`, `comdlg32`, `winhttp` on Windows; `libX11` on Linux).
2. **100% Offline Sovereignty:**
   - Every core Word (`.docx`), PDF (`.pdf`), redlining, sanitization, OCR, spellcheck, and printing workflow must work with **zero network interfaces up**.
   - Any optional AI endpoint feature must pass through `EnterprisePolicy::validate_ai_endpoint` before opening a socket.
3. **Visual & Interaction Craft (`docs/STYLE.md`):**
   - Read [`docs/STYLE.md`](docs/STYLE.md) before modifying `app/src/{ui_paint,word_view,pdf_view}.rs`.
   - Maintain the **Warm Architectural Paper & Linen** daylight palette, crisp physical A4/Letter sheet geometry, exact TrueType glyph hit-testing, and responsive drag-resizable splitters.

---

## 2. Workspace Build & Verification Commands

```bash
# 1. Verify Cargo.lock still has zero external dependencies
grep -c 'source = "registry' Cargo.lock || true   # Must output 0

# 2. Run the full cross-crate unit & adversarial parser test suite
cargo test --workspace

# 3. Build stripped release executable
cargo build --release --bin rustpad

# 4. Smoke-test Enterprise CLI workflows
./target/release/rustpad sbom -o /tmp/sbom.cdx.json
./target/release/rustpad demo-files /tmp/rustpad-samples
./target/release/rustpad inspect /tmp/rustpad-samples/Enterprise_MSA_Redline.docx
./target/release/rustpad inspect --a11y /tmp/rustpad-samples/Enterprise_MSA_Redline.docx
./target/release/rustpad sign /tmp/rustpad-samples/Business_Invoice_Sample.pdf -o /tmp/sealed.pdf --pdfa
./target/release/rustpad verify /tmp/sealed.pdf
```

---

## 3. Issue, Pull Request & Commit Conventions

- Open or reference a tracked GitHub Issue describing the concrete subsystem (`[docx_io]`, `[pdf_engine]`, `[power_tools]`, `[ui]`, `[security]`).
- Include unit or round-trip regression tests inside the modified crate (`#[cfg(test)] mod tests`).
- Use clear Conventional Commit prefixes:
  - `feat(docx_io): ...`
  - `fix(pdf_engine): ...`
  - `sec(rustpad_zip): ...`
  - `docs(governance): ...`
- Ensure both `ubuntu-latest` and `windows-latest` CI jobs pass cleanly before merge.

---

## 4. License

By contributing to RustPad, you agree that your contributions will be licensed under the [MIT License](LICENSE).
