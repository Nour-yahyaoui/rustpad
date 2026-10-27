//! Pure-Rust CRC32, Adler32, SHA-256, RFC 1951 DEFLATE, RFC 1950 Zlib, and hardened PKZIP engine.
//! Powers `.docx` OpenXML archives, `.pdf` `/FlateDecode` & `/ObjStm` streams, PNG codecs,
//! WAL crash-recovery frames, and cryptographic SHA-256 document seals with zero external dependencies.

pub mod checksum;
pub mod deflate;
pub mod inflate;
pub mod zip_archive;

pub use checksum::{adler32, crc32, sha256, sha256_hex};
pub use deflate::{deflate_compress, zlib_compress};
pub use inflate::{
    deflate_decompress, deflate_decompress_bounded, zlib_decompress, zlib_decompress_bounded,
};
pub use zip_archive::{ArchiveSecurityLimits, ZipArchive, ZipWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipError(pub String);

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ZipError: {}", self.0)
    }
}

impl std::error::Error for ZipError {}
