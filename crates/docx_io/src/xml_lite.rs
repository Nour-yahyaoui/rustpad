//! Lightweight, standards-friendly pure-Rust XML parser & entity escaper for OpenXML.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct XmlNode {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<XmlChild>,
}

#[derive(Debug, Clone)]
pub enum XmlChild {
    Element(XmlNode),
    Text(String),
}

impl XmlNode {
    pub fn local_name(&self) -> &str {
        self.tag
            .rsplit_once(':')
            .map(|(_, local)| local)
            .unwrap_or(&self.tag)
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        if let Some(v) = self.attrs.get(key) {
            return Some(v.as_str());
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
        for c in &self.children {
            match c {
                XmlChild::Text(t) => out.push_str(t),
                XmlChild::Element(el) => out.push_str(&el.text_content()),
            }
        }
        out
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

pub const MAX_XML_DEPTH: usize = 128;

pub fn parse_xml(xml: &str) -> Option<XmlNode> {
    let mut pos = 0usize;
    let bytes = xml.as_bytes();
    // Skip BOM if present
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        pos = 3;
    }
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;

    while pos < bytes.len() {
        if let Some(rel_lt) = xml[pos..].find('<') {
            let text_slice = &xml[pos..pos + rel_lt];
            if !text_slice.is_empty() {
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
                    let cdata = &rest[9..end];
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlChild::Text(cdata.to_string()));
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
                if let Some(finished) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlChild::Element(finished));
                    } else {
                        root = Some(finished);
                    }
                }
            } else {
                let self_closing = inner.ends_with('/');
                let clean = if self_closing {
                    inner[..inner.len() - 1].trim()
                } else {
                    inner
                };
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

fn parse_tag_and_attrs(s: &str) -> (String, BTreeMap<String, String>) {
    let mut attrs = BTreeMap::new();
    let mut chars = s.char_indices().peekable();
    let mut end_name = s.len();
    for (i, ch) in s.char_indices() {
        if ch.is_whitespace() {
            end_name = i;
            break;
        }
    }
    let tag = s[..end_name].to_string();
    while let Some(&(i, _)) = chars.peek() {
        if i < end_name {
            chars.next();
        } else {
            break;
        }
    }

    let mut rest = s[end_name..].trim();
    while !rest.is_empty() {
        if let Some(eq_idx) = rest.find('=') {
            let key = rest[..eq_idx].trim().to_string();
            let after_eq = rest[eq_idx + 1..].trim_start();
            if let Some(quote) = after_eq.chars().next() {
                if quote == '"' || quote == '\'' {
                    let after_q = &after_eq[quote.len_utf8()..];
                    if let Some(close_q) = after_q.find(quote) {
                        let val = unescape_xml(&after_q[..close_q]);
                        attrs.insert(key, val);
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
