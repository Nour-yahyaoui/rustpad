//! Pure-Rust TrueType (`.ttf`) SFNT Parser & 4×4 Supersampled Scanline Anti-Aliased Glyph Atlas.

use doc_model::FontFamily;
use std::collections::HashMap;

const SANS_REGULAR_TTF: &[u8] = include_bytes!("../../../assets/fonts/Sans-Regular.ttf");
const SANS_BOLD_TTF: &[u8] = include_bytes!("../../../assets/fonts/Sans-Bold.ttf");
const SERIF_REGULAR_TTF: &[u8] = include_bytes!("../../../assets/fonts/Serif-Regular.ttf");
const MONO_REGULAR_TTF: &[u8] = include_bytes!("../../../assets/fonts/Mono-Regular.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSlot {
    SansRegular,
    SansBold,
    SerifRegular,
    MonoRegular,
}

impl FontSlot {
    pub fn select(family: FontFamily, bold: bool) -> Self {
        match (family, bold) {
            (FontFamily::Sans, false) => Self::SansRegular,
            (FontFamily::Sans, true) => Self::SansBold,
            (FontFamily::Serif, _) => Self::SerifRegular,
            (FontFamily::Mono, _) => Self::MonoRegular,
        }
    }
}

#[derive(Clone)]
pub struct RasterGlyph {
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    /// Offset from baseline to top of bitmap (negative means above baseline).
    pub offset_y: i32,
    pub advance_px: f32,
    pub alpha: Vec<u8>,
}

struct TtfFace {
    data: &'static [u8],
    units_per_em: f32,
    ascent: i16,
    descent: i16,
    index_to_loc_long: bool,
    num_h_metrics: u16,
    num_glyphs: u16,
    cmap_off: usize,
    hmtx_off: usize,
    loca_off: usize,
    glyf_off: usize,
    ascii_advances: [u16; 128],
}

impl TtfFace {
    fn parse(data: &'static [u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        let num_tables = read_u16(data, 4)? as usize;
        let mut head_off = 0usize;
        let mut hhea_off = 0usize;
        let mut maxp_off = 0usize;
        let mut cmap_off = 0usize;
        let mut hmtx_off = 0usize;
        let mut loca_off = 0usize;
        let mut glyf_off = 0usize;

        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if rec + 16 > data.len() {
                break;
            }
            let tag = &data[rec..rec + 4];
            let off = read_u32(data, rec + 8)? as usize;
            match tag {
                b"head" => head_off = off,
                b"hhea" => hhea_off = off,
                b"maxp" => maxp_off = off,
                b"cmap" => cmap_off = off,
                b"hmtx" => hmtx_off = off,
                b"loca" => loca_off = off,
                b"glyf" => glyf_off = off,
                _ => {}
            }
        }
        if head_off == 0 || hhea_off == 0 || cmap_off == 0 || hmtx_off == 0 || loca_off == 0 || glyf_off == 0 {
            return None;
        }

        let units_per_em = read_u16(data, head_off + 18)?.max(64) as f32;
        let index_to_loc_long = read_i16(data, head_off + 50)? != 0;
        let ascent = read_i16(data, hhea_off + 4)?;
        let descent = read_i16(data, hhea_off + 6)?;
        let num_h_metrics = read_u16(data, hhea_off + 34)?;
        let num_glyphs = if maxp_off > 0 {
            read_u16(data, maxp_off + 4).unwrap_or(256)
        } else {
            256
        };

