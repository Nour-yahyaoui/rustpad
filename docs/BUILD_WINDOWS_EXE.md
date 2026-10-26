# How to Build & Push `rustpad.exe` (Windows 64-bit)

Because **RustPad** (`rustpad`) has **zero external `crates.io` dependencies** and uses pure Win32 (`user32.dll`, `gdi32.dll`, `comdlg32.dll` — built into every Windows 10/11 PC), `rustpad.exe` compiles into a **single standalone ~3.1 MB `.exe` file** with no installer or DLL folder needed.

> **Important Git Best Practice:** Do **not** commit `target/` into Git. Instead, attach `rustpad.exe` to **GitHub Releases** (which the **"Download v1.0.0 Release"** button on [`rustpad-studio.vercel.app`](https://rustpad-studio.vercel.app) already points to!), or optionally place a copy in `public/downloads/rustpad.exe` if you also want Vercel to host a direct download link.

---

## Option 1 — Easiest (Zero Windows PC Needed): Let GitHub Actions Build & Attach `rustpad.exe` in the Cloud

We prepared a ready-to-run workflow template at [`scripts/github-workflow-release-exe.yml`](../scripts/github-workflow-release-exe.yml).

Run this once from your terminal (or paste the file into `.github/workflows/release.yml` directly in the GitHub Web UI):

```bash
git pull origin main
mkdir -p .github/workflows
cp scripts/github-workflow-release-exe.yml .github/workflows/release.yml
git add .github/workflows/release.yml
git commit -m "ci: build and attach Windows rustpad.exe on GitHub Releases"
git push origin main
```

Then trigger it in **either** of two ways:
1. **Attach `rustpad.exe` to your existing `v1.0.0` release**: Open **GitHub -> Actions -> Build & Publish RustPad Binaries (.exe + Linux) -> Run workflow**, leave `v1.0.0`, and click **Run workflow**. In ~2 minutes, `rustpad.exe`, `rustpad-v1.0.0-windows-x86_64.exe`, `rustpad-v1.0.0-linux-x86_64`, and `SHA256SUMS.txt` appear on `https://github.com/Nour-yahyaoui/word/releases/tag/v1.0.0`.
2. **Or push any new tag**:
   ```bash
   git tag v1.2.0
   git push origin v1.2.0
   ```

---

## Option 2 — Build `rustpad.exe` Locally on Windows (PowerShell) & Upload

### 1. Prerequisites (one-time)
Install Rust and GitHub CLI in PowerShell:
```powershell
winget install Rustlang.Rustup
winget install GitHub.cli
```
*(Tip: If you don't have Visual Studio C++ Build Tools installed, run `rustup default stable-x86_64-pc-windows-gnu` so Rust compiles `.exe` files without needing the 6 GB Visual Studio installer.)*

### 2. Build & Upload in One Command
```powershell
git clone https://github.com/Nour-yahyaoui/word.git
cd word

# Builds target\dist\rustpad.exe + SHA-256 and uploads directly to GitHub Release v1.0.0:
gh auth login
.\scripts\build-windows-exe.ps1 -ReleaseTag v1.0.0 -Upload
```

Or manually without the script:
```powershell
cargo build --release -p rustpad
gh release upload v1.0.0 target\release\rustpad.exe --clobber
```

---

## Option 3 — Cross-Compile `rustpad.exe` from Linux or WSL

Because RustPad only links standard Windows system libraries (`kernel32`, `user32`, `gdi32`, `comdlg32`), you can build `rustpad.exe` right from Ubuntu / Debian / WSL:

```bash
sudo apt update && sudo apt install -y mingw-w64
chmod +x scripts/build-windows-cross.sh

# Builds target/dist/rustpad.exe and uploads it to GitHub Release v1.0.0:
./scripts/build-windows-cross.sh v1.0.0 --upload
```

---

## Optional Bonus — Host `rustpad.exe` Directly on Your Vercel Landing Page Too

If you want visitors on `https://rustpad-studio.vercel.app/downloads/rustpad.exe` to download the `.exe` directly from Vercel without even visiting GitHub:

```bash
cp target/dist/rustpad.exe public/downloads/rustpad.exe
git add public/downloads/rustpad.exe
git commit -m "chore(downloads): add standalone Windows rustpad.exe binary"
git push origin main
```
*(Because `rustpad.exe` is only ~3.1 MB, both GitHub and Vercel serve it cleanly with zero Git LFS setup required.)*

---

## Optional Polish — Hide the Console Window When Double-Clicking `rustpad.exe`

By default, `rustpad.exe` supports both **GUI mode** (double-click) and **CLI mode** (`rustpad.exe convert ...`, `rustpad.exe --help`). If you want double-clicking `rustpad.exe` in Windows Explorer to never show a background terminal window, add this line at the very top of `app/src/main.rs` before building:

```rust
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
```
