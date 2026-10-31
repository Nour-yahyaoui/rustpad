//! Pure-Rust ITU-T.81 JPEG Decoder (`SOF0` Baseline, `SOF1` Extended, `SOF2` Progressive,
//! Grayscale / YCbCr / RGB / CMYK, `4:4:4` / `4:2:2` / `4:2:0`, Restart Markers, 2D IDCT)
//! + Universal `decode_any_image_rgba` Dispatcher (`PNG` / `JPEG` / `BMP` / `DIB`).

use crate::png_codec::{decode_bmp_rgba, decode_dib_rgba, decode_png_rgba, PNG_MAGIC};

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Detect canonical file extension (`"png"`, `"jpeg"`, `"bmp"`, `"gif"`) from magic header bytes.
pub fn detect_image_extension(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 8 && bytes[..8] == PNG_MAGIC {
        "png"
    } else if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        "jpeg"
    } else if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        "bmp"
    } else if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        "gif"
    } else {
        "png"
    }
}

/// Fast 64-bit FNV-1a content hash so decoded RGBA caches stay valid even when blocks move.
pub fn hash_image_bytes(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    h ^= bytes.len() as u64;
    h = h.wrapping_mul(0x0100_0000_01b3);
    // Include head + tail + up to 4096 strided interior samples to prevent same-length screenshot collisions
    for &b in bytes.iter().take(96) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    if bytes.len() > 96 {
        for &b in &bytes[bytes.len() - 96..] {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    let step = (bytes.len() / 4096).max(1);
    let mut i = 0usize;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += step;
    }
    h
}

/// Decodes any supported image (`PNG`, `JPEG`, `BMP`, `DIB`) and immediately box-downsamples
/// oversized Retina/4K/stitched screenshots so neither edge exceeds `max_dim` (`>= 128`).
/// Returns `(orig_w, orig_h, thumb_w, thumb_h, thumb_rgba)` so large 100+ screenshot `.docx`
/// files render crisply on screen while using 8–16x less RAM.
pub fn decode_any_image_thumbnail_rgba(
    bytes: &[u8],
    max_dim: u32,
) -> Result<(u32, u32, u32, u32, Vec<u8>), String> {
    let (ow, oh, rgba) = decode_any_image_rgba(bytes)?;
    let cap = max_dim.max(128);
    if ow <= cap && oh <= cap {
        return Ok((ow, oh, ow, oh, rgba));
    }
    let scale = (cap as f64 / ow.max(oh) as f64).clamp(0.01, 1.0);
    let tw = ((ow as f64 * scale).round() as u32).max(1);
    let th = ((oh as f64 * scale).round() as u32).max(1);

    let mut out = vec![0u8; (tw as usize) * (th as usize) * 4];
    let ow_u = ow as usize;
    let oh_u = oh as usize;
    let tw_u = tw as usize;
    let th_u = th as usize;

    for dy in 0..th_u {
        let sy0 = (dy * oh_u) / th_u;
        let sy1 = (((dy + 1) * oh_u) / th_u).max(sy0 + 1).min(oh_u);
        for dx in 0..tw_u {
            let sx0 = (dx * ow_u) / tw_u;
            let sx1 = (((dx + 1) * ow_u) / tw_u).max(sx0 + 1).min(ow_u);
            let mut r_acc = 0u32;
            let mut g_acc = 0u32;
            let mut b_acc = 0u32;
            let mut a_acc = 0u32;
            let mut cnt = 0u32;
            // Stride large sample boxes so downsampling an 8K image stays < 2 ms
            let y_step = ((sy1 - sy0) / 4).max(1);
            let x_step = ((sx1 - sx0) / 4).max(1);
            let mut sy = sy0;
            while sy < sy1 {
                let row_off = sy * ow_u * 4;
                let mut sx = sx0;
                while sx < sx1 {
                    let o = row_off + sx * 4;
                    r_acc += rgba[o] as u32;
                    g_acc += rgba[o + 1] as u32;
                    b_acc += rgba[o + 2] as u32;
                    a_acc += rgba[o + 3] as u32;
                    cnt += 1;
                    sx += x_step;
                }
                sy += y_step;
            }
            let c = cnt.max(1);
            let dst_o = (dy * tw_u + dx) * 4;
            out[dst_o] = (r_acc / c) as u8;
            out[dst_o + 1] = (g_acc / c) as u8;
            out[dst_o + 2] = (b_acc / c) as u8;
            out[dst_o + 3] = (a_acc / c) as u8;
        }
    }

    Ok((ow, oh, tw, th, out))
}

/// Universal zero-dependency image decoder supporting PNG (all color types/bit depths/Adam7),
/// JPEG (Baseline, Extended & Progressive), Windows BMP, and headerless clipboard DIB.
pub fn decode_any_image_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if bytes.len() >= 8 && bytes[..8] == PNG_MAGIC {
        return decode_png_rgba(bytes);
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return decode_jpeg_rgba(bytes);
    }
    if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        return decode_bmp_rgba(bytes);
    }
    if bytes.len() >= 40 {
        let hdr = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if matches!(hdr, 40 | 52 | 56 | 108 | 124) {
            return decode_dib_rgba(bytes);
        }
    }
    Err("Unsupported or unrecognized image format (expected PNG, JPEG, or BMP)".into())
}

