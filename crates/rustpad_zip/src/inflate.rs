//! RFC 1951 DEFLATE & RFC 1950 Zlib Decompressor (Store, Fixed Huffman, Dynamic Huffman).

use crate::{adler32, ZipError};

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u64,
    bit_count: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_buf: 0,
            bit_count: 0,
        }
    }

    #[inline]
    fn ensure_bits(&mut self, n: u8) {
        while self.bit_count < n && self.pos < self.data.len() {
            self.bit_buf |= (self.data[self.pos] as u64) << self.bit_count;
            self.pos += 1;
            self.bit_count += 8;
        }
    }

    #[inline]
    fn read_bits(&mut self, n: u8) -> Result<u32, ZipError> {
        if n == 0 {
            return Ok(0);
        }
        self.ensure_bits(n);
        if self.bit_count < n {
            return Err(ZipError("Unexpected EOF in DEFLATE bitstream".into()));
        }
        let val = (self.bit_buf & ((1u64 << n) - 1)) as u32;
        self.bit_buf >>= n;
        self.bit_count -= n;
        Ok(val)
    }

    fn align_byte(&mut self) {
        let drop = self.bit_count % 8;
        self.bit_buf >>= drop;
        self.bit_count -= drop;
    }

    fn read_u16_le(&mut self) -> Result<u16, ZipError> {
        let lo = self.read_bits(8)? as u16;
        let hi = self.read_bits(8)? as u16;
        Ok(lo | (hi << 8))
    }
}

struct HuffmanTree {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl HuffmanTree {
    fn from_lengths(lengths: &[u8]) -> Result<Self, ZipError> {
        let mut counts = [0u16; 16];
        for &len in lengths {
            if len as usize >= 16 {
                return Err(ZipError("Invalid Huffman code length > 15".into()));
            }
            if len > 0 {
                counts[len as usize] += 1;
            }
        }
        let mut offsets = [0usize; 16];
        let mut total = 0usize;
        for bits in 1..16 {
            offsets[bits] = total;
            total += counts[bits] as usize;
        }
        let mut symbols = vec![0u16; total.max(1)];
        for (sym, &len) in lengths.iter().enumerate() {
            if len > 0 {
                let idx = offsets[len as usize];
                symbols[idx] = sym as u16;
                offsets[len as usize] += 1;
            }
        }
        Ok(Self { counts, symbols })
    }

