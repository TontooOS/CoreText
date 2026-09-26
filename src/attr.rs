//! Attributed strings and paragraph style.
//!
//! Covers the rich-text half of the request: mixed styles inside one
//! string (bold mid-sentence, colors, underline, strikethrough,
//! monospace, links), paragraph style (alignment, line height,
//! tracking, line break) and a markdown subset parser.

use std::ops::Range;

/// Horizontal alignment of a paragraph (mirrors `CTTextAlignment`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CTTextAlignment {
    #[default]
    Leading,
    Center,
    Trailing,
    Justified,
}

/// Line break strategy (mirrors `CTLineBreakMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CTLineBreakMode {
    /// Wrap at word boundaries (default).
    #[default]
    ByWordWrapping,
    /// Wrap mid-word when nothing else fits.
    ByCharWrapping,
    /// Single line, clip the tail.
    ByClipping,
    /// Single line, truncate with an ellipsis.
    ByTruncatingTail,
}

/// Paragraph style (mirrors `CTParagraphStyleRef`).
#[derive(Clone, Debug, PartialEq)]
pub struct CTParagraphStyle {
    pub alignment: CTTextAlignment,
    pub line_break: CTLineBreakMode,
    /// Multiplier over the font size (1.25 matches TontooUI).
    pub line_height: f32,
    /// Extra space between lines in logical px.
    pub line_spacing: f32,
    /// Letter spacing in logical px (tracking). Negative tightens.
    pub tracking: f32,
    /// Maximum lines; `None` means no limit.
    pub line_limit: Option<usize>,
}

impl Default for CTParagraphStyle {
    fn default() -> Self {
        Self {
            alignment: CTTextAlignment::Leading,
            line_break: CTLineBreakMode::ByWordWrapping,
            line_height: 1.25,
            line_spacing: 0.0,
            tracking: 0.0,
            line_limit: None,
        }
    }
}

impl CTParagraphStyle {
    pub fn alignment(mut self, alignment: CTTextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn line_height(mut self, value: f32) -> Self {
        self.line_height = value.max(0.5);
        self
    }

    pub fn line_spacing(mut self, value: f32) -> Self {
        self.line_spacing = value.max(0.0);
        self
    }

    pub fn tracking(mut self, value: f32) -> Self {
        self.tracking = value;
        self
    }

    pub fn line_break(mut self, mode: CTLineBreakMode) -> Self {
        self.line_break = mode;
        self
    }

    pub fn line_limit(mut self, lines: usize) -> Self {
        self.line_limit = Some(lines.max(1));
        self
    }
}

/// One inline style span: byte `range` into the plain string plus
/// style flags. Mirrors the attributes dictionary of
/// `CFAttributedStringRef`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttrSpan {
    pub range: Range<usize>,
    pub bold: bool,
    pub italic: bool,
    pub monospace: bool,
    /// Packed RGBA8 color override; `None` keeps the base color.
    pub color: Option<[u8; 4]>,
    pub underline: bool,
    pub underline_color: Option<[u8; 4]>,
    pub strikethrough: bool,
    pub strikethrough_color: Option<[u8; 4]>,
    pub link: Option<String>,
}

impl AttrSpan {
    pub fn new(range: Range<usize>) -> Self {
        Self {
            range,
            ..Self::default()
        }
    }
}

/// Attributed string: plain text plus inline spans (mirrors
/// `CFAttributedStringRef`).
#[derive(Clone, Debug, Default)]
pub struct AttributedString {
    text: String,
    spans: Vec<AttrSpan>,
}