#[derive(Clone)]
struct HuffTable {
    min_code: [i32; 17],
    max_code: [i32; 17],
    val_ptr: [usize; 17],
    vals: Vec<u8>,
}

impl Default for HuffTable {
    fn default() -> Self {
        Self {
            min_code: [0; 17],
            max_code: [-1; 17],
            val_ptr: [0; 17],
            vals: Vec::new(),
        }
    }
}

impl HuffTable {
    fn from_counts_and_vals(counts: &[u8; 16], vals: Vec<u8>) -> Self {
        let mut tbl = Self {
            min_code: [0; 17],
            max_code: [-1; 17],
            val_ptr: [0; 17],
            vals,
        };
        let mut code = 0i32;
        let mut k = 0usize;
        for bits in 1..=16 {
            let cnt = counts[bits - 1] as usize;
            if cnt == 0 {
                tbl.max_code[bits] = -1;
            } else {
                tbl.val_ptr[bits] = k;
                tbl.min_code[bits] = code;
                code += (cnt as i32) - 1;
                tbl.max_code[bits] = code;
                k += cnt;
                code += 1;
            }
            code <<= 1;
        }
        tbl
    }
}

#[derive(Clone)]
struct Component {
    id: u8,
    h_samp: usize,
    v_samp: usize,
    tq: usize,
    blocks_w: usize,
    blocks_h: usize,
    stride_blocks: usize,
    coeffs: Vec<[i32; 64]>,
}

struct ScanBitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u32,
    bit_count: u8,
    hit_marker: bool,
}

impl<'a> ScanBitReader<'a> {
    fn new(data: &'a [u8], start: usize) -> Self {
        Self {
            data,
            pos: start,
            bit_buf: 0,
            bit_count: 0,
            hit_marker: false,
        }
    }

    fn next_entropy_byte(&mut self) -> u8 {
        if self.hit_marker || self.pos >= self.data.len() {
            return 0;
        }
        let b = self.data[self.pos];
        self.pos += 1;
        if b == 0xFF {
            while self.pos < self.data.len() && self.data[self.pos] == 0xFF {
                self.pos += 1;
            }
            if self.pos >= self.data.len() {
                self.hit_marker = true;
                return 0;
            }
            let next = self.data[self.pos];
            if next == 0x00 {
                self.pos += 1;
                return 0xFF;
            }
            // Hit a real marker (`RSTn`, `SOS`, `DQT`, `EOI`, etc.) — back up to `0xFF`
            self.pos -= 1;
            self.hit_marker = true;
            return 0;
        }
        b
    }

    #[inline]
    fn read_bit(&mut self) -> u32 {
        if self.bit_count == 0 {
            let b = self.next_entropy_byte();
            self.bit_buf = b as u32;
            self.bit_count = 8;
        }
        self.bit_count -= 1;
        (self.bit_buf >> self.bit_count) & 1
    }

    #[inline]
    fn read_bits(&mut self, n: u8) -> i32 {
        let mut val = 0u32;
        for _ in 0..n {
            val = (val << 1) | self.read_bit();
        }
        val as i32
    }

    #[inline]
    fn receive_extend(&mut self, s: u8) -> i32 {
        if s == 0 {
            return 0;
        }
        let v = self.read_bits(s);
        let vt = 1i32 << (s - 1);
        if v < vt {
            v - ((1i32 << s) - 1)
        } else {
            v
        }
    }

