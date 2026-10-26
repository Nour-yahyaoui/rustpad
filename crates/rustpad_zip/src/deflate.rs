//! Fast RFC 1951 DEFLATE (LZ77 + Fixed Huffman) and RFC 1950 Zlib Compressor.

use crate::adler32;

struct BitWriter {
    out: Vec<u8>,
    bit_buf: u32,
    bit_count: u8,
}

impl BitWriter {
    fn with_capacity(cap: usize) -> Self {
        Self {
            out: Vec::with_capacity(cap),
            bit_buf: 0,
            bit_count: 0,
        }
    }

    #[inline]
    fn write_bits(&mut self, val: u32, count: u8) {
        self.bit_buf |= (val & ((1u32 << count) - 1)) << self.bit_count;
        self.bit_count += count;
        while self.bit_count >= 8 {
            self.out.push((self.bit_buf & 0xFF) as u8);
            self.bit_buf >>= 8;
            self.bit_count -= 8;
        }
    }

    #[inline]
    fn write_huff(&mut self, code_msb: u32, len: u8) {
        // Huffman codes in DEFLATE are packed MSB-first into the LSB-first bitstream
        let mut rev = 0u32;
        for i in 0..len {
            if (code_msb & (1 << (len - 1 - i))) != 0 {
                rev |= 1 << i;
            }
        }
        self.write_bits(rev, len);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit_count > 0 {
            self.out.push((self.bit_buf & 0xFF) as u8);
        }
        self.out
    }
}

#[inline]
fn encode_fixed_lit(bw: &mut BitWriter, sym: usize) {
    match sym {
        0..=143 => bw.write_huff(0x30 + sym as u32, 8),
        144..=255 => bw.write_huff(0x190 + (sym as u32 - 144), 9),
        256..=279 => bw.write_huff(sym as u32 - 256, 7),
        280..=287 => bw.write_huff(0xC0 + (sym as u32 - 280), 8),
        _ => unreachable!(),
    }
}

fn encode_length(bw: &mut BitWriter, mut len: usize) {
    len = len.clamp(3, 258);
    const LEN_BASE: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LEN_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    let mut code_idx = 0;
    for i in (0..29).rev() {
        if len >= LEN_BASE[i] {
            code_idx = i;
            break;
        }
    }
    encode_fixed_lit(bw, 257 + code_idx);
    let extra = LEN_EXTRA[code_idx];
    if extra > 0 {
        bw.write_bits((len - LEN_BASE[code_idx]) as u32, extra);
    }
}

fn encode_distance(bw: &mut BitWriter, dist: usize) {
    const DIST_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    let mut code_idx = 0;
    for i in (0..30).rev() {
        if dist >= DIST_BASE[i] {
            code_idx = i;
            break;
        }
    }
    bw.write_huff(code_idx as u32, 5);
    let extra = DIST_EXTRA[code_idx];
    if extra > 0 {
        bw.write_bits((dist - DIST_BASE[code_idx]) as u32, extra);
    }
}

pub fn deflate_compress(input: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::with_capacity(input.len() / 2 + 16);
    // Single final Fixed-Huffman block (BFINAL=1, BTYPE=01)
    bw.write_bits(1, 1);
    bw.write_bits(1, 2);

    const HASH_BITS: usize = 14;
    const HASH_SIZE: usize = 1 << HASH_BITS;
    let mut head = vec![usize::MAX; HASH_SIZE];
    let mut prev = vec![usize::MAX; input.len().max(1)];

    let hash3 = |a: u8, b: u8, c: u8| -> usize {
        (((a as usize) << 10) ^ ((b as usize) << 5) ^ (c as usize)) & (HASH_SIZE - 1)
    };

    let mut pos = 0usize;
    while pos < input.len() {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;

        if pos + 3 <= input.len() {
            let h = hash3(input[pos], input[pos + 1], input[pos + 2]);
            let mut candidate = head[h];
            prev[pos] = candidate;
            head[h] = pos;

            let max_len = (input.len() - pos).min(258);
            let mut chain = 0usize;
            while candidate != usize::MAX && pos - candidate <= 32768 && chain < 32 {
                let mut l = 0usize;
                while l < max_len && input[candidate + l] == input[pos + l] {
                    l += 1;
                }
                if l >= 3 && l > best_len {
                    best_len = l;
                    best_dist = pos - candidate;
                    if best_len == max_len {
                        break;
                    }
                }
                candidate = prev[candidate];
                chain += 1;
            }
        }

        if best_len >= 3 {
            encode_length(&mut bw, best_len);
            encode_distance(&mut bw, best_dist);
            for step in 1..best_len {
                let p = pos + step;
                if p + 3 <= input.len() {
                    let h = hash3(input[p], input[p + 1], input[p + 2]);
                    prev[p] = head[h];
                    head[h] = p;
                }
            }
            pos += best_len;
        } else {
            encode_fixed_lit(&mut bw, input[pos] as usize);
            pos += 1;
        }
    }

    encode_fixed_lit(&mut bw, 256); // End-of-block
    bw.finish()
}

pub fn zlib_compress(input: &[u8]) -> Vec<u8> {
    let raw = deflate_compress(input);
    let mut out = Vec::with_capacity(raw.len() + 6);
    out.push(0x78); // CMF: Deflate, 32K window
    out.push(0x9C); // FLG: Default compression, (0x78*256 + 0x9C) % 31 == 0
    out.extend_from_slice(&raw);
    out.extend_from_slice(&adler32(input).to_be_bytes());
    out
}