impl AttributedString {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            spans: Vec::new(),
        }
    }

    /// Parse the markdown subset (`**bold**`, `*italic*`, `_italic_`,
    /// `***bold italic***`, `~~strike~~`, `` `code` ``, `[label](url)`,
    /// `\` escapes). Unmatched markers stay literal.
    pub fn markdown(source: impl Into<String>) -> Self {
        let (text, spans, _) = parse_markdown(&source.into());
        Self { text, spans }
    }

    pub fn push_span(&mut self, span: AttrSpan) {
        if span.range.start < span.range.end {
            self.spans.push(span);
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn spans(&self) -> &[AttrSpan] {
        &self.spans
    }

    /// Links as (byte range, url) pairs.
    pub fn links(&self) -> Vec<(Range<usize>, String)> {
        self.spans
            .iter()
            .filter_map(|s| s.link.clone().map(|url| (s.range.clone(), url)))
            .collect()
    }
}

#[derive(Clone, Copy, Default)]
struct Flags {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    out: String,
    out_spans: Vec<(usize, usize, Flags, Option<String>)>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            bytes: source.as_bytes(),
            pos: 0,
            out: String::new(),
            out_spans: Vec::new(),
        }
    }

    fn peek(&self, token: &str) -> bool {
        self.bytes[self.pos..].starts_with(token.as_bytes())
    }

    fn emit(&mut self, text: &str, flags: Flags, link: Option<String>) {
        let start = self.out.len();
        self.out.push_str(text);
        let end = self.out.len();
        if flags.bold || flags.italic || flags.strike || flags.code || link.is_some() {
            self.out_spans.push((start, end, flags, link));
        }
    }

    fn parse_until(&mut self, closer: &str, flags: Flags) -> bool {
        let start_out = self.out.len();
        let start_spans = self.out_spans.len();
        let mut closed = false;
        while self.pos < self.bytes.len() {
            if self.peek(closer) {
                self.pos += closer.len();
                closed = true;
                break;
            }
            if closer != "`" && self.try_inline(flags) {
                continue;
            }
            if self.peek("\\") && self.pos + 1 < self.bytes.len() {
                let ch = self.bytes[self.pos + 1] as char;
                self.emit(&ch.to_string(), flags, None);
                self.pos += 2;
                continue;
            }
            let ch_len = self.bytes[self.pos..]
                .iter()
                .position(|_| true)
                .map(|_| {
                    self.bytes[self.pos..]
                        .windows(1)
                        .next()
                        .map(|_| 1)
                        .unwrap_or(1)
                })
                .unwrap_or(1);
            // Decode one UTF-8 char safely.
            let rest = &self.bytes[self.pos..];
            let ch = std::str::from_utf8(rest)
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or('\u{FFFD}');
            self.emit(&ch.to_string(), flags, None);
            self.pos += ch.len_utf8().max(ch_len.min(ch.len_utf8()));
            if self.pos >= self.bytes.len() {
                break;
            }
        }
        if !closed {
            // Roll back: keep the literal opener instead.
            self.out.truncate(start_out);
            self.out_spans.truncate(start_spans);
        }
        closed
    }

    fn try_inline(&mut self, outer: Flags) -> bool {
        if self.peek("***") {
            self.pos += 3;
            let mut flags = outer;
            flags.bold = true;
            flags.italic = true;
            if self.parse_until("***", flags) {
                return true;
            }
            self.pos -= 3;
            return false;
        }
        if self.peek("**") {
            self.pos += 2;
            let mut flags = outer;
            flags.bold = true;
            if self.parse_until("**", flags) {
                return true;
            }
            self.pos -= 2;
            return false;
        }
        if self.peek("~~") {
            self.pos += 2;
            let mut flags = outer;
            flags.strike = true;
            if self.parse_until("~~", flags) {
                return true;
            }
            self.pos -= 2;
            return false;
        }
        if self.peek("`") {
            self.pos += 1;
            let mut flags = outer;
            flags.code = true;
            if self.parse_until("`", flags) {
                return true;
            }
            self.pos -= 1;
            return false;
        }
        if self.bytes.get(self.pos) == Some(&b'[') {
            return self.try_link(outer);
        }
        if self.peek("*") {
            self.pos += 1;
            let mut flags = outer;
            flags.italic = true;
            if self.parse_until("*", flags) {
                return true;
            }
            self.pos -= 1;
            return false;
        }
        if self.bytes.get(self.pos) == Some(&b'_') && flanks_word(self.bytes, self.pos) {
            self.pos += 1;
            let mut flags = outer;
            flags.italic = true;
            if self.parse_until("_", flags) {
                return true;
            }
            self.pos -= 1;
            return false;
        }
        false
    }

    fn try_link(&mut self, outer: Flags) -> bool {
        let rest = &self.bytes[self.pos..];
        let close_label = rest.iter().position(|&b| b == b']');
        let Some(close_label) = close_label else {
            return false;
        };
        let after = self.pos + close_label + 1;
        if self.bytes.get(after) != Some(&b'(') {
            return false;
        }
        let tail = &self.bytes[after + 1..];
        let close_url = tail.iter().position(|&b| b == b')');
        let Some(close_url) = close_url else {
            return false;
        };
        let label = String::from_utf8_lossy(&self.bytes[self.pos + 1..self.pos + close_label]);
        let url = String::from_utf8_lossy(&self.bytes[after + 1..after + 1 + close_url]).to_string();
        if url.is_empty() {
            return false;
        }
        self.pos = after + 1 + close_url + 1;
        // Parse the label with the outer flags so `[*x*](u)` keeps italic.
        let label_owned = label.to_string();
        let mut label_parser = Parser::new(&label_owned);
        // Reuse the same opener logic on the label slice.
        while label_parser.pos < label_parser.bytes.len() {
            if label_parser.try_inline(outer) {
                continue;
            }
            let rest = &label_parser.bytes[label_parser.pos..];
            let ch = std::str::from_utf8(rest)
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or('\u{FFFD}');
            label_parser.emit(&ch.to_string(), outer, None);
            label_parser.pos += ch.len_utf8();
        }
        let base = self.out.len();
        for (s, e, flags, _) in &label_parser.out_spans {
            let text = label_parser.out[*s..*e].to_string();
            let start = self.out.len();
            self.out.push_str(&text);
            let end = self.out.len();
            self.out_spans.push((start, end, *flags, Some(url.clone())));
        }
        if label_parser.out_spans.is_empty() {
            let start = self.out.len();
            self.out.push_str(&label_parser.out);
            let end = self.out.len();
            let _ = base;
            self.out_spans.push((start, end, outer, Some(url)));
        }
        true
    }
}