    fn decode(&self, br: &mut BitReader<'_>) -> Result<u16, ZipError> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: usize = 0;
        for len in 1..=15 {
            code |= br.read_bits(1)? as i32;
            let count = self.counts[len] as i32;
            if code - count < first {
                return Ok(self.symbols[index + (code - first) as usize]);
            }
            index += count as usize;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(ZipError("Invalid Huffman code encountered".into()))
    }
}

fn fixed_trees() -> (HuffmanTree, HuffmanTree) {
    let mut lit_lengths = [0u8; 288];
    lit_lengths[0..=143].fill(8);
    lit_lengths[144..=255].fill(9);
    lit_lengths[256..=279].fill(7);
    lit_lengths[280..=287].fill(8);
    let dist_lengths = [5u8; 32];
    (
        HuffmanTree::from_lengths(&lit_lengths).unwrap(),
        HuffmanTree::from_lengths(&dist_lengths).unwrap(),
    )
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn read_dynamic_trees(br: &mut BitReader<'_>) -> Result<(HuffmanTree, HuffmanTree), ZipError> {
    let hlit = br.read_bits(5)? as usize + 257;
    let hdist = br.read_bits(5)? as usize + 1;
    let hclen = br.read_bits(4)? as usize + 4;

    let mut cl_lengths = [0u8; 19];
    for i in 0..hclen {
        cl_lengths[CL_ORDER[i]] = br.read_bits(3)? as u8;
    }
    let cl_tree = HuffmanTree::from_lengths(&cl_lengths)?;

    let total = hlit + hdist;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        let sym = cl_tree.decode(br)?;
        match sym {
            0..=15 => lengths.push(sym as u8),
            16 => {
                let prev = *lengths
                    .last()
                    .ok_or_else(|| ZipError("Repeat code 16 with no previous length".into()))?;
                let repeat = br.read_bits(2)? as usize + 3;
                for _ in 0..repeat {
                    lengths.push(prev);
                }
            }
            17 => {
                let repeat = br.read_bits(3)? as usize + 3;
                lengths.resize(lengths.len() + repeat, 0);
            }
            18 => {
                let repeat = br.read_bits(7)? as usize + 11;
                lengths.resize(lengths.len() + repeat, 0);
            }
            _ => return Err(ZipError("Invalid code length symbol".into())),
        }
    }
    if lengths.len() > total {
        lengths.truncate(total);
    }
    let lit_tree = HuffmanTree::from_lengths(&lengths[..hlit])?;
    let dist_tree = HuffmanTree::from_lengths(&lengths[hlit..])?;
    Ok((lit_tree, dist_tree))
}

pub fn deflate_decompress(input: &[u8]) -> Result<Vec<u8>, ZipError> {
    let mut br = BitReader::new(input);
    let mut out = Vec::with_capacity(input.len().saturating_mul(2).max(256));

    loop {
        let bfinal = br.read_bits(1)?;
        let btype = br.read_bits(2)?;
        match btype {
            0 => {
                br.align_byte();
                let len = br.read_u16_le()? as usize;
                let nlen = br.read_u16_le()?;
                if (len as u16) ^ 0xFFFF != nlen {
                    return Err(ZipError("Stored block LEN/NLEN mismatch".into()));
                }
                for _ in 0..len {
                    out.push(br.read_bits(8)? as u8);
                }
            }
            1 | 2 => {
                let (lit_tree, dist_tree) = if btype == 1 {
                    fixed_trees()
                } else {
                    read_dynamic_trees(&mut br)?
                };
                loop {
                    let sym = lit_tree.decode(&mut br)? as usize;
                    if sym < 256 {
                        out.push(sym as u8);
                    } else if sym == 256 {
                        break;
                    } else if sym <= 285 {
                        let idx = sym - 257;
                        let length =
                            LEN_BASE[idx] as usize + br.read_bits(LEN_EXTRA[idx])? as usize;
                        let dist_sym = dist_tree.decode(&mut br)? as usize;
                        if dist_sym >= 30 {
                            return Err(ZipError("Invalid distance symbol".into()));
                        }
                        let distance = DIST_BASE[dist_sym] as usize
                            + br.read_bits(DIST_EXTRA[dist_sym])? as usize;
                        if distance == 0 || distance > out.len() {
                            return Err(ZipError("LZ77 distance out of bounds".into()));
                        }
                        let start = out.len() - distance;
                        for i in 0..length {
                            let b = out[start + i];
                            out.push(b);
                        }
                    } else {
                        return Err(ZipError("Reserved literal/length symbol".into()));
                    }
                }
            }
            _ => return Err(ZipError("Reserved DEFLATE block type 3".into())),
        }

        if bfinal == 1 {
            break;
        }
    }

    Ok(out)
}

pub fn zlib_decompress(input: &[u8]) -> Result<Vec<u8>, ZipError> {
    if input.len() < 6 {
        return Err(ZipError("Zlib stream too short".into()));
    }
    let cmf = input[0];
    let flg = input[1];
    if (cmf & 0x0F) != 8 {
        return Err(ZipError("Unsupported Zlib compression method".into()));
    }
    if ((cmf as u16) * 256 + (flg as u16)) % 31 != 0 {
        return Err(ZipError("Invalid Zlib header checksum".into()));
    }
    let fdict = (flg & 0x20) != 0;
    let payload_start = if fdict { 6 } else { 2 };
    let trimmed = trim_trailing_pdf_ws(input);
    if trimmed.len() < payload_start + 4 {
        return Err(ZipError("Truncated Zlib payload".into()));
    }
    let out = deflate_decompress(&trimmed[payload_start..trimmed.len() - 4])?;
    let expected = u32::from_be_bytes([
        trimmed[trimmed.len() - 4],
        trimmed[trimmed.len() - 3],
        trimmed[trimmed.len() - 2],
        trimmed[trimmed.len() - 1],
    ]);
    let actual = adler32(&out);
    if expected != actual {
        return Err(ZipError("Zlib Adler32 checksum mismatch".into()));
    }
    Ok(out)
}

fn trim_trailing_pdf_ws(mut s: &[u8]) -> &[u8] {
    while let Some(&last) = s.last() {
        if last == b'\n' || last == b'\r' || last == b' ' {
            s = &s[..s.len() - 1];
        } else {
            break;
        }
    }
    s
}
