//! Pure-Rust PDF 1.4–1.7 Object & Content Stream Parser.

use crate::{PdfAnnotation, PdfDocument, PdfPage, PdfRectItem, PdfTextItem};
use doc_model::{FontFamily, PaperSize, RgbColor};
use rustpad_zip::{deflate_decompress, zlib_decompress};
use std::collections::BTreeMap;
use std::path::Path;

pub fn load_pdf_file(path: impl AsRef<Path>) -> Result<PdfDocument, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let mut doc = load_pdf_bytes(&bytes)?;
    if doc.title.is_empty() || doc.title == "Untitled PDF" {
        if let Some(stem) = path.as_ref().file_stem().and_then(|s| s.to_str()) {
            doc.title = stem.to_string();
        }
    }
    Ok(doc)
}

pub fn load_pdf_bytes(bytes: &[u8]) -> Result<PdfDocument, String> {
    if !bytes.starts_with(b"%PDF-") {
        return Err("Not a valid PDF file (missing %PDF- header)".into());
    }

    let objects = scan_and_expand_objects(bytes);
    if objects.is_empty() {
        return Err("No PDF objects found in file".into());
    }

    let mut title = "Untitled PDF".to_string();
    let mut author = "Nour Yahyaoui".to_string();

    for body in objects.values() {
        let text = String::from_utf8_lossy(body);
        if text.contains("/Title") && text.contains("/Author") {
            if let Some(t) = extract_pdf_paren_field(&text, "/Title") {
                if !t.trim().is_empty() {
                    title = t;
                }
            }
            if let Some(a) = extract_pdf_paren_field(&text, "/Author") {
                if !a.trim().is_empty() {
                    author = a;
                }
            }
        }
    }

    // Collect /Type /Page objects in document order (via /Pages /Kids if available, else sorted id)
    let mut page_obj_ids = collect_kids_page_ids(&objects);
    if page_obj_ids.is_empty() {
        for (&id, body) in &objects {
            let hdr = header_ascii(body);
            if is_type_page(&hdr) {
                page_obj_ids.push(id);
            }
        }
    }

    let mut pages = Vec::new();
    for page_id in page_obj_ids {
        let Some(page_body) = objects.get(&page_id) else {
            continue;
        };
        let hdr = header_ascii(page_body);
        let (w_pt, h_pt) = extract_media_box(&hdr).unwrap_or((595.28, 841.89));
        let rot = extract_int_field(&hdr, "/Rotate")
            .map(|r| r.rem_euclid(360) as u16)
            .unwrap_or(0);

        let content_ids = extract_content_refs(&hdr);
        let mut combined_stream = String::new();
        for cid in content_ids {
            if let Some(c_obj) = objects.get(&cid) {
                if let Some(decoded) = decode_stream_object(c_obj) {
                    combined_stream.push_str(&String::from_utf8_lossy(&decoded));
                    combined_stream.push('\n');
                }
            }
        }

        let mut page = parse_content_stream(&combined_stream, w_pt, h_pt, rot);
        extract_page_annots(&hdr, &objects, &mut page);
        pages.push(page);
    }

    if pages.is_empty() {
        pages.push(PdfPage::blank(PaperSize::A4));
    }

    Ok(PdfDocument {
        title,
        author,
        pages,
    })
}

pub(crate) fn scan_and_expand_objects(bytes: &[u8]) -> BTreeMap<u32, Vec<u8>> {
    let mut objects = scan_pdf_objects(bytes);
    expand_object_streams(&mut objects);
    objects
}

