//! Pure-Rust CRC32, Adler32, RFC 1951 DEFLATE, RFC 1950 Zlib, and PKZIP archive engine.
//! Powers `.docx` OpenXML archives, `.pdf` `/FlateDecode` streams, PNG decoding/encoding,
//! and compressed local Time-Machine snapshots with zero external C dependencies.

pub mod checksum;
pub mod deflate;
pub mod inflate;
pub mod zip_archive;

pub use checksum::{adler32, crc32};
pub use deflate::{deflate_compress, zlib_compress};
pub use inflate::{deflate_decompress, zlib_decompress};
pub use zip_archive::{ZipArchive, ZipWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipError(pub String);

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ZipError: {}", self.0)
    }
}

impl std::error::Error for ZipError {}
