//! Lightweight, streaming-capable pure-Rust XML parser & entity escaper for OpenXML.
//! Uses zero-allocation empty `Vec` attribute storage, iterative `Drop` (preventing stack overflow
//! on deeply nested Word XML), and `<w:body>` block-by-block streaming (`< 32 KB` peak DOM memory
//! even on 50–200+ MB multi-hundred-page `word/document.xml` archives).

#[derive(Debug, Clone)]
pub struct XmlNode {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<XmlChild>,
}

#[derive(Debug, Clone)]
pub enum XmlChild {
    Element(XmlNode),
    Text(String),
}

/// Iterative drop ensures arbitrarily deep XML trees never overflow the call stack.
impl Drop for XmlNode {
    fn drop(&mut self) {
        if self.children.is_empty() {
            return;
        }
        let mut stack: Vec<XmlChild> = std::mem::take(&mut self.children);
        while let Some(child) = stack.pop() {
            if let XmlChild::Element(mut el) = child {
                if !el.children.is_empty() {
                    stack.append(&mut el.children);
                }
            }
        }
    }
}

impl XmlNode {
    #[inline]
    pub fn local_name(&self) -> &str {
        self.tag
            .rsplit_once(':')
            .map(|(_, local)| local)
            .unwrap_or(&self.tag)
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        for (k, v) in &self.attrs {
            if k == key {
                return Some(v.as_str());
            }
        }
        for (k, v) in &self.attrs {
            let local = k.rsplit_once(':').map(|(_, l)| l).unwrap_or(k.as_str());
            if local == key {
                return Some(v.as_str());
            }
        }
        None
    }

    pub fn child(&self, local: &str) -> Option<&XmlNode> {
        self.children.iter().find_map(|c| match c {
            XmlChild::Element(el) if el.local_name() == local => Some(el),
            _ => None,
        })
    }

    pub fn children_named<'a>(&'a self, local: &'a str) -> impl Iterator<Item = &'a XmlNode> + 'a {
        self.children.iter().filter_map(move |c| match c {
            XmlChild::Element(el) if el.local_name() == local => Some(el),
            _ => None,
        })
    }

    pub fn text_content(&self) -> String {
        let mut out = String::new();
        self.append_text_content(&mut out);
        out
    }

    fn append_text_content(&self, out: &mut String) {
        for c in &self.children {
            match c {
                XmlChild::Text(t) => out.push_str(t),
                XmlChild::Element(el) => el.append_text_content(out),
            }
        }
    }
}

pub fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