fn flanks_word(bytes: &[u8], pos: usize) -> bool {
    let prev = pos.checked_sub(1).and_then(|i| bytes.get(i)).copied();
    let next = bytes.get(pos + 1).copied();
    let is_word = |b: Option<u8>| matches!(b, Some(b) if (b as char).is_alphanumeric() || b == b'_');
    is_word(prev) || is_word(next)
}

/// Parse markdown into (plain text, spans, links).
pub fn parse_markdown(source: &str) -> (String, Vec<AttrSpan>, Vec<(Range<usize>, String)>) {
    let mut parser = Parser::new(source);
    let flags = Flags::default();
    while parser.pos < parser.bytes.len() {
        if parser.peek("\\") && parser.pos + 1 < parser.bytes.len() {
            let ch = parser.bytes[parser.pos + 1] as char;
            parser.emit(&ch.to_string(), flags, None);
            parser.pos += 2;
            continue;
        }
        if parser.try_inline(flags) {
            continue;
        }
        let rest = &parser.bytes[parser.pos..];
        let ch = std::str::from_utf8(rest)
            .ok()
            .and_then(|s| s.chars().next())
            .unwrap_or('\u{FFFD}');
        parser.emit(&ch.to_string(), flags, None);
        parser.pos += ch.len_utf8();
    }
    let mut spans = Vec::with_capacity(parser.out_spans.len());
    let mut links = Vec::new();
    for (start, end, flags, link) in parser.out_spans {
        let mut span = AttrSpan::new(start..end);
        span.bold = flags.bold;
        span.italic = flags.italic;
        span.strikethrough = flags.strike;
        span.monospace = flags.code;
        span.link = link.clone();
        if let Some(url) = link {
            links.push((start..end, url));
        }
        spans.push(span);
    }
    (parser.out, spans, links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_bold_and_italic() {
        let (text, spans, _) = parse_markdown("**bold** and *italic*");
        assert_eq!(text, "bold and italic");
        assert!(spans[0].bold);
        assert!(spans.iter().any(|s| s.italic));
    }

    #[test]
    fn markdown_link_keeps_label() {
        let (text, _, links) = parse_markdown("[Label](https://example.com)");
        assert_eq!(text, "Label");
        assert_eq!(links[0].1, "https://example.com");
    }

    #[test]
    fn paragraph_defaults_match_tontooui() {
        let style = CTParagraphStyle::default();
        assert_eq!(style.line_height, 1.25);
        assert_eq!(style.alignment, CTTextAlignment::Leading);
    }
}