    fn decode_huff(&mut self, tbl: &HuffTable) -> Result<u8, String> {
        let mut code = 0i32;
        for bits in 1..=16 {
            code = (code << 1) | (self.read_bit() as i32);
            if code <= tbl.max_code[bits] {
                let idx = tbl.val_ptr[bits] + ((code - tbl.min_code[bits]) as usize);
                return tbl
                    .vals
                    .get(idx)
                    .copied()
                    .ok_or_else(|| "Malformed JPEG Huffman symbol index".to_string());
            }
        }
        Ok(0)
    }

    fn consume_restart_marker(&mut self) {
        self.bit_buf = 0;
        self.bit_count = 0;
        self.hit_marker = false;
        while self.pos + 1 < self.data.len() && self.data[self.pos] == 0xFF {
            let m = self.data[self.pos + 1];
            if m == 0xFF {
                self.pos += 1;
                continue;
            }
            if (0xD0..=0xD7).contains(&m) || m == 0x00 {
                self.pos += 2;
            }
            break;
        }
    }
}

/// Decodes a Baseline (`SOF0`), Extended (`SOF1`), or Progressive (`SOF2`) JPEG image to RGBA.
pub fn decode_jpeg_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err("Invalid JPEG SOI signature".into());
    }

    let mut qtables = [[16i32; 64]; 4];
    let mut dc_huff: [Option<HuffTable>; 4] = [None, None, None, None];
    let mut ac_huff: [Option<HuffTable>; 4] = [None, None, None, None];
    let mut width = 0usize;
    let mut height = 0usize;
    let mut progressive = false;
    let mut restart_interval = 0usize;
    let mut comps: Vec<Component> = Vec::new();
    let mut max_h = 1usize;
    let mut max_v = 1usize;
    let mut adobe_transform: Option<u8> = None;

    let mut pos = 2usize;
    while pos + 1 < bytes.len() {
        // Advance to next `0xFF` marker
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;

        if marker == 0xD9 {
            // EOI
            break;
        }
        if marker == 0x00 || marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            continue;
        }
        if pos + 2 > bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return Err("Truncated JPEG marker segment".into());
        }
        let seg = &bytes[pos + 2..pos + seg_len];
        pos += seg_len;

        match marker {
            0xDB => {
                // DQT — Define Quantization Table(s)
                let mut sp = 0usize;
                while sp < seg.len() {
                    let pq = (seg[sp] >> 4) as usize;
                    let tq = (seg[sp] & 0x0F) as usize;
                    sp += 1;
                    if tq >= 4 {
                        return Err("Invalid JPEG DQT table id".into());
                    }
                    if pq == 0 {
                        if sp + 64 > seg.len() {
                            return Err("Truncated 8-bit DQT".into());
                        }
                        for i in 0..64 {
                            qtables[tq][ZIGZAG[i]] = seg[sp + i] as i32;
                        }
                        sp += 64;
                    } else {
                        if sp + 128 > seg.len() {
                            return Err("Truncated 16-bit DQT".into());
                        }
                        for i in 0..64 {
                            let v = u16::from_be_bytes([seg[sp + i * 2], seg[sp + i * 2 + 1]]);
                            qtables[tq][ZIGZAG[i]] = v as i32;
                        }
                        sp += 128;
                    }
                }
            }
            0xC4 => {
                // DHT — Define Huffman Table(s)
                let mut sp = 0usize;
                while sp + 17 <= seg.len() {
                    let tc = (seg[sp] >> 4) as usize;
                    let th = (seg[sp] & 0x0F) as usize;
                    sp += 1;
                    if th >= 4 || tc > 1 {
                        return Err("Invalid JPEG DHT selector".into());
                    }
                    let mut counts = [0u8; 16];
                    counts.copy_from_slice(&seg[sp..sp + 16]);
                    sp += 16;
                    let total_syms: usize = counts.iter().map(|&c| c as usize).sum();
                    if sp + total_syms > seg.len() {
                        return Err("Truncated JPEG DHT symbols".into());
                    }
                    let vals = seg[sp..sp + total_syms].to_vec();
                    sp += total_syms;
                    let huff = HuffTable::from_counts_and_vals(&counts, vals);
                    if tc == 0 {
                        dc_huff[th] = Some(huff);
                    } else {
                        ac_huff[th] = Some(huff);
                    }
                }
            }
            0xDD => {
                // DRI — Define Restart Interval
                if seg.len() >= 2 {
                    restart_interval = u16::from_be_bytes([seg[0], seg[1]]) as usize;
                }
            }
            0xEE => {
                // APP14 Adobe color transform marker
                if seg.len() >= 12 && &seg[0..5] == b"Adobe" {
                    adobe_transform = Some(seg[11]);
                }
            }
            0xC0 | 0xC1 | 0xC2 => {
                // SOF0 (Baseline), SOF1 (Extended Sequential), SOF2 (Progressive)
                if seg.len() < 6 {
                    return Err("Malformed JPEG SOF header".into());
                }
                let precision = seg[0];
                if precision != 8 {
                    return Err(format!("Unsupported JPEG sample precision {precision}"));
                }
                height = u16::from_be_bytes([seg[1], seg[2]]) as usize;
                width = u16::from_be_bytes([seg[3], seg[4]]) as usize;
                let nf = seg[5] as usize;
                if width == 0 || height == 0 || width > 32_768 || height > 32_768 {
                    return Err("JPEG dimensions out of bounds (1..=32768)".into());
                }
                if !matches!(nf, 1 | 3 | 4) || seg.len() < 6 + nf * 3 {
                    return Err("Unsupported JPEG component count".into());
                }
                if width.saturating_mul(height) > 64 * 1024 * 1024 {
                    return Err("JPEG exceeds 64MP safety limit".into());
                }
                progressive = marker == 0xC2;
                comps.clear();
                max_h = 1;
                max_v = 1;
                let mut raw_comps = Vec::with_capacity(nf);
                for i in 0..nf {
                    let o = 6 + i * 3;
                    let cid = seg[o];
                    let h_samp = ((seg[o + 1] >> 4) & 0x0F) as usize;
                    let v_samp = (seg[o + 1] & 0x0F) as usize;
                    let tq = seg[o + 2] as usize;
                    if h_samp == 0 || h_samp > 4 || v_samp == 0 || v_samp > 4 || tq >= 4 {
                        return Err("Invalid JPEG component sampling factors".into());
                    }
                    max_h = max_h.max(h_samp);
                    max_v = max_v.max(v_samp);
                    raw_comps.push((cid, h_samp, v_samp, tq));
                }
                let mcu_cols = width.div_ceil(max_h * 8);
                let mcu_rows = height.div_ceil(max_v * 8);
                for (cid, hs, vs, tq) in raw_comps {
                    let comp_px_w = (width * hs).div_ceil(max_h);
                    let comp_px_h = (height * vs).div_ceil(max_v);
                    let blocks_w = comp_px_w.div_ceil(8);
                    let blocks_h = comp_px_h.div_ceil(8);
                    let stride_blocks = mcu_cols * hs;
                    let alloc_rows = mcu_rows * vs;
                    comps.push(Component {
                        id: cid,
                        h_samp: hs,
                        v_samp: vs,
                        tq,
                        blocks_w,
                        blocks_h,
                        stride_blocks,
                        coeffs: vec![[0i32; 64]; stride_blocks * alloc_rows],
                    });
                }
            }
            0xDA => {
                // SOS — Start of Scan
                if comps.is_empty() || seg.is_empty() {
                    return Err("JPEG SOS encountered before SOF".into());
                }
                let ns = seg[0] as usize;
                if ns == 0 || ns > 4 || seg.len() < 1 + ns * 2 + 3 {
                    return Err("Malformed JPEG SOS header".into());
                }
                let mut scan_comps: Vec<(usize, usize, usize)> = Vec::with_capacity(ns);
                for i in 0..ns {
                    let cs = seg[1 + i * 2];
                    let td = ((seg[1 + i * 2 + 1] >> 4) & 0x0F) as usize;
                    let ta = (seg[1 + i * 2 + 1] & 0x0F) as usize;
                    let c_idx = comps
                        .iter()
                        .position(|c| c.id == cs)
                        .ok_or_else(|| "Unknown component id in JPEG SOS".to_string())?;
                    if td >= 4 || ta >= 4 {
                        return Err("Invalid Huffman selector in SOS".into());
                    }
                    scan_comps.push((c_idx, td, ta));
                }
                let spec_pos = 1 + ns * 2;
                let ss = seg[spec_pos] as usize;
                let se = seg[spec_pos + 1] as usize;
                let ah = (seg[spec_pos + 2] >> 4) & 0x0F;
                let al = seg[spec_pos + 2] & 0x0F;

                let mut reader = ScanBitReader::new(bytes, pos);
                let mut dc_pred = [0i32; 4];
                let mut eob_run = 0usize;
                let mut mcu_counter = 0usize;

                if ns == 1 {
                    // Non-interleaved scan (ITU-T.81 §A.2.2): iterate exact `blocks_w * blocks_h`
                    let (c_idx, td, ta) = scan_comps[0];
                    let bw = comps[c_idx].blocks_w;
                    let bh = comps[c_idx].blocks_h;
                    let stride = comps[c_idx].stride_blocks;
                    for by in 0..bh {
                        for bx in 0..bw {
                            if restart_interval > 0 && mcu_counter > 0 && mcu_counter % restart_interval == 0 {
                                reader.consume_restart_marker();
                                dc_pred = [0; 4];
                                eob_run = 0;
                            }
                            let blk = &mut comps[c_idx].coeffs[by * stride + bx];
                            decode_scan_block(
                                &mut reader,
                                blk,
                                &mut dc_pred[c_idx],
                                &mut eob_run,
                                dc_huff[td].as_ref(),
                                ac_huff[ta].as_ref(),
                                progressive,
                                ss,
                                se,
                                ah,
                                al,
                            )?;
                            mcu_counter += 1;
                        }
                    }
                } else {
                    // Interleaved multi-component MCU scan
                    let mcu_cols = width.div_ceil(max_h * 8);
                    let mcu_rows = height.div_ceil(max_v * 8);
                    for my in 0..mcu_rows {
                        for mx in 0..mcu_cols {
                            if restart_interval > 0 && mcu_counter > 0 && mcu_counter % restart_interval == 0 {
                                reader.consume_restart_marker();
                                dc_pred = [0; 4];
                                eob_run = 0;
                            }
                            for &(c_idx, td, ta) in &scan_comps {
                                let hs = comps[c_idx].h_samp;
                                let vs = comps[c_idx].v_samp;
                                let stride = comps[c_idx].stride_blocks;
                                for vy in 0..vs {
                                    for hx in 0..hs {
                                        let bx = mx * hs + hx;
                                        let by = my * vs + vy;
                                        let blk = &mut comps[c_idx].coeffs[by * stride + bx];
                                        decode_scan_block(
                                            &mut reader,
                                            blk,
                                            &mut dc_pred[c_idx],
                                            &mut eob_run,
                                            dc_huff[td].as_ref(),
                                            ac_huff[ta].as_ref(),
                                            progressive,
                                            ss,
                                            se,
                                            ah,
                                            al,
                                        )?;
                                    }
                                }
                            }
                            mcu_counter += 1;
                        }
                    }
                }

                pos = reader.pos;
            }
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                return Err(format!("Unsupported lossless/arithmetic JPEG SOF 0x{marker:02X}"));
            }
            _ => {}
        }
    }

    if comps.is_empty() || width == 0 || height == 0 {
        return Err("JPEG stream contained no decodable image frame".into());
    }

    // Dequantize & run 2D 8×8 IDCT per component into planar u8 buffers
    let mut planes: Vec<(usize, usize, usize, usize, Vec<u8>)> = Vec::with_capacity(comps.len());
    for comp in &comps {
        let pw = comp.stride_blocks * 8;
        let alloc_rows = comp.coeffs.len() / comp.stride_blocks.max(1);
        let ph = alloc_rows * 8;
        let mut pixels = vec![128u8; pw * ph];
        let qt = &qtables[comp.tq];
        let mut spatial = [0u8; 64];

        for by in 0..alloc_rows {
            for bx in 0..comp.stride_blocks {
                let coeffs = &comp.coeffs[by * comp.stride_blocks + bx];
                idct_8x8(coeffs, qt, &mut spatial);
                for py in 0..8 {
                    let dst_off = (by * 8 + py) * pw + bx * 8;
                    pixels[dst_off..dst_off + 8].copy_from_slice(&spatial[py * 8..py * 8 + 8]);
                }
            }
        }
        planes.push((comp.h_samp, comp.v_samp, pw, ph, pixels));
    }

    // Reconstruct final RGBA buffer with smooth chroma upsampling
    let mut rgba = vec![255u8; width * height * 4];
    let is_rgb = comps.len() == 3
        && ((comps[0].id == b'R' && comps[1].id == b'G' && comps[2].id == b'B')
            || adobe_transform == Some(0));

    for y in 0..height {
        for x in 0..width {
            let out_o = (y * width + x) * 4;
            match planes.len() {
                1 => {
                    let g = sample_plane(&planes[0], x, y, max_h, max_v);
                    rgba[out_o] = g;
                    rgba[out_o + 1] = g;
                    rgba[out_o + 2] = g;
                }
                3 => {
                    let c0 = sample_plane(&planes[0], x, y, max_h, max_v);
                    let c1 = sample_plane(&planes[1], x, y, max_h, max_v);
                    let c2 = sample_plane(&planes[2], x, y, max_h, max_v);
                    let (r, g, b) = if is_rgb {
                        (c0, c1, c2)
                    } else {
                        ycbcr_to_rgb(c0, c1, c2)
                    };
                    rgba[out_o] = r;
                    rgba[out_o + 1] = g;
                    rgba[out_o + 2] = b;
                }
                4 => {
                    let c0 = sample_plane(&planes[0], x, y, max_h, max_v);
                    let c1 = sample_plane(&planes[1], x, y, max_h, max_v);
                    let c2 = sample_plane(&planes[2], x, y, max_h, max_v);
                    let k = sample_plane(&planes[3], x, y, max_h, max_v) as u16;
                    let (c, m, y_ch) = if adobe_transform == Some(2) {
                        let (r, g, b) = ycbcr_to_rgb(c0, c1, c2);
                        (255 - r, 255 - g, 255 - b)
                    } else {
                        (c0, c1, c2)
                    };
                    rgba[out_o] = ((c as u16 * k) / 255) as u8;
                    rgba[out_o + 1] = ((m as u16 * k) / 255) as u8;
                    rgba[out_o + 2] = ((y_ch as u16 * k) / 255) as u8;
                }
                _ => {}
            }
        }
    }

    Ok((width as u32, height as u32, rgba))
}