/// Unpacks ISO 32000-1 §7.5.7 `/Type /ObjStm` compressed object streams into `objects`.
fn expand_object_streams(objects: &mut BTreeMap<u32, Vec<u8>>) {
    let mut unpacked: Vec<(u32, Vec<u8>)> = Vec::new();
    for body in objects.values() {
        let hdr = header_ascii(body);
        if !hdr.contains("/ObjStm") {
            continue;
        }
        let Some(n_objs) = extract_int_field(&hdr, "/N").map(|v| v.max(0) as usize) else {
            continue;
        };
        let Some(first_off) = extract_int_field(&hdr, "/First").map(|v| v.max(0) as usize) else {
            continue;
        };
        if n_objs == 0 || n_objs > 16_384 {
            continue;
        }
        let Some(decoded) = decode_stream_object(body) else {
            continue;
        };
        if first_off > decoded.len() {
            continue;
        }
        let index_str = String::from_utf8_lossy(&decoded[..first_off]);
        let nums: Vec<usize> = index_str
            .split_whitespace()
            .filter_map(|t| t.parse::<usize>().ok())
            .collect();
        let mut pairs: Vec<(u32, usize)> = Vec::with_capacity(n_objs);
        let mut idx = 0usize;
        while idx + 1 < nums.len() && pairs.len() < n_objs {
            pairs.push((nums[idx] as u32, nums[idx + 1]));
            idx += 2;
        }
        for (k, &(obj_id, rel_start)) in pairs.iter().enumerate() {
            let abs_start = first_off.saturating_add(rel_start);
            let abs_end = if k + 1 < pairs.len() {
                first_off.saturating_add(pairs[k + 1].1).min(decoded.len())
            } else {
                decoded.len()
            };
            if abs_start <= abs_end && abs_end <= decoded.len() {
                unpacked.push((obj_id, decoded[abs_start..abs_end].to_vec()));
            }
        }
    }
    for (id, data) in unpacked {
        objects.entry(id).or_insert(data);
    }
}

fn scan_pdf_objects(bytes: &[u8]) -> BTreeMap<u32, Vec<u8>> {
    let mut map = BTreeMap::new();
    let mut i = 0usize;
    let len = bytes.len();

    while i + 10 < len {
        // Look for `<num> <gen> obj` at start of line or after whitespace
        if bytes[i].is_ascii_digit() && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            let start_num = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let Ok(obj_id) = std::str::from_utf8(&bytes[start_num..i])
                .unwrap_or("")
                .parse::<u32>()
            else {
                continue;
            };
            if i >= len || !bytes[i].is_ascii_whitespace() {
                continue;
            }
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let start_gen = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if start_gen == i || i >= len || !bytes[i].is_ascii_whitespace() {
                continue;
            }
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i + 3 <= len && &bytes[i..i + 3] == b"obj" && (i + 3 == len || !bytes[i + 3].is_ascii_alphabetic()) {
                i += 3;
                while i < len && (bytes[i] == b'\r' || bytes[i] == b'\n' || bytes[i] == b' ') {
                    i += 1;
                }
                let body_start = i;
                if let Some(rel_end) = find_subslice(&bytes[body_start..], b"endobj") {
                    let mut body_end = body_start + rel_end;
                    while body_end > body_start
                        && (bytes[body_end - 1] == b'\n' || bytes[body_end - 1] == b'\r')
                    {
                        body_end -= 1;
                    }
                    map.insert(obj_id, bytes[body_start..body_end].to_vec());
                    i = body_start + rel_end + 6;
                    continue;
                }
            }
        }
        i += 1;
    }

    map
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn header_ascii(obj: &[u8]) -> String {
    let end = find_subslice(obj, b"stream").unwrap_or(obj.len());
    String::from_utf8_lossy(&obj[..end]).into_owned()
}

fn is_type_page(hdr: &str) -> bool {
    if let Some(pos) = hdr.find("/Type") {
        let rest = hdr[pos + 5..].trim_start();
        if let Some(after) = rest.strip_prefix("/Page") {
            return !after.starts_with('s');
        }
    }
    false
}

fn collect_kids_page_ids(objects: &BTreeMap<u32, Vec<u8>>) -> Vec<u32> {
    // Find root /Pages object
    let mut root_pages_id = None;
    for body in objects.values() {
        let hdr = header_ascii(body);
        if hdr.contains("/Type") && hdr.contains("/Catalog") {
            if let Some(pos) = hdr.find("/Pages") {
                let refs = parse_obj_refs(&hdr[pos + 6..]);
                if let Some(&first) = refs.first() {
                    root_pages_id = Some(first);
                    break;
                }
            }
        }
    }

    let mut out = Vec::new();
    let mut visited = Vec::new();
    if let Some(pid) = root_pages_id {
        walk_pages_tree(pid, objects, &mut visited, &mut out);
    }
    out
}