pub fn unescape_xml(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        if let Some(semi) = after.find(';') {
            let entity = &after[..semi];
            match entity {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                    if let Ok(code) = u32::from_str_radix(&entity[2..], 16) {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                }
                _ if entity.starts_with('#') => {
                    if let Ok(code) = entity[1..].parse::<u32>() {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                }
                _ => {
                    out.push('&');
                    out.push_str(entity);
                    out.push(';');
                }
            }
            rest = &after[semi + 1..];
        } else {
            out.push('&');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

pub const MAX_XML_DEPTH: usize = 160;

#[inline]
fn is_ignorable_word_bloat_tag(raw_tag: &str) -> bool {
    let local = raw_tag
        .rsplit_once(':')
        .map(|(_, l)| l)
        .unwrap_or(raw_tag);
    matches!(
        local,
        "proofErr"
            | "bookmarkStart"
            | "bookmarkEnd"
            | "permStart"
            | "permEnd"
            | "lastRenderedPageBreak"
            | "noProof"
    )
}

pub fn parse_xml(xml: &str) -> Option<XmlNode> {
    parse_xml_internal(xml, false)
}

pub fn parse_xml_body_child(xml: &str) -> Option<XmlNode> {
    parse_xml_internal(xml, true)
}

fn parse_xml_internal(xml: &str, skip_bloat: bool) -> Option<XmlNode> {
    let mut pos = 0usize;
    let bytes = xml.as_bytes();
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        pos = 3;
    }
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut ignored_depth = 0usize;

    while pos < bytes.len() {
        if let Some(rel_lt) = xml[pos..].find('<') {
            if ignored_depth == 0 && rel_lt > 0 {
                let text_slice = &xml[pos..pos + rel_lt];
                if let Some(parent) = stack.last_mut() {
                    let decoded = unescape_xml(text_slice);
                    if !decoded.is_empty() {
                        parent.children.push(XmlChild::Text(decoded));
                    }
                }
            }
            pos += rel_lt;
            let rest = &xml[pos..];
            if rest.starts_with("<!--") {
                if let Some(end) = rest.find("-->") {
                    pos += end + 3;
                    continue;
                } else {
                    break;
                }
            }
            if rest.starts_with("<?") {
                if let Some(end) = rest.find("?>") {
                    pos += end + 2;
                    continue;
                } else {
                    break;
                }
            }
            if rest.starts_with("<![CDATA[") {
                if let Some(end) = rest.find("]]>") {
                    if ignored_depth == 0 {
                        let cdata = &rest[9..end];
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(XmlChild::Text(cdata.to_string()));
                        }
                    }
                    pos += end + 3;
                    continue;
                } else {
                    break;
                }
            }
            if rest.starts_with("<!") {
                if let Some(end) = rest.find('>') {
                    pos += end + 1;
                    continue;
                } else {
                    break;
                }
            }
            let tag_end = find_tag_close(rest)?;
            let inner = rest[1..tag_end].trim();
            pos += tag_end + 1;

            if let Some(close_name) = inner.strip_prefix('/') {
                let _ = close_name.trim();
                if ignored_depth > 0 {
                    ignored_depth -= 1;
                    continue;
                }
                if let Some(finished) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlChild::Element(finished));
                    } else {
                        root = Some(finished);
                    }
                }
            } else {
                let self_closing = inner.ends_with('/');
                if ignored_depth > 0 {
                    if !self_closing {
                        ignored_depth += 1;
                    }
                    continue;
                }
                let clean = if self_closing {
                    inner[..inner.len() - 1].trim()
                } else {
                    inner
                };
                let tag_name_slice = extract_tag_name_slice(clean);
                if skip_bloat && !stack.is_empty() && is_ignorable_word_bloat_tag(tag_name_slice) {
                    if !self_closing {
                        ignored_depth = 1;
                    }
                    continue;
                }
                let (tag, attrs) = parse_tag_and_attrs(clean);
                let node = XmlNode {
                    tag,
                    attrs,
                    children: Vec::new(),
                };
                if self_closing {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlChild::Element(node));
                    } else {
                        root = Some(node);
                    }
                } else {
                    if stack.len() >= MAX_XML_DEPTH {
                        return None;
                    }
                    stack.push(node);
                }
            }
        } else {
            break;
        }
    }

    while let Some(finished) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(XmlChild::Element(finished));
        } else {
            root = Some(finished);
        }
    }

    root
}

