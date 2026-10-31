//! Zero-Dependency Pure-Rust Windows PE Resource Generator (`app/build.rs`).
//!
//! When targeting `x86_64-pc-windows-msvc` (or any MSVC Windows target), emits a standards-compliant
//! 32-bit Win32 `.res` binary into `$OUT_DIR/rustpad.res` and passes it to `link.exe` for the
//! `rustpad` binary target (`cargo:rustc-link-arg-bin=rustpad=...`), embedding:
//!   1. `RT_ICON` (IDs `1..=N`) + `RT_GROUP_ICON` (ID `1`) from `assets/icons/rustpad.ico`
//!   2. `RT_VERSION` (ID `1`) `VS_VERSION_INFO` with Publisher (`CompanyName`), `ProductName`,
//!      `FileDescription`, `FileVersion`, `OriginalFilename`, and `LegalCopyright`
//!   3. `RT_MANIFEST` (ID `1`) with `requestedExecutionLevel="asInvoker"`, Windows 10/11
//!      compatibility GUIDs, and `PerMonitorV2` HiDPI awareness.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;
const LANG_EN_US: u16 = 0x0409;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../assets/icons/rustpad.ico");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ico_path = manifest_dir
        .join("..")
        .join("assets")
        .join("icons")
        .join("rustpad.ico");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let res_path = out_dir.join("rustpad.res");

    let res_bytes = build_win32_res_file(&ico_path);
    fs::write(&res_path, res_bytes).expect("failed to write OUT_DIR/rustpad.res");

    println!("cargo:rustc-link-arg-bin=rustpad={}", res_path.display());
    // Provision a 32 MiB main-thread stack on Windows MSVC (default is only 1 MiB) alongside
    // our 32 MiB-stack async streaming worker so massive enterprise `.docx` files never overflow.
    println!("cargo:rustc-link-arg-bin=rustpad=/STACK:33554432");
}

fn align4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

fn push_res_record(
    out: &mut Vec<u8>,
    type_id: u16,
    name_id: u16,
    mem_flags: u16,
    lang_id: u16,
    payload: &[u8],
) {
    align4(out);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes()); // DataSize
    out.extend_from_slice(&32u32.to_le_bytes()); // HeaderSize (32 bytes with ordinal Type & Name)
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&type_id.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&name_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    out.extend_from_slice(&mem_flags.to_le_bytes()); // MemoryFlags
    out.extend_from_slice(&lang_id.to_le_bytes()); // LanguageId
    out.extend_from_slice(&0u32.to_le_bytes()); // Version
    out.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    out.extend_from_slice(payload);
    align4(out);
}

fn build_win32_res_file(ico_path: &Path) -> Vec<u8> {
    let mut out = Vec::with_capacity(96 * 1024);

    // 0. Mandatory 32-bit `.res` empty preamble record (Type = 0, Name = 0, DataSize = 0)
    push_res_record(&mut out, 0, 0, 0, 0, &[]);

    // 1. Embed `RT_ICON` frames + `RT_GROUP_ICON` directory (ID = 1)
    if let Ok(ico) = fs::read(ico_path) {
        if ico.len() >= 6 {
            let reserved = u16::from_le_bytes([ico[0], ico[1]]);
            let ico_type = u16::from_le_bytes([ico[2], ico[3]]);
            let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
            if reserved == 0 && ico_type == 1 && count > 0 && ico.len() >= 6 + count * 16 {
                let mut grp_dir = Vec::with_capacity(6 + count * 14);
                grp_dir.extend_from_slice(&0u16.to_le_bytes());
                grp_dir.extend_from_slice(&1u16.to_le_bytes());
                grp_dir.extend_from_slice(&(count as u16).to_le_bytes());

                let mut valid_frames = 0u16;
                for i in 0..count {
                    let e = 6 + i * 16;
                    let width = ico[e];
                    let height = ico[e + 1];
                    let color_count = ico[e + 2];
                    let planes = u16::from_le_bytes([ico[e + 4], ico[e + 5]]).max(1);
                    let bit_count = u16::from_le_bytes([ico[e + 6], ico[e + 7]]);
                    let bytes_in_res =
                        u32::from_le_bytes([ico[e + 8], ico[e + 9], ico[e + 10], ico[e + 11]])
                            as usize;
                    let img_off =
                        u32::from_le_bytes([ico[e + 12], ico[e + 13], ico[e + 14], ico[e + 15]])
                            as usize;
                    if img_off + bytes_in_res <= ico.len() {
                        valid_frames += 1;
                        let icon_id = valid_frames;
                        push_res_record(
                            &mut out,
                            RT_ICON,
                            icon_id,
                            0x1010, // MOVEABLE | DISCARDABLE
                            LANG_EN_US,
                            &ico[img_off..img_off + bytes_in_res],
                        );

                        // 14-byte GRPICONDIRENTRY
                        grp_dir.push(width);
                        grp_dir.push(height);
                        grp_dir.push(color_count);
                        grp_dir.push(0);
                        grp_dir.extend_from_slice(&planes.to_le_bytes());
                        grp_dir.extend_from_slice(&bit_count.to_le_bytes());
                        grp_dir.extend_from_slice(&(bytes_in_res as u32).to_le_bytes());
                        grp_dir.extend_from_slice(&icon_id.to_le_bytes());
                    }
                }
                if valid_frames > 0 {
                    grp_dir[4..6].copy_from_slice(&valid_frames.to_le_bytes());
                    push_res_record(&mut out, RT_GROUP_ICON, 1, 0x1030, LANG_EN_US, &grp_dir);
                }
            }
        }
    }

    // 2. Embed `RT_VERSION` (`VS_VERSION_INFO`, ID = 1) with Publisher & Product Identity
    let version_blob = build_vs_version_info();
    push_res_record(&mut out, RT_VERSION, 1, 0x0030, LANG_EN_US, &version_blob);

    // 3. Embed `RT_MANIFEST` (`CREATEPROCESS_MANIFEST_RESOURCE_ID` = 1) with `asInvoker` & `PerMonitorV2`
    let manifest_xml = build_app_manifest_xml();
    push_res_record(
        &mut out,
        RT_MANIFEST,
        1,
        0x0030,
        LANG_EN_US,
        manifest_xml.as_bytes(),
    );

    out
}