#[allow(clippy::too_many_arguments)]
fn decode_scan_block(
    reader: &mut ScanBitReader<'_>,
    blk: &mut [i32; 64],
    dc_pred: &mut i32,
    eob_run: &mut usize,
    dc_tbl: Option<&HuffTable>,
    ac_tbl: Option<&HuffTable>,
    progressive: bool,
    ss: usize,
    se: usize,
    ah: u8,
    al: u8,
) -> Result<(), String> {
    if !progressive {
        let dt = dc_tbl.ok_or_else(|| "Missing JPEG DC Huffman table".to_string())?;
        let at = ac_tbl.ok_or_else(|| "Missing JPEG AC Huffman table".to_string())?;
        let s = reader.decode_huff(dt)? & 0x0F;
        let diff = reader.receive_extend(s);
        *dc_pred = dc_pred.wrapping_add(diff);
        blk[0] = *dc_pred;

        let mut k = 1usize;
        while k < 64 {
            let rs = reader.decode_huff(at)?;
            let r = (rs >> 4) as usize;
            let s = rs & 0x0F;
            if s == 0 {
                if r == 15 {
                    k += 16;
                    continue;
                }
                break;
            }
            k += r;
            if k >= 64 {
                break;
            }
            blk[ZIGZAG[k]] = reader.receive_extend(s);
            k += 1;
        }
        return Ok(());
    }

    // Progressive mode (`SOF2`)
    if ss == 0 && se == 0 {
        if ah == 0 {
            let dt = dc_tbl.ok_or_else(|| "Missing progressive DC Huffman table".to_string())?;
            let s = reader.decode_huff(dt)? & 0x0F;
            let diff = reader.receive_extend(s);
            *dc_pred = dc_pred.wrapping_add(diff);
            blk[0] = (*dc_pred) << al;
        } else if reader.read_bit() == 1 {
            blk[0] |= 1i32 << al;
        }
        return Ok(());
    }

    let at = ac_tbl.ok_or_else(|| "Missing progressive AC Huffman table".to_string())?;
    let se_clamped = se.min(63);
    if ah == 0 {
        if *eob_run > 0 {
            *eob_run -= 1;
            return Ok(());
        }
        let mut k = ss.max(1);
        while k <= se_clamped {
            let rs = reader.decode_huff(at)?;
            let r = (rs >> 4) as usize;
            let s = rs & 0x0F;
            if s == 0 {
                if r < 15 {
                    *eob_run = (1usize << r) + (reader.read_bits(r as u8) as usize) - 1;
                    break;
                }
                k += 16;
                continue;
            }
            k += r;
            if k <= se_clamped {
                blk[ZIGZAG[k]] = reader.receive_extend(s) << al;
            }
            k += 1;
        }
    } else {
        // AC successive approximation refinement
        let p1 = 1i32 << al;
        let m1 = (-1i32) << al;
        let mut k = ss.max(1);
        if *eob_run == 0 {
            while k <= se_clamped {
                let rs = reader.decode_huff(at)?;
                let mut r = (rs >> 4) as usize;
                let s = rs & 0x0F;
                let new_val = if s == 0 {
                    if r < 15 {
                        *eob_run = (1usize << r) + (reader.read_bits(r as u8) as usize);
                        break;
                    }
                    0
                } else {
                    if reader.read_bit() == 1 { p1 } else { m1 }
                };

                while k <= se_clamped {
                    let z = ZIGZAG[k];
                    if blk[z] != 0 {
                        if reader.read_bit() == 1 && (blk[z] & p1) == 0 {
                            blk[z] += if blk[z] >= 0 { p1 } else { m1 };
                        }
                    } else if r == 0 {
                        break;
                    } else {
                        r -= 1;
                    }
                    k += 1;
                }
                if new_val != 0 && k <= se_clamped {
                    blk[ZIGZAG[k]] = new_val;
                }
                k += 1;
            }
        }
        if *eob_run > 0 {
            while k <= se_clamped {
                let z = ZIGZAG[k];
                if blk[z] != 0 && reader.read_bit() == 1 && (blk[z] & p1) == 0 {
                    blk[z] += if blk[z] >= 0 { p1 } else { m1 };
                }
                k += 1;
            }
            *eob_run -= 1;
        }
    }

    Ok(())
}