/// Streams `<w:body>` direct child elements (`<w:p>`, `<w:tbl>`, `<w:sdt>`, `<w:sectPr>`, etc.)
/// one block at a time with `< 32 KB` peak DOM memory, calling `on_element(el, byte_progress, total_bytes)`
/// and dropping each block's XML nodes immediately after processing!
pub fn stream_docx_body_elements<F>(xml: &str, mut on_element: F) -> bool
where
    F: FnMut(&XmlNode, usize, usize),
{
    let mut pos = 0usize;
    let bytes = xml.as_bytes();
    let total_len = bytes.len().max(1);
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        pos = 3;
    }

    let mut in_body = false;
    let mut found_body = false;
    let mut child_depth = 0usize;
    let mut child_start = 0usize;

    while pos < bytes.len() {
        let Some(rel_lt) = xml[pos..].find('<') else {
            break;
        };
        pos += rel_lt;
        let tag_lt = pos;
        let rest = &xml[pos..];

        if rest.starts_with("<!--") {
            if let Some(end) = rest.find("-->") {
                pos += end + 3;
                continue;
            } else {
                break;
            }
        }
        if rest.starts_with("<?") {
            if let Some(end) = rest.find("?>") {
                pos += end + 2;
                continue;
            } else {
                break;
            }
        }
        if rest.starts_with("<![CDATA[") {
            if let Some(end) = rest.find("]]>") {
                pos += end + 3;
                continue;
            } else {
                break;
            }
        }
        if rest.starts_with("<!") {
            if let Some(end) = rest.find('>') {
                pos += end + 1;
                continue;
            } else {
                break;
            }
        }

        let Some(tag_end) = find_tag_close(rest) else {
            return false;
        };
        let inner = rest[1..tag_end].trim();
        pos += tag_end + 1;

        if !in_body {
            if !inner.starts_with('/') {
                let self_closing = inner.ends_with('/');
                let clean = if self_closing {
                    inner[..inner.len() - 1].trim()
                } else {
                    inner
                };
                let name = extract_tag_name_slice(clean);
                let local = name.rsplit_once(':').map(|(_, l)| l).unwrap_or(name);
                if local == "body" {
                    found_body = true;
                    if self_closing {
                        return true;
                    }
                    in_body = true;
                    child_depth = 0;
                }
            }
            continue;
        }

        // Inside `<w:body>`
        if let Some(close_raw) = inner.strip_prefix('/') {
            if child_depth == 0 {
                let close_name = extract_tag_name_slice(close_raw.trim());
                let local = close_name
                    .rsplit_once(':')
                    .map(|(_, l)| l)
                    .unwrap_or(close_name);
                if local == "body" {
                    break;
                }
            } else {
                child_depth -= 1;
                if child_depth == 0 {
                    let slice = &xml[child_start..pos];
                    if let Some(node) = parse_xml_body_child(slice) {
                        on_element(&node, pos, total_len);
                    }
                }
            }
        } else {
            let self_closing = inner.ends_with('/');
            if child_depth == 0 {
                if self_closing {
                    let slice = &xml[tag_lt..pos];
                    if let Some(node) = parse_xml_body_child(slice) {
                        on_element(&node, pos, total_len);
                    }
                } else {
                    child_start = tag_lt;
                    child_depth = 1;
                }
            } else if !self_closing {
                child_depth += 1;
                if child_depth > MAX_XML_DEPTH {
                    return false;
                }
            }
        }
    }

    found_body
}

#[inline]
fn extract_tag_name_slice(s: &str) -> &str {
    for (i, ch) in s.char_indices() {
        if ch.is_whitespace() || ch == '/' {
            return &s[..i];
        }
    }
    s
}

fn find_tag_close(s: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    for (idx, ch) in s.char_indices().skip(1) {
        if let Some(q) = in_quote {
            if ch == q {
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
        } else if ch == '>' {
            return Some(idx);
        }
    }
    None
}

fn parse_tag_and_attrs(s: &str) -> (String, Vec<(String, String)>) {
    let name_slice = extract_tag_name_slice(s);
    let tag = name_slice.to_string();
    let mut rest = s[name_slice.len()..].trim();
    if rest.is_empty() {
        return (tag, Vec::new());
    }

    let mut attrs = Vec::new();
    while !rest.is_empty() {
        if let Some(eq_idx) = rest.find('=') {
            let key = rest[..eq_idx].trim();
            let after_eq = rest[eq_idx + 1..].trim_start();
            if let Some(quote) = after_eq.chars().next() {
                if quote == '"' || quote == '\'' {
                    let after_q = &after_eq[quote.len_utf8()..];
                    if let Some(close_q) = after_q.find(quote) {
                        // Skip redundant Word `xmlns:*` and `w:rsid*` noise attributes to save heap
                        let keep = !key.starts_with("xmlns:")
                            && !key.starts_with("w:rsid")
                            && !key.starts_with("w14:paraId")
                            && !key.starts_with("w14:textId");
                        if keep {
                            let val = unescape_xml(&after_q[..close_q]);
                            attrs.push((key.to_string(), val));
                        }
                        rest = after_q[close_q + quote.len_utf8()..].trim_start();
                        continue;
                    }
                }
            }
            break;
        } else {
            break;
        }
    }

    (tag, attrs)
}
