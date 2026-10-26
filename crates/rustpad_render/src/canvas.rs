//! Smooth 2D Anti-Aliased Pixel Canvas (`Surface`) with SDF rounded rectangles,
//! floating paper elevation shadows, crisp hairline borders, TrueType text drawing,
//! anti-aliased polylines, scissor clipping, and embedded RustPad logo blitting.

use crate::theme::Color;
use crate::ttf::{FontBook, FontSlot};

pub const LOGO_64_RGBA: &[u8] = include_bytes!("../../../assets/icons/logo_64.rgba");
pub const LOGO_36_RGBA: &[u8] = include_bytes!("../../../assets/icons/logo_36.rgba");

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    pub fn inset(&self, pad: f32) -> Self {
        Self {
            x: self.x + pad,
            y: self.y + pad,
            w: (self.w - pad * 2.0).max(0.0),
            h: (self.h - pad * 2.0).max(0.0),
        }
    }
}

pub struct Surface {
    pub width: u32,
    pub height: u32,
    /// Packed `0xFFRRGGBB` pixel buffer compatible with X11 32-bit ZPixmap & Win32 top-down DIB.
    pub pixels: Vec<u32>,
    clip: (i32, i32, i32, i32),
}

impl Surface {
    pub fn new(width: u32, height: u32, bg: Color) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        Self {
            width: w,
            height: h,
            pixels: vec![bg.pack_argb(); (w * h) as usize],
            clip: (0, 0, w as i32, h as i32),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, bg: Color) {
        let w = width.max(1);
        let h = height.max(1);
        if self.width != w || self.height != h {
            self.width = w;
            self.height = h;
            self.pixels.resize((w * h) as usize, bg.pack_argb());
        }
        self.reset_clip();
        self.clear(bg);
    }

    pub fn clear(&mut self, bg: Color) {
        self.pixels.fill(bg.pack_argb());
    }

    pub fn set_clip(&mut self, r: Rect) {
        let x0 = (r.x.floor() as i32).clamp(0, self.width as i32);
        let y0 = (r.y.floor() as i32).clamp(0, self.height as i32);
        let x1 = ((r.x + r.w).ceil() as i32).clamp(x0, self.width as i32);
        let y1 = ((r.y + r.h).ceil() as i32).clamp(y0, self.height as i32);
        self.clip = (x0, y0, x1, y1);
    }