#[inline]
fn sample_plane(
    plane: &(usize, usize, usize, usize, Vec<u8>),
    x: usize,
    y: usize,
    max_h: usize,
    max_v: usize,
) -> u8 {
    let (hs, vs, pw, ph, ref buf) = *plane;
    if hs == max_h && vs == max_v {
        return buf[y.min(ph - 1) * pw + x.min(pw - 1)];
    }
    // Smooth bilinear chroma upsampling
    let fx = ((x as f32 + 0.5) * (hs as f32) / (max_h as f32) - 0.5).max(0.0);
    let fy = ((y as f32 + 0.5) * (vs as f32) / (max_v as f32) - 0.5).max(0.0);
    let x0 = (fx.floor() as usize).min(pw - 1);
    let y0 = (fy.floor() as usize).min(ph - 1);
    let x1 = (x0 + 1).min(pw - 1);
    let y1 = (y0 + 1).min(ph - 1);
    let tx = fx - (x0 as f32);
    let ty = fy - (y0 as f32);

    let p00 = buf[y0 * pw + x0] as f32;
    let p10 = buf[y0 * pw + x1] as f32;
    let p01 = buf[y1 * pw + x0] as f32;
    let p11 = buf[y1 * pw + x1] as f32;
    let top = p00 + (p10 - p00) * tx;
    let bot = p01 + (p11 - p01) * tx;
    (top + (bot - top) * ty).round().clamp(0.0, 255.0) as u8
}