fn walk_pages_tree(
    id: u32,
    objects: &BTreeMap<u32, Vec<u8>>,
    visited: &mut Vec<u32>,
    out: &mut Vec<u32>,
) {
    if visited.contains(&id) {
        return;
    }
    visited.push(id);
    let Some(body) = objects.get(&id) else {
        return;
    };
    let hdr = header_ascii(body);
    if is_type_page(&hdr) {
        out.push(id);
        return;
    }
    if let Some(kpos) = hdr.find("/Kids") {
        let after = &hdr[kpos + 5..];
        if let (Some(lb), Some(rb)) = (after.find('['), after.find(']')) {
            if lb < rb {
                for child_id in parse_obj_refs(&after[lb + 1..rb]) {
                    walk_pages_tree(child_id, objects, visited, out);
                }
            }
        }
    }
}

fn parse_obj_refs(s: &str) -> Vec<u32> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut refs = Vec::new();
    let mut i = 0;
    while i + 2 < tokens.len() {
        if tokens[i + 2].starts_with('R') {
            if let (Ok(id), Ok(_gen)) = (tokens[i].parse::<u32>(), tokens[i + 1].parse::<u32>()) {
                refs.push(id);
                i += 3;
                continue;
            }
        }
        i += 1;
    }
    refs
}

fn extract_media_box(hdr: &str) -> Option<(f32, f32)> {
    let pos = hdr.find("/MediaBox")?;
    let after = &hdr[pos + 9..];
    let lb = after.find('[')?;
    let rb = after[lb..].find(']')? + lb;
    let nums: Vec<f32> = after[lb + 1..rb]
        .split_whitespace()
        .filter_map(|t| t.parse::<f32>().ok())
        .collect();
    if nums.len() == 4 {
        Some(((nums[2] - nums[0]).abs(), (nums[3] - nums[1]).abs()))
    } else {
        None
    }
}

fn extract_int_field(hdr: &str, key: &str) -> Option<i32> {
    let pos = hdr.find(key)?;
    let rest = hdr[pos + key.len()..].trim_start();
    let tok = rest.split(|c: char| c.is_whitespace() || c == '/' || c == '>').next()?;
    tok.parse::<i32>().ok()
}

fn extract_content_refs(hdr: &str) -> Vec<u32> {
    let Some(pos) = hdr.find("/Contents") else {
        return Vec::new();
    };
    let after = hdr[pos + 9..].trim_start();
    if let Some(stripped) = after.strip_prefix('[') {
        if let Some(rb) = stripped.find(']') {
            return parse_obj_refs(&stripped[..rb]);
        }
    }
    parse_obj_refs(after).into_iter().take(1).collect()
}

fn decode_stream_object(obj: &[u8]) -> Option<Vec<u8>> {
    let stream_kw = find_subslice(obj, b"stream")?;
    let hdr = String::from_utf8_lossy(&obj[..stream_kw]);
    let mut data_start = stream_kw + 6;
    if data_start < obj.len() && obj[data_start] == b'\r' {
        data_start += 1;
    }
    if data_start < obj.len() && obj[data_start] == b'\n' {
        data_start += 1;
    }
    let end_kw = find_subslice(&obj[data_start..], b"endstream")?;
    let mut data_end = data_start + end_kw;
    if data_end > data_start && obj[data_end - 1] == b'\n' {
        data_end -= 1;
    }
    if data_end > data_start && obj[data_end - 1] == b'\r' {
        data_end -= 1;
    }
    let slice = &obj[data_start..data_end];
    let raw = if hdr.contains("/FlateDecode") {
        zlib_decompress(slice)
            .or_else(|_| deflate_decompress(slice))
            .ok()?
    } else {
        slice.to_vec()
    };

    if let Some(pred) = extract_int_field(&hdr, "/Predictor") {
        if pred >= 10 {
            let cols = extract_int_field(&hdr, "/Columns")
                .map(|c| c.max(1) as usize)
                .unwrap_or(1);
            if let Some(unfiltered) = undo_png_predictor(&raw, cols) {
                return Some(unfiltered);
            }
        }
    }
    Some(raw)
}

fn undo_png_predictor(data: &[u8], cols: usize) -> Option<Vec<u8>> {
    let row_len = cols.checked_add(1)?;
    if row_len <= 1 || data.is_empty() {
        return None;
    }
    let rows = data.len() / row_len;
    let mut out = Vec::with_capacity(rows * cols);
    let mut prev_row = vec![0u8; cols];

    for r in 0..rows {
        let base = r * row_len;
        let filter = data[base];
        let src = &data[base + 1..base + row_len];
        let mut cur = vec![0u8; cols];
        match filter {
            0 => cur.copy_from_slice(src),
            1 => {
                for i in 0..cols {
                    let left = if i > 0 { cur[i - 1] } else { 0 };
                    cur[i] = src[i].wrapping_add(left);
                }
            }
            2 => {
                for i in 0..cols {
                    cur[i] = src[i].wrapping_add(prev_row[i]);
                }
            }
            _ => cur.copy_from_slice(src),
        }
        out.extend_from_slice(&cur);
        prev_row = cur;
    }
    Some(out)
}