    pub fn reset_clip(&mut self) {
        self.clip = (0, 0, self.width as i32, self.height as i32);
    }

    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, src: Color) {
        if src.a == 0 {
            return;
        }
        let (cx0, cy0, cx1, cy1) = self.clip;
        if x < cx0 || x >= cx1 || y < cy0 || y >= cy1 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        if src.a == 255 {
            self.pixels[idx] = src.pack_argb();
            return;
        }
        let dst = self.pixels[idx];
        let dr = ((dst >> 16) & 0xFF) as u32;
        let dg = ((dst >> 8) & 0xFF) as u32;
        let db = (dst & 0xFF) as u32;
        let a = src.a as u32;
        let inv = 255 - a;
        let r = (src.r as u32 * a + dr * inv + 127) / 255;
        let g = (src.g as u32 * a + dg * inv + 127) / 255;
        let b = (src.b as u32 * a + db * inv + 127) / 255;
        self.pixels[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }

    pub fn fill_rect(&mut self, r: Rect, color: Color) {
        if color.a == 0 || r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        let (cx0, cy0, cx1, cy1) = self.clip;
        let x0 = (r.x.round() as i32).max(cx0);
        let y0 = (r.y.round() as i32).max(cy0);
        let x1 = ((r.x + r.w).round() as i32).min(cx1);
        let y1 = ((r.y + r.h).round() as i32).min(cy1);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        if color.a == 255 {
            let packed = color.pack_argb();
            let stride = self.width as usize;
            for y in y0..y1 {
                let row = y as usize * stride;
                self.pixels[row + x0 as usize..row + x1 as usize].fill(packed);
            }
        } else {
            for y in y0..y1 {
                for x in x0..x1 {
                    self.blend_pixel(x, y, color);
                }
            }
        }
    }

    pub fn fill_rounded_rect(&mut self, r: Rect, radius: f32, color: Color) {
        if radius <= 0.5 {
            self.fill_rect(r, color);
            return;
        }
        let rad = radius.min(r.w * 0.5).min(r.h * 0.5);
        let (cx0, cy0, cx1, cy1) = self.clip;
        let x0 = (r.x.floor() as i32).max(cx0);
        let y0 = (r.y.floor() as i32).max(cy0);
        let x1 = ((r.x + r.w).ceil() as i32).min(cx1);
        let y1 = ((r.y + r.h).ceil() as i32).min(cy1);

        let center_x = r.x + r.w * 0.5;
        let center_y = r.y + r.h * 0.5;
        let half_w = r.w * 0.5 - rad;
        let half_h = r.h * 0.5 - rad;

        for py in y0..y1 {
            let dy = ((py as f32 + 0.5) - center_y).abs() - half_h;
            for px in x0..x1 {
                let dx = ((px as f32 + 0.5) - center_x).abs() - half_w;
                if dx <= 0.0 && dy <= 0.0 {
                    self.blend_pixel(px, py, color);
                } else {
                    let ox = dx.max(0.0);
                    let oy = dy.max(0.0);
                    let dist = (ox * ox + oy * oy).sqrt() - rad;
                    if dist < 0.5 {
                        let cov = (0.5 - dist).clamp(0.0, 1.0);
                        let a = ((color.a as f32) * cov).round() as u8;
                        self.blend_pixel(px, py, color.with_alpha(a));
                    }
                }
            }
        }
    }

    pub fn stroke_rounded_rect(&mut self, r: Rect, radius: f32, stroke_w: f32, color: Color) {
        let rad = radius.min(r.w * 0.5).min(r.h * 0.5);
        let (cx0, cy0, cx1, cy1) = self.clip;
        let pad = stroke_w.ceil() as i32 + 1;
        let x0 = (r.x.floor() as i32 - pad).max(cx0);
        let y0 = (r.y.floor() as i32 - pad).max(cy0);
        let x1 = ((r.x + r.w).ceil() as i32 + pad).min(cx1);
        let y1 = ((r.y + r.h).ceil() as i32 + pad).min(cy1);

        let center_x = r.x + r.w * 0.5;
        let center_y = r.y + r.h * 0.5;
        let half_w = r.w * 0.5 - rad;
        let half_h = r.h * 0.5 - rad;
        let half_stroke = stroke_w * 0.5;

        for py in y0..y1 {
            let dy = ((py as f32 + 0.5) - center_y).abs() - half_h;
            for px in x0..x1 {
                let dx = ((px as f32 + 0.5) - center_x).abs() - half_w;
                // Quickly skip interior far from border
                if dx < -rad - stroke_w && dy < -rad - stroke_w {
                    continue;
                }
                let ox = dx.max(0.0);
                let oy = dy.max(0.0);
                let inside = dx.max(dy).min(0.0);
                let sdf = (ox * ox + oy * oy).sqrt() + inside - rad;
                let border_dist = sdf.abs() - half_stroke;
                if border_dist < 0.65 {
                    let cov = (0.65 - border_dist).clamp(0.0, 1.0);
                    let a = ((color.a as f32) * cov).round() as u8;
                    self.blend_pixel(px, py, color.with_alpha(a));
                }
            }
        }
    }

    /// Renders a soft floating elevation shadow behind a white document sheet or popover card.
    pub fn draw_paper_shadow(&mut self, r: Rect, radius: f32, spread: f32, max_alpha: u8) {
        let (cx0, cy0, cx1, cy1) = self.clip;
        let pad = spread.ceil() as i32 + 2;
        let sy_offset = 3.0f32;
        let x0 = (r.x.floor() as i32 - pad).max(cx0);
        let y0 = ((r.y + sy_offset).floor() as i32 - pad).max(cy0);
        let x1 = ((r.x + r.w).ceil() as i32 + pad).min(cx1);
        let y1 = ((r.y + r.h + sy_offset).ceil() as i32 + pad).min(cy1);

        let rad = radius.min(r.w * 0.5).min(r.h * 0.5);
        let center_x = r.x + r.w * 0.5;
        let center_y = r.y + sy_offset + r.h * 0.5;
        let half_w = r.w * 0.5 - rad;
        let half_h = r.h * 0.5 - rad;

        for py in y0..y1 {
            // Skip interior covered by the opaque white sheet itself
            let in_sheet_y = (py as f32) >= r.y + radius && (py as f32) < r.y + r.h - radius;
            let dy = ((py as f32 + 0.5) - center_y).abs() - half_h;
            for px in x0..x1 {
                if in_sheet_y && (px as f32) >= r.x + 1.0 && (px as f32) < r.x + r.w - 1.0 {
                    continue;
                }
                let dx = ((px as f32 + 0.5) - center_x).abs() - half_w;
                let ox = dx.max(0.0);
                let oy = dy.max(0.0);
                let dist = (ox * ox + oy * oy).sqrt() - rad;
                if dist < spread {
                    let t = (1.0 - (dist.max(0.0) / spread)).clamp(0.0, 1.0);
                    let falloff = t * t;
                    let a = ((max_alpha as f32) * falloff).round() as u8;
                    self.blend_pixel(px, py, Color::rgba(15, 23, 42, a));
                }
            }
        }
    }

    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: Color) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.01 {
            return;
        }
        let half = (thickness * 0.5).max(0.5);
        let min_x = (x0.min(x1) - half - 1.0).floor() as i32;
        let max_x = (x0.max(x1) + half + 1.0).ceil() as i32;
        let min_y = (y0.min(y1) - half - 1.0).floor() as i32;
        let max_y = (y0.max(y1) + half + 1.0).ceil() as i32;

        let len_sq = len * len;
        for py in min_y..=max_y {
            let fy = py as f32 + 0.5;
            for px in min_x..=max_x {
                let fx = px as f32 + 0.5;
                let t = (((fx - x0) * dx + (fy - y0) * dy) / len_sq).clamp(0.0, 1.0);
                let proj_x = x0 + t * dx;
                let proj_y = y0 + t * dy;
                let dist = ((fx - proj_x).powi(2) + (fy - proj_y).powi(2)).sqrt() - half;
                if dist < 0.65 {
                    let cov = (0.65 - dist).clamp(0.0, 1.0);
                    let a = ((color.a as f32) * cov).round() as u8;
                    self.blend_pixel(px, py, color.with_alpha(a));
                }
            }
        }
    }

    /// Draws anti-aliased TrueType text at `(x, baseline_y)`, returning the final X cursor.
    pub fn draw_text(
        &mut self,
        fonts: &mut FontBook,
        slot: FontSlot,
        px_size: f32,
        x: f32,
        baseline_y: f32,
        color: Color,
        text: &str,
    ) -> f32 {
        let mut pen_x = x;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            if ch == '\t' {
                pen_x += fonts.glyph(slot, ' ', px_size).advance_px * 4.0;
                continue;
            }
            let g = fonts.glyph(slot, ch, px_size);
            let gx0 = pen_x.round() as i32 + g.offset_x;
            let gy0 = baseline_y.round() as i32 + g.offset_y;
            let gw = g.width as i32;
            let gh = g.height as i32;
            let adv = g.advance_px;

            if gw > 0 && gh > 0 {
                // Copy needed fields before borrowing self mutably
                for row in 0..gh {
                    let py = gy0 + row;
                    if py < self.clip.1 || py >= self.clip.3 {
                        continue;
                    }
                    let row_base = (row * gw) as usize;
                    for col in 0..gw {
                        let a_cov = g.alpha[row_base + col as usize];
                        if a_cov > 0 {
                            let eff_a = ((a_cov as u32 * color.a as u32) / 255) as u8;
                            self.blend_pixel(gx0 + col, py, color.with_alpha(eff_a));
                        }
                    }
                }
            }
            pen_x += adv;
        }
        pen_x
    }

    pub fn draw_text_clipped(
        &mut self,
        fonts: &mut FontBook,
        slot: FontSlot,
        px_size: f32,
        x: f32,
        baseline_y: f32,
        max_width: f32,
        color: Color,
        text: &str,
    ) {
        if fonts.measure_text(slot, text, px_size) <= max_width {
            self.draw_text(fonts, slot, px_size, x, baseline_y, color, text);
            return;
        }
        let ellipsis_w = fonts.measure_text(slot, "…", px_size);
        let budget = (max_width - ellipsis_w).max(0.0);
        let mut clipped = String::new();
        let mut acc = 0.0f32;
        for ch in text.chars() {
            let adv = fonts.glyph(slot, ch, px_size).advance_px;
            if acc + adv > budget {
                break;
            }
            acc += adv;
            clipped.push(ch);
        }
        clipped.push('…');
        self.draw_text(fonts, slot, px_size, x, baseline_y, color, &clipped);
    }

    pub fn blit_rgba(&mut self, dst_x: i32, dst_y: i32, src_w: u32, src_h: u32, rgba: &[u8]) {
        if rgba.len() < (src_w * src_h * 4) as usize {
            return;
        }
        for sy in 0..src_h as i32 {
            for sx in 0..src_w as i32 {
                let off = ((sy as u32 * src_w + sx as u32) * 4) as usize;
                let c = Color::rgba(rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3]);
                self.blend_pixel(dst_x + sx, dst_y + sy, c);
            }
        }
    }

    pub fn draw_logo_36(&mut self, x: i32, y: i32) {
        self.blit_rgba(x, y, 36, 36, LOGO_36_RGBA);
    }

    pub fn draw_logo_64(&mut self, x: i32, y: i32) {
        self.blit_rgba(x, y, 64, 64, LOGO_64_RGBA);
    }

    /// Draws crisp anti-aliased geometric desktop icons without bitmap blur or emoji.
    pub fn draw_icon(&mut self, icon: UiIcon, cx: f32, cy: f32, size: f32, color: Color) {
        let h = size * 0.5;
        let sw = (size * 0.105).clamp(1.25, 2.0);
        match icon {
            UiIcon::Sidebar => {
                let r = Rect::new(cx - h, cy - h * 0.82, size, size * 0.82);
                self.stroke_rounded_rect(r, 2.5, sw, color);
                self.draw_line(r.x + r.w * 0.34, r.y + 1.0, r.x + r.w * 0.34, r.y + r.h - 1.0, sw, color);
            }
            UiIcon::SidebarRight => {
                let r = Rect::new(cx - h, cy - h * 0.82, size, size * 0.82);
                self.stroke_rounded_rect(r, 2.5, sw, color);
                self.draw_line(r.x + r.w * 0.66, r.y + 1.0, r.x + r.w * 0.66, r.y + r.h - 1.0, sw, color);
            }
            UiIcon::SplitPane => {
                let r = Rect::new(cx - h, cy - h * 0.82, size, size * 0.82);
                self.stroke_rounded_rect(r, 2.5, sw, color);
                self.draw_line(cx, r.y + 1.0, cx, r.y + r.h - 1.0, sw, color);
            }
            UiIcon::Undo => {
                self.draw_line(cx - h * 0.65, cy - h * 0.15, cx - h * 0.18, cy - h * 0.60, sw, color);
                self.draw_line(cx - h * 0.65, cy - h * 0.15, cx - h * 0.18, cy + h * 0.28, sw, color);
                self.draw_line(cx - h * 0.60, cy - h * 0.15, cx + h * 0.45, cy - h * 0.15, sw, color);
                self.draw_line(cx + h * 0.45, cy - h * 0.15, cx + h * 0.65, cy + h * 0.18, sw, color);
                self.draw_line(cx + h * 0.65, cy + h * 0.18, cx + h * 0.35, cy + h * 0.55, sw, color);
                self.draw_line(cx + h * 0.35, cy + h * 0.55, cx - h * 0.20, cy + h * 0.55, sw, color);
            }
            UiIcon::Redo => {
                self.draw_line(cx + h * 0.65, cy - h * 0.15, cx + h * 0.18, cy - h * 0.60, sw, color);
                self.draw_line(cx + h * 0.65, cy - h * 0.15, cx + h * 0.18, cy + h * 0.28, sw, color);
                self.draw_line(cx + h * 0.60, cy - h * 0.15, cx - h * 0.45, cy - h * 0.15, sw, color);
                self.draw_line(cx - h * 0.45, cy - h * 0.15, cx - h * 0.65, cy + h * 0.18, sw, color);
                self.draw_line(cx - h * 0.65, cy + h * 0.18, cx - h * 0.35, cy + h * 0.55, sw, color);
                self.draw_line(cx - h * 0.35, cy + h * 0.55, cx + h * 0.20, cy + h * 0.55, sw, color);
            }
            UiIcon::AlignLeft => {
                let x0 = cx - h * 0.72;
                for (i, w_mul) in [1.44f32, 0.98, 1.36, 0.82].iter().enumerate() {
                    let y = cy - h * 0.58 + (i as f32) * (size * 0.25);
                    self.draw_line(x0, y, x0 + h * w_mul, y, sw, color);
                }
            }
            UiIcon::AlignCenter => {
                for (i, w_mul) in [0.72f32, 0.46, 0.68, 0.40].iter().enumerate() {
                    let y = cy - h * 0.58 + (i as f32) * (size * 0.25);
                    self.draw_line(cx - h * w_mul, y, cx + h * w_mul, y, sw, color);
                }
            }
            UiIcon::AlignRight => {
                let x1 = cx + h * 0.72;
                for (i, w_mul) in [1.44f32, 0.98, 1.36, 0.82].iter().enumerate() {
                    let y = cy - h * 0.58 + (i as f32) * (size * 0.25);
                    self.draw_line(x1 - h * w_mul, y, x1, y, sw, color);
                }
            }
            UiIcon::BulletList => {
                for i in 0..3 {
                    let y = cy - h * 0.52 + (i as f32) * (size * 0.34);
                    self.fill_rounded_rect(Rect::new(cx - h * 0.78, y - 1.6, 3.2, 3.2), 1.6, color);
                    self.draw_line(cx - h * 0.32, y, cx + h * 0.76, y, sw, color);
                }
            }
            UiIcon::Checklist => {
                let box1 = Rect::new(cx - h * 0.80, cy - h * 0.65, size * 0.34, size * 0.34);
                self.stroke_rounded_rect(box1, 1.5, sw * 0.9, color);
                self.draw_line(box1.x + 1.2, box1.y + box1.h * 0.55, box1.x + box1.w * 0.42, box1.y + box1.h - 1.2, sw * 0.9, color);
                self.draw_line(box1.x + box1.w * 0.42, box1.y + box1.h - 1.2, box1.x + box1.w - 0.8, box1.y + 1.2, sw * 0.9, color);
                self.draw_line(cx - h * 0.12, cy - h * 0.32, cx + h * 0.78, cy - h * 0.32, sw, color);

                let box2 = Rect::new(cx - h * 0.80, cy + h * 0.12, size * 0.34, size * 0.34);
                self.stroke_rounded_rect(box2, 1.5, sw * 0.9, color);
                self.draw_line(cx - h * 0.12, cy + h * 0.45, cx + h * 0.78, cy + h * 0.45, sw, color);
            }
            UiIcon::Table => {
                let r = Rect::new(cx - h * 0.78, cy - h * 0.72, size * 0.78, size * 0.72);
                self.stroke_rounded_rect(r, 2.0, sw, color);
                self.draw_line(r.x, r.y + r.h * 0.38, r.x + r.w, r.y + r.h * 0.38, sw, color);
                self.draw_line(r.x + r.w * 0.5, r.y, r.x + r.w * 0.5, r.y + r.h, sw, color);
            }
            UiIcon::Sparkles => {
                // Clean 4-point architectural star + secondary dot
                self.draw_line(cx - h * 0.12, cy - h * 0.76, cx - h * 0.12, cy + h * 0.64, sw, color);
                self.draw_line(cx - h * 0.82, cy - h * 0.06, cx + h * 0.58, cy - h * 0.06, sw, color);
                self.draw_line(cx - h * 0.48, cy - h * 0.42, cx + h * 0.24, cy + h * 0.30, sw * 0.85, color);
                self.draw_line(cx + h * 0.24, cy - h * 0.42, cx - h * 0.48, cy + h * 0.30, sw * 0.85, color);
                self.draw_line(cx + h * 0.58, cy - h * 0.72, cx + h * 0.58, cy - h * 0.28, sw * 0.85, color);
                self.draw_line(cx + h * 0.36, cy - h * 0.50, cx + h * 0.80, cy - h * 0.50, sw * 0.85, color);
            }
            UiIcon::Search => {
                let r = Rect::new(cx - h * 0.74, cy - h * 0.74, size * 0.62, size * 0.62);
                self.stroke_rounded_rect(r, size * 0.31, sw, color);
                self.draw_line(cx + h * 0.16, cy + h * 0.16, cx + h * 0.74, cy + h * 0.74, sw * 1.15, color);
            }
            UiIcon::Plus => {
                self.draw_line(cx - h * 0.62, cy, cx + h * 0.62, cy, sw * 1.1, color);
                self.draw_line(cx, cy - h * 0.62, cx, cy + h * 0.62, sw * 1.1, color);
            }
            UiIcon::Minus => {
                self.draw_line(cx - h * 0.62, cy, cx + h * 0.62, cy, sw * 1.1, color);
            }
            UiIcon::ChevronDown => {
                self.draw_line(cx - h * 0.52, cy - h * 0.20, cx, cy + h * 0.32, sw * 1.05, color);
                self.draw_line(cx, cy + h * 0.32, cx + h * 0.52, cy - h * 0.20, sw * 1.05, color);
            }
            UiIcon::ChevronRight => {
                self.draw_line(cx - h * 0.22, cy - h * 0.52, cx + h * 0.30, cy, sw * 1.05, color);
                self.draw_line(cx + h * 0.30, cy, cx - h * 0.22, cy + h * 0.52, sw * 1.05, color);
            }
            UiIcon::Close => {
                self.draw_line(cx - h * 0.52, cy - h * 0.52, cx + h * 0.52, cy + h * 0.52, sw * 1.05, color);
                self.draw_line(cx + h * 0.52, cy - h * 0.52, cx - h * 0.52, cy + h * 0.52, sw * 1.05, color);
            }
            UiIcon::Trash => {
                self.draw_line(cx - h * 0.68, cy - h * 0.48, cx + h * 0.68, cy - h * 0.48, sw, color);
                self.draw_line(cx - h * 0.25, cy - h * 0.72, cx + h * 0.25, cy - h * 0.72, sw, color);
                let body = Rect::new(cx - h * 0.50, cy - h * 0.44, size * 0.50, size * 0.60);
                self.stroke_rounded_rect(body, 1.8, sw, color);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiIcon {
    Sidebar,
    SidebarRight,
    SplitPane,
    Undo,
    Redo,
    AlignLeft,
    AlignCenter,
    AlignRight,
    BulletList,
    Checklist,
    Table,
    Sparkles,
    Search,
    Plus,
    Minus,
    ChevronDown,
    ChevronRight,
    Close,
    Trash,
}