        let mut face = Self {
            data,
            units_per_em,
            ascent,
            descent,
            index_to_loc_long,
            num_h_metrics,
            num_glyphs,
            cmap_off,
            hmtx_off,
            loca_off,
            glyf_off,
            ascii_advances: [0u16; 128],
        };
        for b in 0..128u8 {
            let gid = face.glyph_index(b as char);
            face.ascii_advances[b as usize] = face.advance_units(gid);
        }
        Some(face)
    }

    fn glyph_index(&self, ch: char) -> u16 {
        let cp = ch as u32;
        let d = self.data;
        let base = self.cmap_off;
        let Some(num_sub) = read_u16(d, base + 2) else {
            return 0;
        };
        let mut fmt12_off = None;
        let mut fmt4_off = None;

        for i in 0..num_sub as usize {
            let rec = base + 4 + i * 8;
            let Some(pid) = read_u16(d, rec) else { break };
            let Some(eid) = read_u16(d, rec + 2) else { break };
            let Some(rel) = read_u32(d, rec + 4) else { break };
            let sub = base + rel as usize;
            let Some(fmt) = read_u16(d, sub) else { continue };
            if (pid == 3 || pid == 0) && fmt == 12 {
                fmt12_off = Some(sub);
            } else if ((pid == 3 && (eid == 1 || eid == 0)) || pid == 0) && fmt == 4 {
                fmt4_off = Some(sub);
            }
        }

        if let Some(sub) = fmt12_off {
            if let Some(n_groups) = read_u32(d, sub + 12) {
                for g in 0..n_groups as usize {
                    let goff = sub + 16 + g * 12;
                    let (Some(start), Some(end), Some(gid0)) = (
                        read_u32(d, goff),
                        read_u32(d, goff + 4),
                        read_u32(d, goff + 8),
                    ) else {
                        break;
                    };
                    if cp >= start && cp <= end {
                        return (gid0 + (cp - start)) as u16;
                    }
                }
            }
        }

        if cp <= 0xFFFF {
            let cp16 = cp as u16;
            if let Some(sub) = fmt4_off {
                if let Some(seg_x2) = read_u16(d, sub + 6) {
                    let seg_count = (seg_x2 / 2) as usize;
                    let end_codes = sub + 14;
                    let start_codes = end_codes + seg_count * 2 + 2;
                    let id_deltas = start_codes + seg_count * 2;
                    let id_range_offsets = id_deltas + seg_count * 2;
                    for s in 0..seg_count {
                        let Some(end_c) = read_u16(d, end_codes + s * 2) else {
                            break;
                        };
                        if cp16 <= end_c {
                            let Some(start_c) = read_u16(d, start_codes + s * 2) else {
                                break;
                            };
                            if cp16 < start_c {
                                return 0;
                            }
                            let delta = read_i16(d, id_deltas + s * 2).unwrap_or(0);
                            let ro_pos = id_range_offsets + s * 2;
                            let ro = read_u16(d, ro_pos).unwrap_or(0);
                            if ro == 0 {
                                return (cp16 as i32 + delta as i32) as u16;
                            } else {
                                let glyph_addr =
                                    ro_pos + ro as usize + ((cp16 - start_c) as usize) * 2;
                                let gid = read_u16(d, glyph_addr).unwrap_or(0);
                                if gid == 0 {
                                    return 0;
                                }
                                return (gid as i32 + delta as i32) as u16;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    fn advance_units(&self, gid: u16) -> u16 {
        let idx = gid.min(self.num_h_metrics.saturating_sub(1)) as usize;
        read_u16(self.data, self.hmtx_off + idx * 4).unwrap_or(self.units_per_em as u16 / 2)
    }

    fn glyf_range(&self, gid: u16) -> Option<(usize, usize)> {
        if gid >= self.num_glyphs {
            return None;
        }
        let i = gid as usize;
        let (a, b) = if self.index_to_loc_long {
            (
                read_u32(self.data, self.loca_off + i * 4)? as usize,
                read_u32(self.data, self.loca_off + (i + 1) * 4)? as usize,
            )
        } else {
            (
                read_u16(self.data, self.loca_off + i * 2)? as usize * 2,
                read_u16(self.data, self.loca_off + (i + 1) * 2)? as usize * 2,
            )
        };
        if a >= b {
            None
        } else {
            Some((self.glyf_off + a, self.glyf_off + b))
        }
    }

    fn collect_segments(&self, gid: u16, depth: u8, edges: &mut Vec<(f32, f32, f32, f32)>) {
        if depth > 5 {
            return;
        }
        let Some((start, end)) = self.glyf_range(gid) else {
            return;
        };
        if end > self.data.len() || start + 10 > end {
            return;
        }
        let d = self.data;
        let num_contours = read_i16(d, start).unwrap_or(0);

        if num_contours > 0 {
            let nc = num_contours as usize;
            if start + 10 + nc * 2 > end {
                return;
            }
            let mut end_pts = Vec::with_capacity(nc);
            for c in 0..nc {
                end_pts.push(read_u16(d, start + 10 + c * 2).unwrap_or(0) as usize);
            }
            let num_pts = (*end_pts.last().unwrap_or(&0)) + 1;
            let insn_len = read_u16(d, start + 10 + nc * 2).unwrap_or(0) as usize;
            let mut pos = start + 10 + nc * 2 + 2 + insn_len;

            let mut flags = Vec::with_capacity(num_pts);
            while flags.len() < num_pts && pos < end {
                let f = d[pos];
                pos += 1;
                flags.push(f);
                if (f & 0x08) != 0 && pos < end {
                    let repeat = d[pos] as usize;
                    pos += 1;
                    for _ in 0..repeat {
                        if flags.len() < num_pts {
                            flags.push(f);
                        }
                    }
                }
            }
            if flags.len() < num_pts {
                return;
            }

            let mut xs = Vec::with_capacity(num_pts);
            let mut cur_x = 0i32;
            for &f in &flags {
                if (f & 0x02) != 0 {
                    if pos >= end {
                        return;
                    }
                    let dx = d[pos] as i32;
                    pos += 1;
                    cur_x += if (f & 0x10) != 0 { dx } else { -dx };
                } else if (f & 0x10) == 0 {
                    let Some(dx) = read_i16(d, pos) else { return };
                    pos += 2;
                    cur_x += dx as i32;
                }
                xs.push(cur_x as f32);
            }

            let mut ys = Vec::with_capacity(num_pts);
            let mut cur_y = 0i32;
            for &f in &flags {
                if (f & 0x04) != 0 {
                    if pos >= end {
                        return;
                    }
                    let dy = d[pos] as i32;
                    pos += 1;
                    cur_y += if (f & 0x20) != 0 { dy } else { -dy };
                } else if (f & 0x20) == 0 {
                    let Some(dy) = read_i16(d, pos) else { return };
                    pos += 2;
                    cur_y += dy as i32;
                }
                ys.push(cur_y as f32);
            }

            let mut c_start = 0usize;
            for c_end in end_pts {
                if c_end >= num_pts || c_start > c_end {
                    break;
                }
                emit_contour_edges(
                    &xs[c_start..=c_end],
                    &ys[c_start..=c_end],
                    &flags[c_start..=c_end],
                    edges,
                );
                c_start = c_end + 1;
            }
        } else if num_contours < 0 {
            // Composite glyph (e.g., accented letters é, à, ü)
            let mut pos = start + 10;
            loop {
                if pos + 4 > end {
                    break;
                }
                let comp_flags = read_u16(d, pos).unwrap_or(0);
                let sub_gid = read_u16(d, pos + 2).unwrap_or(0);
                pos += 4;

                let (dx, dy): (f32, f32) = if (comp_flags & 0x0001) != 0 {
                    let a1 = read_i16(d, pos).unwrap_or(0) as f32;
                    let a2 = read_i16(d, pos + 2).unwrap_or(0) as f32;
                    pos += 4;
                    if (comp_flags & 0x0002) != 0 {
                        (a1, a2)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    if pos + 2 > end {
                        break;
                    }
                    let a1 = d[pos] as i8 as f32;
                    let a2 = d[pos + 1] as i8 as f32;
                    pos += 2;
                    if (comp_flags & 0x0002) != 0 {
                        (a1, a2)
                    } else {
                        (0.0, 0.0)
                    }
                };

                if (comp_flags & 0x0008) != 0 {
                    pos += 2;
                } else if (comp_flags & 0x0040) != 0 {
                    pos += 4;
                } else if (comp_flags & 0x0080) != 0 {
                    pos += 8;
                }

                let mut sub_edges = Vec::new();
                self.collect_segments(sub_gid, depth + 1, &mut sub_edges);
                for (x0, y0, x1, y1) in sub_edges {
                    edges.push((x0 + dx, y0 + dy, x1 + dx, y1 + dy));
                }

                if (comp_flags & 0x0020) == 0 {
                    break;
                }
            }
        }
    }

    fn rasterize(&self, ch: char, px_size: f32) -> RasterGlyph {
        let gid = self.glyph_index(ch);
        let scale = px_size / self.units_per_em;
        let advance_px = (self.advance_units(gid) as f32) * scale;

        let mut raw_edges = Vec::new();
        self.collect_segments(gid, 0, &mut raw_edges);
        if raw_edges.is_empty() {
            return RasterGlyph {
                width: 0,
                height: 0,
                offset_x: 0,
                offset_y: 0,
                advance_px,
                alpha: Vec::new(),
            };
        }

        // Convert font coordinates (Y-up) into screen pixel coordinates (Y-down relative to baseline)
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut scaled = Vec::with_capacity(raw_edges.len());

        for (x0, y0, x1, y1) in raw_edges {
            let sx0 = x0 * scale;
            let sy0 = -y0 * scale;
            let sx1 = x1 * scale;
            let sy1 = -y1 * scale;
            min_x = min_x.min(sx0).min(sx1);
            max_x = max_x.max(sx0).max(sx1);
            min_y = min_y.min(sy0).min(sy1);
            max_y = max_y.max(sy0).max(sy1);
            if (sy0 - sy1).abs() > 1e-4 {
                scaled.push((sx0, sy0, sx1, sy1));
            }
        }

        let ix0 = min_x.floor() as i32;
        let iy0 = min_y.floor() as i32;
        let ix1 = max_x.ceil() as i32;
        let iy1 = max_y.ceil() as i32;
        let w = (ix1 - ix0).clamp(1, 256) as u32;
        let h = (iy1 - iy0).clamp(1, 256) as u32;

        // 4x4 supersampled non-zero winding scanline rasterizer
        let mut alpha = vec![0u8; (w * h) as usize];
        let sub_ys = [0.125f32, 0.375, 0.625, 0.875];
        let mut intersections: Vec<(f32, i32)> = Vec::with_capacity(16);

        for py in 0..h {
            let mut row_cov = vec![0u8; w as usize];
            for &sy_frac in &sub_ys {
                let scan_y = (iy0 + py as i32) as f32 + sy_frac;
                intersections.clear();
                for &(x0, y0, x1, y1) in &scaled {
                    let (ymin, ymax, dir) = if y0 < y1 {
                        (y0, y1, 1)
                    } else {
                        (y1, y0, -1)
                    };
                    if scan_y >= ymin && scan_y < ymax {
                        let t = (scan_y - y0) / (y1 - y0);
                        let ix = x0 + t * (x1 - x0) - (ix0 as f32);
                        intersections.push((ix, dir));
                    }
                }
                if intersections.len() < 2 {
                    continue;
                }
                intersections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                let mut winding = 0i32;
                let mut span_start = 0.0f32;
                for &(x_hit, dir) in &intersections {
                    if winding == 0 {
                        span_start = x_hit;
                    }
                    winding += dir;
                    if winding == 0 {
                        let span_end = x_hit;
                        let px_start = (span_start.floor() as i32).max(0) as u32;
                        let px_end = (span_end.ceil() as i32).min(w as i32).max(0) as u32;
                        for px in px_start..px_end {
                            let left = (px as f32).max(span_start);
                            let right = ((px + 1) as f32).min(span_end);
                            if right > left {
                                let cov = ((right - left) * 63.75).round() as u8;
                                row_cov[px as usize] = row_cov[px as usize].saturating_add(cov);
                            }
                        }
                    }
                }
            }
            let row_off = (py * w) as usize;
            alpha[row_off..row_off + w as usize].copy_from_slice(&row_cov);
        }

        RasterGlyph {
            width: w,
            height: h,
            offset_x: ix0,
            offset_y: iy0,
            advance_px,
            alpha,
        }
    }
}

fn emit_contour_edges(
    xs: &[f32],
    ys: &[f32],
    flags: &[u8],
    edges: &mut Vec<(f32, f32, f32, f32)>,
) {
    let n = xs.len();
    if n < 2 {
        return;
    }
    // Build expanded list of (point, on_curve) inserting implicit midpoints between consecutive off-curve points
    let mut pts: Vec<(f32, f32, bool)> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let on1 = (flags[i] & 0x01) != 0;
        pts.push((xs[i], ys[i], on1));
        let next = (i + 1) % n;
        let on2 = (flags[next] & 0x01) != 0;
        if !on1 && !on2 {
            pts.push(((xs[i] + xs[next]) * 0.5, (ys[i] + ys[next]) * 0.5, true));
        }
    }

    // Rotate so first point is on-curve
    if let Some(first_on) = pts.iter().position(|p| p.2) {
        pts.rotate_left(first_on);
    } else {
        return;
    }

    let m = pts.len();
    let mut i = 0usize;
    while i < m {
        let (x0, y0, _) = pts[i];
        let (x1, y1, on1) = pts[(i + 1) % m];
        if on1 {
            edges.push((x0, y0, x1, y1));
            i += 1;
        } else {
            let (x2, y2, _) = pts[(i + 2) % m];
            // Flatten quadratic Bezier (x0,y0) -> (x1,y1) -> (x2,y2) into 6 crisp segments
            let mut px = x0;
            let mut py = y0;
            const STEPS: usize = 6;
            for s in 1..=STEPS {
                let t = (s as f32) / (STEPS as f32);
                let mt = 1.0 - t;
                let nx = mt * mt * x0 + 2.0 * mt * t * x1 + t * t * x2;
                let ny = mt * mt * y0 + 2.0 * mt * t * y1 + t * t * y2;
                edges.push((px, py, nx, ny));
                px = nx;
                py = ny;
            }
            i += 2;
        }
    }
}

pub struct FontBook {
    sans_reg: TtfFace,
    sans_bold: TtfFace,
    serif_reg: TtfFace,
    mono_reg: TtfFace,
    cache: HashMap<(FontSlot, char, u16), RasterGlyph>,
    unicode_adv_cache: HashMap<(FontSlot, char), u16>,
}

impl Default for FontBook {
    fn default() -> Self {
        Self::new()
    }
}

impl FontBook {
    pub fn new() -> Self {
        Self {
            sans_reg: TtfFace::parse(SANS_REGULAR_TTF).expect("valid embedded Sans-Regular.ttf"),
            sans_bold: TtfFace::parse(SANS_BOLD_TTF).expect("valid embedded Sans-Bold.ttf"),
            serif_reg: TtfFace::parse(SERIF_REGULAR_TTF).expect("valid embedded Serif-Regular.ttf"),
            mono_reg: TtfFace::parse(MONO_REGULAR_TTF).expect("valid embedded Mono-Regular.ttf"),
            cache: HashMap::with_capacity(1024),
            unicode_adv_cache: HashMap::with_capacity(256),
        }
    }

    fn face(&self, slot: FontSlot) -> &TtfFace {
        match slot {
            FontSlot::SansRegular => &self.sans_reg,
            FontSlot::SansBold => &self.sans_bold,
            FontSlot::SerifRegular => &self.serif_reg,
            FontSlot::MonoRegular => &self.mono_reg,
        }
    }

    pub fn glyph(&mut self, slot: FontSlot, ch: char, px_size: f32) -> &RasterGlyph {
        let size_key = (px_size.clamp(6.0, 140.0) * 4.0).round() as u16;
        let quantized_px = (size_key as f32) / 4.0;
        let key = (slot, ch, size_key);
        if !self.cache.contains_key(&key) {
            if self.cache.len() >= 4096 {
                self.cache.clear();
            }
            let raster = self.face(slot).rasterize(ch, quantized_px);
            self.cache.insert(key, raster);
        }
        &self.cache[&key]
    }

    /// Ultra-fast glyph-advance measurement (`O(1)` array lookup for ASCII, zero bitmap rasterization!).
    pub fn measure_text(&mut self, slot: FontSlot, text: &str, px_size: f32) -> f32 {
        let size_key = (px_size.clamp(6.0, 140.0) * 4.0).round() as u16;
        let quantized_px = (size_key as f32) / 4.0;
        let units_per_em = self.face(slot).units_per_em;
        let scale = quantized_px / units_per_em;
        let space_adv = (self.face(slot).ascii_advances[b' ' as usize] as f32) * scale;

        let mut w = 0.0f32;
        for ch in text.chars() {
            if ch == '\t' {
                w += space_adv * 4.0;
            } else if ch != '\n' {
                let cp = ch as u32;
                let adv_u16 = if cp < 128 {
                    self.face(slot).ascii_advances[cp as usize]
                } else if let Some(&u) = self.unicode_adv_cache.get(&(slot, ch)) {
                    u
                } else {
                    let f = self.face(slot);
                    let u = f.advance_units(f.glyph_index(ch));
                    if self.unicode_adv_cache.len() >= 4096 {
                        self.unicode_adv_cache.clear();
                    }
                    self.unicode_adv_cache.insert((slot, ch), u);
                    u
                };
                w += (adv_u16 as f32) * scale;
            }
        }
        w
    }

    pub fn ascent_px(&self, slot: FontSlot, px_size: f32) -> f32 {
        let f = self.face(slot);
        (f.ascent as f32) * (px_size / f.units_per_em)
    }

    pub fn line_height_px(&self, slot: FontSlot, px_size: f32) -> f32 {
        let f = self.face(slot);
        ((f.ascent - f.descent) as f32) * (px_size / f.units_per_em) * 1.15
    }
}

#[inline]
fn read_u16(d: &[u8], off: usize) -> Option<u16> {
    if off + 2 <= d.len() {
        Some(u16::from_be_bytes([d[off], d[off + 1]]))
    } else {
        None
    }
}

#[inline]
fn read_i16(d: &[u8], off: usize) -> Option<i16> {
    read_u16(d, off).map(|v| v as i16)
}

#[inline]
fn read_u32(d: &[u8], off: usize) -> Option<u32> {
    if off + 4 <= d.len() {
        Some(u32::from_be_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]]))
    } else {
        None
    }
}