fn extract_pdf_paren_field(text: &str, key: &str) -> Option<String> {
    let pos = text.find(key)?;
    let rest = text[pos + key.len()..].trim_start();
    if !rest.starts_with('(') {
        return None;
    }
    let (decoded, _) = parse_pdf_literal_string(rest.as_bytes());
    Some(decoded)
}

fn extract_page_annots(hdr: &str, objects: &BTreeMap<u32, Vec<u8>>, page: &mut PdfPage) {
    let Some(pos) = hdr.find("/Annots") else {
        return;
    };
    let after = hdr[pos + 7..].trim_start();
    let Some(stripped) = after.strip_prefix('[') else {
        return;
    };
    let Some(rb) = stripped.find(']') else {
        return;
    };
    for aid in parse_obj_refs(&stripped[..rb]) {
        if let Some(abody) = objects.get(&aid) {
            let ahdr = header_ascii(abody);
            if ahdr.contains("/Subtype /Text") {
                let note = extract_pdf_paren_field(&ahdr, "/Contents").unwrap_or_default();
                let author =
                    extract_pdf_paren_field(&ahdr, "/T").unwrap_or_else(|| "Note".into());
                if let Some((x, y, _, _)) = extract_rect_4(&ahdr) {
                    page.annotations.push(PdfAnnotation::StickyNote {
                        x,
                        y,
                        author,
                        note,
                    });
                }
            }
        }
    }
}

fn extract_rect_4(hdr: &str) -> Option<(f32, f32, f32, f32)> {
    let pos = hdr.find("/Rect")?;
    let after = &hdr[pos + 5..];
    let lb = after.find('[')?;
    let rb = after[lb..].find(']')? + lb;
    let nums: Vec<f32> = after[lb + 1..rb]
        .split_whitespace()
        .filter_map(|t| t.parse::<f32>().ok())
        .collect();
    if nums.len() == 4 {
        Some((nums[0], nums[1], nums[2] - nums[0], nums[3] - nums[1]))
    } else {
        None
    }
}

#[derive(Clone)]
struct GfxState {
    fill_rgb: RgbColor,
    stroke_rgb: RgbColor,
    line_width: f32,
}

impl Default for GfxState {
    fn default() -> Self {
        Self {
            fill_rgb: RgbColor::INK_SLATE,
            stroke_rgb: RgbColor::INK_SLATE,
            line_width: 1.0,
        }
    }
}

