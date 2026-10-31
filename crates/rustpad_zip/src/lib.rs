//! Pure-Rust CRC32, Adler32, SHA-256, RFC 1951 DEFLATE, RFC 1950 Zlib, zero-copy `SharedBytes`,
//! and hardened PKZIP / `LazyZipArchive` engine.
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
pub use zip_archive::{ArchiveSecurityLimits, LazyZipArchive, ZipArchive, ZipWriter};

/// Reference-counted immutable byte slice (`O(1)` clone, `O(1)` zero-copy sub-slicing, `Send + Sync`)
/// so a 30–200+ MB `.docx` file can be read once into a single `Arc<[u8]>` while every embedded
/// screenshot slices directly into it without duplicating media bytes in memory.
#[derive(Clone)]
pub struct SharedBytes {
    data: std::sync::Arc<[u8]>,
    offset: usize,
    len: usize,
}

impl Default for SharedBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedBytes {
    pub fn new() -> Self {
        let empty: &[u8] = &[];
        Self {
            data: std::sync::Arc::from(empty),
            offset: 0,
            len: 0,
        }
    }

    pub fn from_vec(v: Vec<u8>) -> Self {
        let len = v.len();
        Self {
            data: std::sync::Arc::from(v),
            offset: 0,
            len,
        }
    }

    pub fn from_slice(s: &[u8]) -> Self {
        let len = s.len();
        Self {
            data: std::sync::Arc::from(s),
            offset: 0,
            len,
        }
    }

    /// Zero-copy `O(1)` sub-slice sharing the same backing `Arc<[u8]>` allocation.
    pub fn slice(&self, start: usize, end: usize) -> Self {
        let s = start.min(self.len);
        let e = end.clamp(s, self.len);
        Self {
            data: self.data.clone(),
            offset: self.offset + s,
            len: e - s,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.data[self.offset..self.offset + self.len]
    }
}

impl std::ops::Deref for SharedBytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for SharedBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl From<Vec<u8>> for SharedBytes {
    fn from(v: Vec<u8>) -> Self {
        Self::from_vec(v)
    }
}

impl From<&[u8]> for SharedBytes {
    fn from(s: &[u8]) -> Self {
        Self::from_slice(s)
    }
}

impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        (std::sync::Arc::ptr_eq(&self.data, &other.data)
            && self.offset == other.offset
            && self.len == other.len)
            || self.as_slice() == other.as_slice()
    }
}

impl Eq for SharedBytes {}

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedBytes({} B)", self.len)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipError(pub String);

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ZipError: {}", self.0)
    }
}

impl std::error::Error for ZipError {}