#[inline]
fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    let yf = y as f32;
    let cbf = (cb as f32) - 128.0;
    let crf = (cr as f32) - 128.0;
    let r = (yf + 1.402 * crf).round().clamp(0.0, 255.0) as u8;
    let g = (yf - 0.344136 * cbf - 0.714136 * crf)
        .round()
        .clamp(0.0, 255.0) as u8;
    let b = (yf + 1.772 * cbf).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// Fast separable 2D 8×8 inverse DCT (`ITU-T.81` §A.3.3) using exact cosine constants.
fn idct_8x8(coeffs: &[i32; 64], qt: &[i32; 64], out: &mut [u8; 64]) {
    const IDCT_BASIS: [[f32; 8]; 8] = [
        [0.35355338, 0.35355338, 0.35355338, 0.35355338, 0.35355338, 0.35355338, 0.35355338, 0.35355338],
        [0.49039263, 0.4157348,  0.27778512, 0.09754516, -0.09754516, -0.27778512, -0.4157348,  -0.49039263],
        [0.46193975, 0.19134171, -0.19134171, -0.46193975, -0.46193975, -0.19134171, 0.19134171, 0.46193975],
        [0.4157348,  -0.09754516, -0.49039263, -0.27778512, 0.27778512, 0.49039263, 0.09754516, -0.4157348],
        [0.35355338, -0.35355338, -0.35355338, 0.35355338, 0.35355338, -0.35355338, -0.35355338, 0.35355338],
        [0.27778512, -0.49039263, 0.09754516, 0.4157348,  -0.4157348,  -0.09754516, 0.49039263, -0.27778512],
        [0.19134171, -0.46193975, 0.46193975, -0.19134171, -0.19134171, 0.46193975, -0.46193975, 0.19134171],
        [0.09754516, -0.27778512, 0.4157348,  -0.49039263, 0.49039263, -0.4157348,  0.27778512, -0.09754516],
    ];

    // Fast path for flat DC-only blocks (very common in smooth document diagrams & photos!)
    if coeffs[1..].iter().all(|&c| c == 0) {
        let dc = ((coeffs[0] * qt[0]) as f32) * 0.125 + 128.0;
        out.fill(dc.round().clamp(0.0, 255.0) as u8);
        return;
    }

    let mut tmp = [0.0f32; 64];
    for row in 0..8 {
        let ro = row * 8;
        for x in 0..8 {
            let mut sum = 0.0f32;
            for u in 0..8 {
                let c = coeffs[ro + u];
                if c != 0 {
                    sum += ((c * qt[ro + u]) as f32) * IDCT_BASIS[u][x];
                }
            }
            tmp[ro + x] = sum;
        }
    }

    for col in 0..8 {
        for y in 0..8 {
            let mut sum = 0.0f32;
            for v in 0..8 {
                sum += tmp[v * 8 + col] * IDCT_BASIS[v][y];
            }
            out[y * 8 + col] = (sum + 128.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_rust_jpeg_baseline_decode_and_universal_dispatch() {
        // Construct a minimal standards-valid 8x8 grayscale Baseline JPEG (`SOF0`)
        let mut jpg: Vec<u8> = Vec::new();
        jpg.extend_from_slice(&[0xFF, 0xD8]); // SOI

        // DQT (table 0, all 1s)
        jpg.extend_from_slice(&[0xFF, 0xDB, 0x00, 67, 0x00]);
        jpg.extend_from_slice(&[1u8; 64]);

        // SOF0: 8-bit, 8x8, 1 component (id=1, 1x1 sampling, tq=0)
        jpg.extend_from_slice(&[
            0xFF, 0xC0, 0x00, 11, 8, 0, 8, 0, 8, 1, 1, 0x11, 0,
        ]);

        // DHT DC table 0: 1 symbol of length 1 bit -> symbol `0` (DC diff = 0 -> mid-gray 128!)
        let mut dc_counts = [0u8; 16];
        dc_counts[0] = 1;
        jpg.extend_from_slice(&[0xFF, 0xC4, 0x00, 20, 0x00]);
        jpg.extend_from_slice(&dc_counts);
        jpg.push(0x00);

        // DHT AC table 0: 1 symbol of length 1 bit -> symbol `0x00` (EOB)
        let mut ac_counts = [0u8; 16];
        ac_counts[0] = 1;
        jpg.extend_from_slice(&[0xFF, 0xC4, 0x00, 20, 0x10]);
        jpg.extend_from_slice(&ac_counts);
        jpg.push(0x00);

        // SOS: 1 component (id=1, td=0, ta=0), Ss=0, Se=63, Ah/Al=0
        jpg.extend_from_slice(&[0xFF, 0xDA, 0x00, 8, 1, 1, 0x00, 0, 63, 0]);
        // Entropy bits: `0` (DC diff=0) + `0` (AC EOB) + `111111` pad -> `0b0011_1111` = `0x3F`
        jpg.push(0x3F);
        jpg.extend_from_slice(&[0xFF, 0xD9]); // EOI

        assert_eq!(detect_image_extension(&jpg), "jpeg");
        let (w, h, rgba) = decode_any_image_rgba(&jpg).expect("decode baseline JPEG");
        assert_eq!((w, h), (8, 8));
        assert_eq!(rgba.len(), 8 * 8 * 4);
        assert_eq!(&rgba[0..4], &[128, 128, 128, 255]);

        // Verify `decode_any_image_thumbnail_rgba` downsamples oversized buffers cleanly
        let raw_png = crate::png_codec::encode_rgba_png(320, 160, &vec![200u8; 320 * 160 * 4]);
        let (ow, oh, tw, th, thumb) =
            decode_any_image_thumbnail_rgba(&raw_png, 160).expect("thumbnail downsample");
        assert_eq!((ow, oh), (320, 160));
        assert_eq!((tw, th), (160, 80));
        assert_eq!(thumb.len(), 160 * 80 * 4);
    }
}