fn parse_content_stream(stream: &str, width_pt: f32, height_pt: f32, rotation_deg: u16) -> PdfPage {
    let tokens = tokenize_pdf_content(stream.as_bytes());
    let mut state = GfxState::default();
    let mut stack: Vec<GfxState> = Vec::new();
    let mut operand_stack: Vec<PdfToken> = Vec::new();

    let mut cur_font = FontFamily::Sans;
    let mut cur_bold = false;
    let mut cur_italic = false;
    let mut cur_size = 11.5f32;
    let mut text_x = 0.0f32;
    let mut text_y = 0.0f32;
    let mut pending_rect: Option<(f32, f32, f32, f32)> = None;
    let mut current_stroke_pts: Vec<(f32, f32)> = Vec::new();

    let mut text_items: Vec<PdfTextItem> = Vec::new();
    let mut rects: Vec<PdfRectItem> = Vec::new();
    let mut annotations: Vec<PdfAnnotation> = Vec::new();
    let mut next_id = 1u32;

    for tok in tokens {
        match &tok {
            PdfToken::Op(op) => {
                match op.as_str() {
                    "q" => stack.push(state.clone()),
                    "Q" => {
                        if let Some(prev) = stack.pop() {
                            state = prev;
                        }
                    }
                    "BT" => {
                        text_x = 0.0;
                        text_y = 0.0;
                    }
                    "ET" => {}
                    "w" => {
                        if let Some(w) = pop_num(&mut operand_stack) {
                            state.line_width = w;
                        }
                    }
                    "rg" => {
                        let b = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let g = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let r = pop_num(&mut operand_stack).unwrap_or(0.0);
                        state.fill_rgb = RgbColor::new(
                            (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                            (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                            (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                        );
                    }
                    "RG" => {
                        let b = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let g = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let r = pop_num(&mut operand_stack).unwrap_or(0.0);
                        state.stroke_rgb = RgbColor::new(
                            (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                            (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                            (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                        );
                    }
                    "g" => {
                        if let Some(gray) = pop_num(&mut operand_stack) {
                            let v = (gray.clamp(0.0, 1.0) * 255.0).round() as u8;
                            state.fill_rgb = RgbColor::new(v, v, v);
                        }
                    }
                    "G" => {
                        if let Some(gray) = pop_num(&mut operand_stack) {
                            let v = (gray.clamp(0.0, 1.0) * 255.0).round() as u8;
                            state.stroke_rgb = RgbColor::new(v, v, v);
                        }
                    }
                    "Tf" => {
                        let sz = pop_num(&mut operand_stack).unwrap_or(11.5);
                        cur_size = sz.abs().max(6.0);
                        if let Some(PdfToken::Name(fname)) = operand_stack.pop() {
                            decode_font_alias(&fname, &mut cur_font, &mut cur_bold, &mut cur_italic);
                        }
                    }
                    "Td" | "TD" => {
                        let ty = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let tx = pop_num(&mut operand_stack).unwrap_or(0.0);
                        text_x += tx;
                        text_y += ty;
                    }
                    "Tm" => {
                        let ty = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let tx = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let _d = pop_num(&mut operand_stack).unwrap_or(1.0);
                        let _c = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let _b = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let a = pop_num(&mut operand_stack).unwrap_or(1.0);
                        if a.abs() > 1.5 && cur_size <= 1.5 {
                            cur_size = a.abs();
                        }
                        text_x = tx;
                        text_y = ty;
                    }
                    "Tj" => {
                        if let Some(PdfToken::Str(s)) = operand_stack.pop() {
                            if !s.is_empty() && cur_size < 46.0 {
                                text_items.push(PdfTextItem {
                                    id: next_id,
                                    x: text_x,
                                    y: text_y,
                                    font_family: cur_font,
                                    font_size: cur_size,
                                    bold: cur_bold,
                                    italic: cur_italic,
                                    color: state.fill_rgb,
                                    text: s,
                                });
                                next_id += 1;
                            }
                        }
                    }
                    "TJ" => {
                        if let Some(PdfToken::Array(arr)) = operand_stack.pop() {
                            let mut combined = String::new();
                            for item in arr {
                                match item {
                                    PdfToken::Str(s) => combined.push_str(&s),
                                    PdfToken::Num(kern) if kern < -180.0 => combined.push(' '),
                                    _ => {}
                                }
                            }
                            if !combined.is_empty() {
                                text_items.push(PdfTextItem {
                                    id: next_id,
                                    x: text_x,
                                    y: text_y,
                                    font_family: cur_font,
                                    font_size: cur_size,
                                    bold: cur_bold,
                                    italic: cur_italic,
                                    color: state.fill_rgb,
                                    text: combined,
                                });
                                next_id += 1;
                            }
                        }
                    }
                    "re" => {
                        let h = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let w = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let y = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let x = pop_num(&mut operand_stack).unwrap_or(0.0);
                        pending_rect = Some((x, y, w, h));
                    }
                    "f" | "F" => {
                        if let Some((x, y, w, h)) = pending_rect.take() {
                            rects.push(PdfRectItem {
                                x,
                                y,
                                w,
                                h,
                                fill: Some(state.fill_rgb),
                                stroke: None,
                            });
                        }
                    }
                    "B" => {
                        if let Some((x, y, w, h)) = pending_rect.take() {
                            rects.push(PdfRectItem {
                                x,
                                y,
                                w,
                                h,
                                fill: Some(state.fill_rgb),
                                stroke: Some((state.stroke_rgb, state.line_width)),
                            });
                        }
                    }
                    "m" => {
                        let y = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let x = pop_num(&mut operand_stack).unwrap_or(0.0);
                        current_stroke_pts.clear();
                        current_stroke_pts.push((x, y));
                    }
                    "l" => {
                        let y = pop_num(&mut operand_stack).unwrap_or(0.0);
                        let x = pop_num(&mut operand_stack).unwrap_or(0.0);
                        current_stroke_pts.push((x, y));
                    }
                    "S" => {
                        if let Some((x, y, w, h)) = pending_rect.take() {
                            rects.push(PdfRectItem {
                                x,
                                y,
                                w,
                                h,
                                fill: None,
                                stroke: Some((state.stroke_rgb, state.line_width)),
                            });
                        } else if current_stroke_pts.len() >= 3 {
                            annotations.push(PdfAnnotation::Signature {
                                strokes: vec![std::mem::take(&mut current_stroke_pts)],
                                signer: String::new(),
                                color: state.stroke_rgb,
                            });
                        } else if current_stroke_pts.len() == 2 {
                            let (x0, y0) = current_stroke_pts[0];
                            let (x1, y1) = current_stroke_pts[1];
                            rects.push(PdfRectItem {
                                x: x0.min(x1),
                                y: y0.min(y1),
                                w: (x1 - x0).abs().max(0.8),
                                h: (y1 - y0).abs().max(0.8),
                                fill: Some(state.stroke_rgb),
                                stroke: None,
                            });
                            current_stroke_pts.clear();
                        }
                    }
                    _ => {}
                }
                operand_stack.clear();
            }
            _ => operand_stack.push(tok),
        }
    }

    PdfPage {
        width_pt,
        height_pt,
        rotation_deg,
        text_items,
        rects,
        annotations,
        watermark: None,
    }
}

fn decode_font_alias(alias: &str, family: &mut FontFamily, bold: &mut bool, italic: &mut bool) {
    match alias {
        "F1" => (*family, *bold, *italic) = (FontFamily::Sans, false, false),
        "F2" => (*family, *bold, *italic) = (FontFamily::Sans, true, false),
        "F3" => (*family, *bold, *italic) = (FontFamily::Sans, false, true),
        "F4" => (*family, *bold, *italic) = (FontFamily::Sans, true, true),
        "F5" => (*family, *bold, *italic) = (FontFamily::Serif, false, false),
        "F6" => (*family, *bold, *italic) = (FontFamily::Serif, true, false),
        "F7" => (*family, *bold, *italic) = (FontFamily::Serif, false, true),
        "F8" => (*family, *bold, *italic) = (FontFamily::Serif, true, true),
        "F9" => (*family, *bold, *italic) = (FontFamily::Mono, false, false),
        "F10" => (*family, *bold, *italic) = (FontFamily::Mono, true, false),
        "F11" => (*family, *bold, *italic) = (FontFamily::Mono, false, true),
        "F12" => (*family, *bold, *italic) = (FontFamily::Mono, true, true),
        other => {
            let low = other.to_ascii_lowercase();
            *bold = low.contains("bold") || low == "b";
            *italic = low.contains("italic") || low.contains("oblique");
            *family = FontFamily::from_docx_name(&low);
        }
    }
}

#[derive(Debug, Clone)]
enum PdfToken {
    Num(f32),
    Name(String),
    Str(String),
    Array(Vec<PdfToken>),
    Op(String),
}

fn pop_num(stack: &mut Vec<PdfToken>) -> Option<f32> {
    match stack.pop()? {
        PdfToken::Num(n) => Some(n),
        _ => None,
    }
}

fn tokenize_pdf_content(data: &[u8]) -> Vec<PdfToken> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let b = data[pos];
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        if b == b'%' {
            while pos < data.len() && data[pos] != b'\n' && data[pos] != b'\r' {
                pos += 1;
            }
            continue;
        }
        if b == b'/' {
            pos += 1;
            let start = pos;
            while pos < data.len()
                && !data[pos].is_ascii_whitespace()
                && !b"()<>[]{}/%".contains(&data[pos])
            {
                pos += 1;
            }
            out.push(PdfToken::Name(
                String::from_utf8_lossy(&data[start..pos]).into_owned(),
            ));
            continue;
        }
        if b == b'(' {
            let (s, consumed) = parse_pdf_literal_string(&data[pos..]);
            pos += consumed;
            out.push(PdfToken::Str(s));
            continue;
        }
        if b == b'<' && pos + 1 < data.len() && data[pos + 1] != b'<' {
            pos += 1;
            let start = pos;
            while pos < data.len() && data[pos] != b'>' {
                pos += 1;
            }
            let hex_s = decode_pdf_hex_string(&data[start..pos]);
            if pos < data.len() {
                pos += 1;
            }
            out.push(PdfToken::Str(hex_s));
            continue;
        }
        if b == b'[' {
            pos += 1;
            let start = pos;
            let mut depth = 1usize;
            while pos < data.len() && depth > 0 {
                if data[pos] == b'(' {
                    let (_, c) = parse_pdf_literal_string(&data[pos..]);
                    pos += c;
                    continue;
                }
                if data[pos] == b'[' {
                    depth += 1;
                } else if data[pos] == b']' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                pos += 1;
            }
            let inner = tokenize_pdf_content(&data[start..pos]);
            if pos < data.len() {
                pos += 1;
            }
            out.push(PdfToken::Array(inner));
            continue;
        }
        if b.is_ascii_digit() || b == b'-' || b == b'+' || b == b'.' {
            let start = pos;
            pos += 1;
            while pos < data.len() && (data[pos].is_ascii_digit() || data[pos] == b'.') {
                pos += 1;
            }
            if let Ok(num) = std::str::from_utf8(&data[start..pos])
                .unwrap_or("")
                .parse::<f32>()
            {
                out.push(PdfToken::Num(num));
                continue;
            } else {
                out.push(PdfToken::Op(
                    String::from_utf8_lossy(&data[start..pos]).into_owned(),
                ));
                continue;
            }
        }

        let start = pos;
        while pos < data.len()
            && !data[pos].is_ascii_whitespace()
            && !b"()<>[]{}/%".contains(&data[pos])
        {
            pos += 1;
        }
        if pos > start {
            out.push(PdfToken::Op(
                String::from_utf8_lossy(&data[start..pos]).into_owned(),
            ));
        } else {
            pos += 1;
        }
    }
    out
}

fn parse_pdf_literal_string(data: &[u8]) -> (String, usize) {
    let mut pos = 1usize; // skip '('
    let mut depth = 1usize;
    let mut raw_bytes = Vec::new();

    while pos < data.len() && depth > 0 {
        let b = data[pos];
        if b == b'\\' && pos + 1 < data.len() {
            pos += 1;
            let esc = data[pos];
            match esc {
                b'n' => raw_bytes.push(b'\n'),
                b'r' => raw_bytes.push(b'\r'),
                b't' => raw_bytes.push(b'\t'),
                b'(' | b')' | b'\\' => raw_bytes.push(esc),
                b'0'..=b'7' => {
                    let mut oct = (esc - b'0') as u32;
                    for _ in 0..2 {
                        if pos + 1 < data.len() && (b'0'..=b'7').contains(&data[pos + 1]) {
                            pos += 1;
                            oct = oct * 8 + (data[pos] - b'0') as u32;
                        } else {
                            break;
                        }
                    }
                    raw_bytes.push((oct & 0xFF) as u8);
                }
                _ => raw_bytes.push(esc),
            }
            pos += 1;
            continue;
        }
        if b == b'(' {
            depth += 1;
            raw_bytes.push(b);
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                pos += 1;
                break;
            }
            raw_bytes.push(b);
        } else {
            raw_bytes.push(b);
        }
        pos += 1;
    }

    (winansi_bytes_to_utf8(&raw_bytes), pos)
}

fn decode_pdf_hex_string(hex: &[u8]) -> String {
    let clean: Vec<u8> = hex.iter().copied().filter(|b| b.is_ascii_hexdigit()).collect();
    let mut bytes = Vec::with_capacity(clean.len().div_ceil(2));
    let mut i = 0;
    while i < clean.len() {
        let hi = (clean[i] as char).to_digit(16).unwrap_or(0) as u8;
        let lo = if i + 1 < clean.len() {
            (clean[i + 1] as char).to_digit(16).unwrap_or(0) as u8
        } else {
            0
        };
        bytes.push((hi << 4) | lo);
        i += 2;
    }
    winansi_bytes_to_utf8(&bytes)
}

fn winansi_bytes_to_utf8(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let ch = match b {
            0x80 => '€',
            0x85 => '…',
            0x91 => '‘',
            0x92 => '’',
            0x93 => '“',
            0x94 => '”',
            0x95 => '•',
            0x96 => '–',
            0x97 => '—',
            0x20..=0x7E | 0xA0..=0xFF => b as char,
            b'\n' | b'\r' | b'\t' => ' ',
            _ => continue,
        };
        out.push(ch);
    }
    out
}
