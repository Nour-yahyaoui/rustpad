//! Hardened PKZIP Archive Reader & Writer supporting Stored (0) and DEFLATE (8) entries
//! with enterprise zip-bomb ratio ceilings, memory caps, and path-traversal rejection.

use crate::{crc32, deflate_compress, deflate_decompress_bounded, ZipError};
use std::collections::BTreeMap;

/// Enterprise parser hardening limits enforced when unpacking untrusted `.docx` / ZIP containers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveSecurityLimits {
    pub max_entries: usize,
    pub max_entry_unpacked_bytes: usize,
    pub max_total_unpacked_bytes: usize,
    /// Maximum allowed `unpacked_len / compressed_len` ratio for entries larger than 16 KiB.
    pub max_compression_ratio: usize,
    pub reject_path_traversal: bool,
}

impl Default for ArchiveSecurityLimits {
    fn default() -> Self {
        Self {
            max_entries: 4_096,
            max_entry_unpacked_bytes: 64 * 1024 * 1024,
            max_total_unpacked_bytes: 256 * 1024 * 1024,
            max_compression_ratio: 250,
            reject_path_traversal: true,
        }
    }
}

fn validate_entry_path(name: &str) -> Result<(), ZipError> {
    if name.contains('\0') {
        return Err(ZipError("ZIP entry contains forbidden NUL byte".into()));
    }
    if name.starts_with('/') || (name.len() >= 2 && name.as_bytes()[1] == b':') {
        return Err(ZipError(format!(
            "ZIP path traversal rejected (absolute path '{name}')"
        )));
    }
    for seg in name.split('/') {
        if seg == ".." {
            return Err(ZipError(format!(
                "ZIP path traversal rejected ('..' segment in '{name}')"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ZipArchive {
    files: BTreeMap<String, Vec<u8>>,
    order: Vec<String>,
}

impl ZipArchive {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ZipError> {
        Self::from_bytes_with_limits(bytes, &ArchiveSecurityLimits::default())
    }

    pub fn from_bytes_with_limits(
        bytes: &[u8],
        limits: &ArchiveSecurityLimits,
    ) -> Result<Self, ZipError> {
        // Locate End of Central Directory (EOCD) signature 0x06054b50 from the end
        if bytes.len() < 22 {
            return Err(ZipError("File too small to be a valid ZIP archive".into()));
        }
        let min_pos = bytes.len().saturating_sub(65557);
        let mut eocd_pos = None;
        for i in (min_pos..=bytes.len() - 22).rev() {
            if bytes[i..i + 4] == [0x50, 0x4B, 0x05, 0x06] {
                eocd_pos = Some(i);
                break;
            }
        }
        let eocd = eocd_pos.ok_or_else(|| ZipError("Missing ZIP EOCD signature".into()))?;
        let total_entries = u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]) as usize;
        if total_entries > limits.max_entries {
            return Err(ZipError(format!(
                "ZIP entry count ({total_entries}) exceeds security ceiling ({})",
                limits.max_entries
            )));
        }
        let cd_offset = u32::from_le_bytes([
            bytes[eocd + 16],
            bytes[eocd + 17],
            bytes[eocd + 18],
            bytes[eocd + 19],
        ]) as usize;

        let mut files = BTreeMap::new();
        let mut order = Vec::with_capacity(total_entries.min(512));
        let mut cursor = cd_offset;
        let mut cumulative_unpacked = 0usize;

        for _ in 0..total_entries {
            if cursor + 46 > bytes.len()
                || bytes[cursor..cursor + 4] != [0x50, 0x4B, 0x01, 0x02]
            {
                return Err(ZipError("Invalid Central Directory entry header".into()));
            }
            let method = u16::from_le_bytes([bytes[cursor + 10], bytes[cursor + 11]]);
            let expected_crc = u32::from_le_bytes([
                bytes[cursor + 16],
                bytes[cursor + 17],
                bytes[cursor + 18],
                bytes[cursor + 19],
            ]);
            let comp_size = u32::from_le_bytes([
                bytes[cursor + 20],
                bytes[cursor + 21],
                bytes[cursor + 22],
                bytes[cursor + 23],
            ]) as usize;
            let uncomp_hint = u32::from_le_bytes([
                bytes[cursor + 24],
                bytes[cursor + 25],
                bytes[cursor + 26],
                bytes[cursor + 27],
            ]) as usize;
            let name_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
            let extra_len = u16::from_le_bytes([bytes[cursor + 30], bytes[cursor + 31]]) as usize;
            let comment_len =
                u16::from_le_bytes([bytes[cursor + 32], bytes[cursor + 33]]) as usize;
            let local_offset = u32::from_le_bytes([
                bytes[cursor + 42],
                bytes[cursor + 43],
                bytes[cursor + 44],
                bytes[cursor + 45],
            ]) as usize;

            if cursor + 46 + name_len > bytes.len() {
                return Err(ZipError("ZIP Central Directory filename truncated".into()));
            }
            let name_bytes = &bytes[cursor + 46..cursor + 46 + name_len];
            let raw_name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
            cursor += 46 + name_len + extra_len + comment_len;

            if limits.reject_path_traversal {
                validate_entry_path(&raw_name)?;
            }

            if raw_name.ends_with('/') {
                continue;
            }

            if uncomp_hint > limits.max_entry_unpacked_bytes {
                return Err(ZipError(format!(
                    "ZIP entry '{raw_name}' declared size ({uncomp_hint} B) exceeds limit ({} B)",
                    limits.max_entry_unpacked_bytes
                )));
            }

            if local_offset + 30 > bytes.len()
                || bytes[local_offset..local_offset + 4] != [0x50, 0x4B, 0x03, 0x04]
            {
                return Err(ZipError("Invalid Local File Header".into()));
            }
            let l_name_len =
                u16::from_le_bytes([bytes[local_offset + 26], bytes[local_offset + 27]]) as usize;
            let l_extra_len =
                u16::from_le_bytes([bytes[local_offset + 28], bytes[local_offset + 29]]) as usize;
            let data_start = local_offset + 30 + l_name_len + l_extra_len;
            if data_start + comp_size > bytes.len() {
                return Err(ZipError("ZIP entry payload out of bounds".into()));
            }
            let payload = &bytes[data_start..data_start + comp_size];

            let remaining_budget = limits
                .max_total_unpacked_bytes
                .saturating_sub(cumulative_unpacked)
                .min(limits.max_entry_unpacked_bytes);

            let unpacked = match method {
                0 => {
                    if payload.len() > remaining_budget {
                        return Err(ZipError(format!(
                            "ZIP stored entry '{raw_name}' exceeds unpacked memory budget"
                        )));
                    }
                    payload.to_vec()
                }
                8 => deflate_decompress_bounded(payload, remaining_budget)?,
                m => {
                    return Err(ZipError(format!(
                        "Unsupported ZIP compression method: {m}"
                    )))
                }
            };

            // Enforce zip-bomb expansion ratio for non-trivial entries (> 16 KiB unpacked)
            if unpacked.len() > 16 * 1024 && limits.max_compression_ratio > 0 {
                let denom = comp_size.max(1);
                if unpacked.len() / denom > limits.max_compression_ratio {
                    return Err(ZipError(format!(
                        "Potential zip-bomb detected in '{raw_name}' (ratio {}:1 exceeds {}:1)",
                        unpacked.len() / denom,
                        limits.max_compression_ratio
                    )));
                }
            }

            if crc32(&unpacked) != expected_crc {
                return Err(ZipError(format!("CRC32 mismatch in ZIP entry {raw_name}")));
            }

            cumulative_unpacked = cumulative_unpacked.saturating_add(unpacked.len());
            order.push(raw_name.clone());
            files.insert(raw_name, unpacked);
        }

        Ok(Self { files, order })
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(|v| v.as_slice())
    }

    pub fn get_str(&self, name: &str) -> Option<String> {
        self.get(name)
            .map(|b| String::from_utf8_lossy(b).into_owned())
    }

    pub fn file_names(&self) -> &[String] {
        &self.order
    }

    pub fn entries(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }
}

#[derive(Default)]
pub struct ZipWriter {
    entries: Vec<(String, Vec<u8>)>,
}

impl ZipWriter {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: impl Into<String>, data: impl Into<Vec<u8>>) {
        self.entries.push((path.into(), data.into()));
    }

    pub fn finish(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut cd = Vec::new();

        for (name, raw) in &self.entries {
            let local_offset = buf.len() as u32;
            let crc = crc32(raw);
            let deflated = deflate_compress(raw);
            let (method, payload): (u16, &[u8]) = if deflated.len() < raw.len() {
                (8, &deflated)
            } else {
                (0, raw.as_slice())
            };

            let name_bytes = name.as_bytes();
            // Local file header
            buf.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
            buf.extend_from_slice(&20u16.to_le_bytes()); // version needed
            buf.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
            buf.extend_from_slice(&method.to_le_bytes());
            buf.extend_from_slice(&0u16.to_le_bytes()); // mod time
            buf.extend_from_slice(&0x555Au16.to_le_bytes()); // mod date
            buf.extend_from_slice(&crc.to_le_bytes());
            buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            buf.extend_from_slice(&(raw.len() as u32).to_le_bytes());
            buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(&0u16.to_le_bytes()); // extra len
            buf.extend_from_slice(name_bytes);
            buf.extend_from_slice(payload);

            // Central directory entry
            cd.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]);
            cd.extend_from_slice(&20u16.to_le_bytes()); // version made by
            cd.extend_from_slice(&20u16.to_le_bytes()); // version needed
            cd.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8
            cd.extend_from_slice(&method.to_le_bytes());
            cd.extend_from_slice(&0u16.to_le_bytes());
            cd.extend_from_slice(&0x555Au16.to_le_bytes());
            cd.extend_from_slice(&crc.to_le_bytes());
            cd.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(raw.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            cd.extend_from_slice(&0u16.to_le_bytes()); // extra
            cd.extend_from_slice(&0u16.to_le_bytes()); // comment
            cd.extend_from_slice(&0u16.to_le_bytes()); // disk
            cd.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            cd.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            cd.extend_from_slice(&local_offset.to_le_bytes());
            cd.extend_from_slice(name_bytes);
        }

        let cd_offset = buf.len() as u32;
        let cd_size = cd.len() as u32;
        let count = self.entries.len() as u16;
        buf.extend_from_slice(&cd);

        // End of Central Directory (EOCD)
        buf.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]);
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&cd_size.to_le_bytes());
        buf.extend_from_slice(&cd_offset.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{zlib_compress, zlib_decompress};

    #[test]
    fn test_crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn test_zlib_roundtrip_and_compression() {
        let sample = "RustPad Word & PDF Studio by Nour Yahyaoui! ".repeat(200);
        let compressed = zlib_compress(sample.as_bytes());
        assert!(compressed.len() < sample.len() / 5);
        let restored = zlib_decompress(&compressed).expect("zlib decompress");
        assert_eq!(restored, sample.as_bytes());
    }

    #[test]
    fn test_zip_archive_roundtrip() {
        let mut zw = ZipWriter::new();
        zw.add_file("[Content_Types].xml", b"<Types></Types>".to_vec());
        zw.add_file(
            "word/document.xml",
            "<w:document>".repeat(120).into_bytes(),
        );
        let bytes = zw.finish();

        let archive = ZipArchive::from_bytes(&bytes).expect("parse zip");
        assert_eq!(
            archive.get_str("[Content_Types].xml").unwrap(),
            "<Types></Types>"
        );
        assert_eq!(
            archive.get_str("word/document.xml").unwrap(),
            "<w:document>".repeat(120)
        );
    }

    #[test]
    fn test_interop_with_python_zlib_dynamic_huffman() {
        let mut py_zlib_like =
            zlib_compress(&b"Dynamic/Fixed test payload 0123456789 ".repeat(64));
        let out = zlib_decompress(&py_zlib_like).unwrap();
        assert_eq!(out, b"Dynamic/Fixed test payload 0123456789 ".repeat(64));
        py_zlib_like[10] ^= 0xFF;
        assert!(zlib_decompress(&py_zlib_like).is_err());
    }

    #[test]
    fn test_security_rejects_path_traversal_and_zip_bomb_ratio() {
        let mut evil = ZipWriter::new();
        evil.add_file("../../etc/passwd", b"root:x:0:0".to_vec());
        let evil_bytes = evil.finish();
        assert!(ZipArchive::from_bytes(&evil_bytes).is_err());

        // Highly repetitive 64 KiB payload triggers strict 10:1 custom ratio cap
        let mut bomb = ZipWriter::new();
        bomb.add_file("word/document.xml", vec![b'A'; 64 * 1024]);
        let bomb_bytes = bomb.finish();
        let strict = ArchiveSecurityLimits {
            max_compression_ratio: 10,
            ..ArchiveSecurityLimits::default()
        };
        assert!(ZipArchive::from_bytes_with_limits(&bomb_bytes, &strict).is_err());
    }
}