fn push_utf16_nul(out: &mut Vec<u8>, s: &str) {
    for code_unit in s.encode_utf16() {
        out.extend_from_slice(&code_unit.to_le_bytes());
    }
    out.extend_from_slice(&0u16.to_le_bytes());
}

fn write_version_node<F>(
    out: &mut Vec<u8>,
    w_type: u16,
    key: &str,
    value_bytes: &[u8],
    w_value_len: u16,
    write_children: F,
) where
    F: FnOnce(&mut Vec<u8>),
{
    align4(out);
    let start = out.len();
    out.extend_from_slice(&0u16.to_le_bytes()); // Placeholder for wLength
    out.extend_from_slice(&w_value_len.to_le_bytes());
    out.extend_from_slice(&w_type.to_le_bytes());
    push_utf16_nul(out, key);
    align4(out);
    if !value_bytes.is_empty() {
        out.extend_from_slice(value_bytes);
    }
    write_children(out);
    let total_len = (out.len() - start) as u16;
    out[start..start + 2].copy_from_slice(&total_len.to_le_bytes());
}

fn write_version_string(out: &mut Vec<u8>, key: &str, val: &str) {
    let mut val_utf16 = Vec::with_capacity((val.len() + 1) * 2);
    let mut wchar_count: u16 = 0;
    for cu in val.encode_utf16() {
        val_utf16.extend_from_slice(&cu.to_le_bytes());
        wchar_count += 1;
    }
    val_utf16.extend_from_slice(&0u16.to_le_bytes());
    wchar_count += 1; // Include NUL terminator in WCHAR count per MSDN String struct spec

    write_version_node(out, 1, key, &val_utf16, wchar_count, |_| {});
}

fn build_vs_version_info() -> Vec<u8> {
    let major: u32 = env::var("CARGO_PKG_VERSION_MAJOR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let minor: u32 = env::var("CARGO_PKG_VERSION_MINOR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let patch: u32 = env::var("CARGO_PKG_VERSION_PATCH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let ver_ms: u32 = (major << 16) | (minor & 0xFFFF);
    let ver_ls: u32 = (patch << 16) | 0;
    let ver_quad = format!("{major}.{minor}.{patch}.0");
    let ver_semver = format!("{major}.{minor}.{patch}");

    // 52-byte VS_FIXEDFILEINFO
    let mut ffi = Vec::with_capacity(52);
    for d in [
        0xFEEF_04BDu32, // dwSignature
        0x0001_0000u32, // dwStrucVersion (1.0)
        ver_ms,         // dwFileVersionMS
        ver_ls,         // dwFileVersionLS
        ver_ms,         // dwProductVersionMS
        ver_ls,         // dwProductVersionLS
        0x0000_003Fu32, // dwFileFlagsMask (VS_FFI_FILEFLAGSMASK)
        0x0000_0000u32, // dwFileFlags
        0x0004_0004u32, // dwFileOS (VOS_NT_WINDOWS32)
        0x0000_0001u32, // dwFileType (VFT_APP)
        0x0000_0000u32, // dwFileSubtype
        0x0000_0000u32, // dwFileDateMS
        0x0000_0000u32, // dwFileDateLS
    ] {
        ffi.extend_from_slice(&d.to_le_bytes());
    }

    let mut root = Vec::with_capacity(1024);
    write_version_node(&mut root, 0, "VS_VERSION_INFO", &ffi, ffi.len() as u16, |out| {
        // Child 1: StringFileInfo -> "040904B0" (English US, Unicode UTF-16LE)
        write_version_node(out, 1, "StringFileInfo", &[], 0, |sfi| {
            write_version_node(sfi, 1, "040904B0", &[], 0, |st| {
                write_version_string(st, "CompanyName", "Nour Yahyaoui - RustPad Studio");
                write_version_string(
                    st,
                    "FileDescription",
                    "RustPad - Offline Enterprise Word (.docx) & PDF Desktop Studio",
                );
                write_version_string(st, "FileVersion", &ver_quad);
                write_version_string(st, "InternalName", "rustpad");
                write_version_string(
                    st,
                    "LegalCopyright",
                    "Copyright (c) 2026 Nour Yahyaoui. Open-Source MIT License.",
                );
                write_version_string(st, "OriginalFilename", "rustpad.exe");
                write_version_string(st, "ProductName", "RustPad Desktop Studio");
                write_version_string(st, "ProductVersion", &ver_semver);
            });
        });

        // Child 2: VarFileInfo -> Translation = [0x0409, 0x04B0] (4 bytes)
        let mut trans = Vec::with_capacity(4);
        trans.extend_from_slice(&0x0409u16.to_le_bytes());
        trans.extend_from_slice(&0x04B0u16.to_le_bytes());
        write_version_node(out, 1, "VarFileInfo", &[], 0, |vfi| {
            write_version_node(vfi, 0, "Translation", &trans, 4, |_| {});
        });
    });

    root
}

fn build_app_manifest_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="2.4.0.0"
    processorArchitecture="amd64"
    name="NourYahyaoui.RustPad.DesktopStudio"
    type="win32"
  />
  <description>RustPad Offline Enterprise Word and PDF Desktop Studio</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>"#
}
