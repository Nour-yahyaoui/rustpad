# Build and Package the Windows Executable

This guide is aligned with RustPad `v3.0.13`. Build artifacts belong in `target/` or the GitHub Release, not in the source tree. Do not overwrite a published release unless you deliberately intend to replace its artifact.

## Build locally on Windows

Install the stable Rust toolchain and a Windows C/C++ linker appropriate to your Rust target, then run from the repository root:

```powershell
cargo test --workspace
cargo build --release -p rustpad
```

The executable is written to `target\release\rustpad.exe`.

To build and upload using the repository script, use an existing release tag and authenticated GitHub CLI:

```powershell
.\scripts\build-windows-exe.ps1 -ReleaseTag v3.0.13 -Upload
```

For a source-only build without upload, omit `-Upload`. Keep certificates and private keys out of Git; see [`scripts/sign-windows.ps1`](../scripts/sign-windows.ps1) for the signing flow.

## Build from Linux or WSL

The cross-build helper documents its own prerequisites and accepts a release tag:

```bash
chmod +x scripts/build-windows-cross.sh
./scripts/build-windows-cross.sh v3.0.13
```

Add `--upload` only when intentionally publishing to an existing GitHub Release and the GitHub CLI is authenticated.

## GitHub Actions release build

Use the existing [release workflow](../.github/workflows/release.yml) for automated builds. Release artifacts and checksums should be attached to the corresponding version tag. Do not copy the legacy workflow template over the active workflow unless you have reviewed the diff.

For future releases, replace `v3.0.13` with the actual release tag. Keep generated executables, `target/`, and release bundles out of Git unless the project explicitly changes its artifact policy.
