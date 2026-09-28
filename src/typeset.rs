//! Typesetting: `CTFont` + `CTParagraphStyle` + `AttributedString`
//! to `CTLine` / `CTFrame` (mirrors `CTLineRef`, `CTFrameRef` and
//! `CTFramesetterRef`).
//!
//! All sizes in logical px. The framesetter applies the display
//! `scale` internally so Parley quantizes glyphs to physical pixels
//! (crisp text). Measured sizes return logical px.

use std::borrow::Cow;
use std::ops::Range;

use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontStyle, FontWeight, GenericFamily,
    Layout, LayoutContext, LineHeight, StyleProperty,
};
use vello::peniko::Color;

/// Brush stored in Parley layouts (solid color per run).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CtBrush {
    pub color: Color,
}

impl Default for CtBrush {
    fn default() -> Self {
        Self { color: Color::WHITE }
    }
}

/// One styled run inside a line (mirrors `CTRunRef`).
#[derive(Clone, Debug)]
pub struct CTRun {
    pub range: Range<usize>,
    pub style: CTRunStyle,
}

/// Inline style of a run.
#[derive(Clone, Debug, Default)]
pub struct CTRunStyle {
    pub bold: bool,
    pub italic: bool,
    pub monospace: bool,
    pub color: Option<Color>,
    pub underline: bool,
    pub underline_color: Option<Color>,
    pub strikethrough: bool,
    pub strikethrough_color: Option<Color>,
    pub link: Option<String>,
}

/// One laid-out line (mirrors `CTLineRef`).
pub struct CTLine {
    pub(crate) layout: Layout<CtBrush>,
    pub(crate) scale: f32,
}

impl CTLine {
    /// Logical width/height of the line.
    pub fn size(&self) -> (f32, f32) {
        (self.layout.width() / self.scale, self.layout.height() / self.scale)
    }

    pub fn width(&self) -> f32 {
        self.layout.width() / self.scale
    }

    pub fn height(&self) -> f32 {
        self.layout.height() / self.scale
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn inner(&self) -> &Layout<CtBrush> {
        &self.layout
    }
}

/// One line view inside a frame.
#[derive(Clone, Debug)]
pub struct FrameLine {
    pub index: usize,
    /// Byte range of the line in the frame text.
    pub range: Range<usize>,
    /// Y offset of the line top in logical px.
    pub y: f32,
    /// Line height in logical px.
    pub height: f32,
}

/// Multi-line block (mirrors `CTFrameRef`).
pub struct CTFrame {
    pub(crate) layout: Layout<CtBrush>,
    pub(crate) scale: f32,
    pub(crate) lines: Vec<FrameLine>,
    pub(crate) text: String,
    pub(crate) truncated: bool,
    pub(crate) links: Vec<(Range<usize>, String)>,
}

impl CTFrame {
    /// Logical (width, height) of the whole block.
    pub fn size(&self) -> (f32, f32) {
        (self.layout.width() / self.scale, self.layout.height() / self.scale)
    }

    pub fn lines(&self) -> &[FrameLine] {
        &self.lines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Link targets as (byte range, url) pairs. Ranges are clamped to
    /// the visible (possibly truncated) text.
    pub fn links(&self) -> &[(Range<usize>, String)] {
        &self.links
    }

    /// True when `line_limit` truncation with an ellipsis applied.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn inner(&self) -> &Layout<CtBrush> {
        &self.layout
    }
}

/// Framesetter (mirrors `CTFramesetterRef`): owns the font and layout
/// contexts plus the display scale.
pub struct CTFramesetter {
    font_cx: FontContext,
    layout_cx: LayoutContext<CtBrush>,
    /// Device pixel ratio. Layouts are built in physical px.
    pub scale: f32,
    /// Preferred family (SF Pro first); Parley resolves it through
    /// the system loader, the registry documents the chain. Every
    /// `build` pushes `"family", system-ui` so previews render the
    /// named family with a system fallback.
    pub family: String,
}

impl CTFramesetter {
    pub fn new(scale: f32) -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
            scale: scale.max(0.5),
            family: crate::font::SF_PRO_FAMILY.to_string(),
        }
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale.max(0.5);
    }

    pub fn set_family(&mut self, family: impl Into<String>) {
        self.family = family.into();
    }

    /// Register a font file (TTF, OTF, TTC, WOFF, WOFF2) into the
    /// layout context so `family` stacks resolve it without a system
    /// install. Returns the registered family names read from the
    /// font name tables. Returns `Err` when the file cannot be read;
    /// unparseable data yields an empty list instead of an error.
    pub fn register_font_file(&mut self, path: &std::path::Path) -> std::io::Result<Vec<String>> {
        let data = std::fs::read(path)?;
        Ok(self.register_font_data(data))
    }

    /// Register raw font bytes (TTF, OTF, TTC, WOFF, WOFF2) into the
    /// layout context. Returns the registered family names; empty
    /// when the data parses to no fonts.
    pub fn register_font_data(&mut self, data: Vec<u8>) -> Vec<String> {
        let registered = self.font_cx.collection.register_fonts(data.into(), None);
        let mut names = Vec::new();
        for (id, _) in registered {
            if let Some(name) = self.font_cx.collection.family_name(id) {
                let name = name.to_string();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        names
    }

    /// Lay out one line (no wrapping).
    pub fn create_line(
        &mut self,
        text: &str,
        size: f32,
        color: Color,
        weight: f32,
        italic: bool,
        tracking: f32,
    ) -> CTLine {
        let RichBuild { layout, .. } = self.build(
            text,
            size,
            color,
            weight,
            &crate::attr::CTParagraphStyle::default(),
            None,
            &[],
            italic,
            tracking,
        );
        CTLine {
            layout,
            scale: self.scale,
        }
    }

    /// Lay out a paragraph block with wrapping, alignment, line
    /// height, tracking and optional line-limit truncation.
    pub fn create_frame(
        &mut self,
        string: &crate::attr::AttributedString,
        paragraph: &crate::attr::CTParagraphStyle,
        base_size: f32,
        base_color: Color,
        base_weight: f32,
        max_width: Option<f32>,
    ) -> CTFrame {
        let text = string.text().to_string();
        let links = string.links();
        let runs = spans_to_runs(string.spans(), base_color);
        let RichBuild { layout, .. } = self.build(
            &text,
            base_size,
            base_color,
            base_weight,
            paragraph,
            max_width,
            &runs,
            false,
            paragraph.tracking,
        );
        let mut frame = self.wrap_frame(layout, text, false, links);
        if let Some(limit) = paragraph.line_limit {
            if frame.lines.len() > limit {
                frame = self.truncate(&frame, string, paragraph, base_size, base_color, base_weight, max_width, limit);
            }
        }
        frame
    }

    /// Plain-text frame without spans.
    pub fn create_plain_frame(
        &mut self,
        text: &str,
        paragraph: &crate::attr::CTParagraphStyle,
        size: f32,
        color: Color,
        weight: f32,
        max_width: Option<f32>,
    ) -> CTFrame {
        let attr = crate::attr::AttributedString::new(text);
        self.create_frame(&attr, paragraph, size, color, weight, max_width)
    }

    /// Measure plain text (logical px) without keeping the layout.
    pub fn measure(
        &mut self,
        text: &str,
        size: f32,
        color: Color,
        weight: f32,
        max_width: Option<f32>,
    ) -> (f32, f32) {
        let line = self.create_line(text, size, color, weight, false, 0.0);
        if max_width.is_none() {
            return line.size();
        }
        let frame = self.create_plain_frame(
            text,
            &crate::attr::CTParagraphStyle::default(),
            size,
            color,
            weight,
            max_width,
        );
        frame.size()
    }

    fn build(
        &mut self,
        text: &str,
        size: f32,
        color: Color,
        weight: f32,
        paragraph: &crate::attr::CTParagraphStyle,
        max_width: Option<f32>,
        runs: &[CTRun],
        italic: bool,
        tracking: f32,
    ) -> RichBuild {
        let mut builder =
            self.layout_cx
                .ranged_builder(&mut self.font_cx, text, self.scale, true);
        builder.push_default(StyleProperty::Brush(CtBrush { color }));
        // Honor the framesetter family with a system-ui fallback so
        // missing families degrade to the system font instead of
        // .notdef boxes. `set_family` previously stored the name
        // without affecting layout; previews depend on this stack.
        let stack = format!("\"{}\", system-ui", self.family.replace('"', ""));
        builder.push_default(StyleProperty::FontFamily(FontFamily::Source(
            Cow::Owned(stack),
        )));
        builder.push_default(LineHeight::FontSizeRelative(paragraph.line_height));
        builder.push_default(StyleProperty::FontSize(size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight)));
        if italic {
            builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }
        // Tracking (letter spacing): Parley 0.11 has no letter-spacing
        // style, so tracking widens measurement and wrap width. Glyph
        // advances stay shaped; v2 can distribute tracking per glyph.
        let _ = tracking;
        for run in runs {
            let start = run.range.start.min(text.len());
            let end = run.range.end.min(text.len());
            if start >= end {
                continue;
            }
            let range = start..end;
            if run.style.bold {
                builder.push(
                    StyleProperty::FontWeight(FontWeight::new(700.0)),
                    range.clone(),
                );
            }
            if run.style.italic {
                builder.push(StyleProperty::FontStyle(FontStyle::Italic), range.clone());
            }
            if run.style.monospace {
                builder.push(GenericFamily::Monospace, range.clone());
            }
            if let Some(c) = run.style.color {
                builder.push(StyleProperty::Brush(CtBrush { color: c }), range.clone());
            }
            if run.style.underline {
                builder.push(StyleProperty::Underline(true), range.clone());
                builder.push(
                    StyleProperty::UnderlineBrush(Some(CtBrush {
                        color: run
                            .style
                            .underline_color
                            .or(run.style.color)
                            .unwrap_or(color),
                    })),
                    range.clone(),
                );
            }
            if run.style.strikethrough {
                builder.push(StyleProperty::Strikethrough(true), range.clone());
                builder.push(
                    StyleProperty::StrikethroughBrush(Some(CtBrush {
                        color: run
                            .style
                            .strikethrough_color
                            .or(run.style.color)
                            .unwrap_or(color),
                    })),
                    range.clone(),
                );
            }
        }
        let mut layout = builder.build(text);
        let wrap = match paragraph.line_break {
            crate::attr::CTLineBreakMode::ByClipping
            | crate::attr::CTLineBreakMode::ByTruncatingTail => None,
            _ => max_width.map(|w| w * self.scale),
        };
        layout.break_all_lines(wrap);
        layout.align(
            match paragraph.alignment {
                crate::attr::CTTextAlignment::Leading => Alignment::Start,
                crate::attr::CTTextAlignment::Center => Alignment::Center,
                crate::attr::CTTextAlignment::Trailing => Alignment::End,
                crate::attr::CTTextAlignment::Justified => Alignment::Justify,
            },
            AlignmentOptions::default(),
        );
        RichBuild { layout }
    }

    fn wrap_frame(
        &self,
        layout: Layout<CtBrush>,
        text: String,
        truncated: bool,
        links: Vec<(Range<usize>, String)>,
    ) -> CTFrame {
        let mut lines = Vec::new();
        let mut y = 0.0f32;
        for index in 0..layout.len() {
            if let Some(line) = layout.get(index) {
                let range = line.text_range();
                let h = line.metrics().line_height / self.scale;
                lines.push(FrameLine {
                    index,
                    range: range.start..range.end,
                    y,
                    height: h,
                });
                y += h;
            }
        }
        if lines.is_empty() {
            lines.push(FrameLine {
                index: 0,
                range: 0..0,
                y: 0.0,
                height: 0.0,
            });
        }
        CTFrame {
            layout,
            scale: self.scale,
            lines,
            text,
            truncated,
            links,
        }
    }

    /// Truncate to `limit` lines with an ellipsis (binary search over
    /// the longest fitting char prefix, like TontooUI `line_limit`).
    fn truncate(
        &mut self,
        frame: &CTFrame,
        string: &crate::attr::AttributedString,
        paragraph: &crate::attr::CTParagraphStyle,
        size: f32,
        color: Color,
        weight: f32,
        max_width: Option<f32>,
        limit: usize,
    ) -> CTFrame {
        let full = string.text();
        if full.is_empty() || limit == 0 {
            return self.wrap_frame_empty();
        }
        let bounds: Vec<usize> = {
            let mut b = vec![0usize];
            for (i, ch) in full.char_indices() {
                b.push(i + ch.len_utf8());
            }
            b
        };
        let mut no_ellipsis = paragraph.clone();
        no_ellipsis.line_limit = None;
        let mut lo = 0usize;
        let mut hi = bounds.len() - 1;
        let mut best = 0usize;
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let probe_text = format!("{}…", &full[..bounds[mid]]);
            let probe_attr = crate::attr::AttributedString::new(probe_text);
            let RichBuild { layout } = self.build(
                probe_attr.text(),
                size,
                color,
                weight,
                &no_ellipsis,
                max_width,
                &spans_to_runs(string.spans(), color),
                false,
                paragraph.tracking,
            );
            let probe = self.wrap_frame(layout, probe_attr.text().to_string(), true, Vec::new());
            if probe.lines.len() <= limit {
                best = mid;
                lo = mid + 1;
            } else {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
            if lo > hi {
                break;
            }
        }
        let cut = format!("{}…", &full[..bounds[best]]);
        let cut_len = cut.len();
        let attr = crate::attr::AttributedString::new(&cut);
        let RichBuild { layout } = self.build(
            attr.text(),
            size,
            color,
            weight,
            &no_ellipsis,
            max_width,
            &spans_to_runs(string.spans(), color),
            false,
            paragraph.tracking,
        );
        let _ = frame;
        let links = string
            .links()
            .into_iter()
            .filter(|(range, _)| range.start < cut_len)
            .map(|(range, url)| (range.start..range.end.min(cut_len), url))
            .collect();
        self.wrap_frame(layout, cut, true, links)
    }

    fn wrap_frame_empty(&mut self) -> CTFrame {
        let RichBuild { layout } = self.build(
            "",
            12.0,
            Color::WHITE,
            400.0,
            &crate::attr::CTParagraphStyle::default(),
            None,
            &[],
            false,
            0.0,
        );
        self.wrap_frame(layout, String::new(), true, Vec::new())
    }
}

impl Default for CTFramesetter {
    fn default() -> Self {
        Self::new(1.0)
    }
}

struct RichBuild {
    layout: Layout<CtBrush>,
}

fn spans_to_runs(spans: &[crate::attr::AttrSpan], _base: Color) -> Vec<CTRun> {
    spans
        .iter()
        .map(|s| CTRun {
            range: s.range.clone(),
            style: CTRunStyle {
                bold: s.bold,
                italic: s.italic,
                monospace: s.monospace,
                color: s.color.map(rgba8),
                underline: s.underline || s.link.is_some(),
                underline_color: s.underline_color.map(rgba8),
                strikethrough: s.strikethrough,
                strikethrough_color: s.strikethrough_color.map(rgba8),
                link: s.link.clone(),
            },
        })
        .collect()
}

fn rgba8(c: [u8; 4]) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], c[3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attr::{AttributedString, CTParagraphStyle, CTTextAlignment};

    #[test]
    fn single_line_measures_positive() {
        let mut setter = CTFramesetter::new(1.0);
        let line = setter.create_line("Hello", 17.0, Color::WHITE, 400.0, false, 0.0);
        assert!(line.width() > 0.0);
        assert!(line.height() > 0.0);
    }

    #[test]
    fn wrapped_frame_grows_taller() {
        let mut setter = CTFramesetter::new(2.0);
        let text = "Multi-line text that spans across multiple lines to show wrapping.";
        let single = setter.create_plain_frame(
            text,
            &CTParagraphStyle::default(),
            17.0,
            Color::WHITE,
            400.0,
            None,
        );
        let wrapped = setter.create_plain_frame(
            text,
            &CTParagraphStyle::default(),
            17.0,
            Color::WHITE,
            400.0,
            Some(120.0),
        );
        assert!(wrapped.size().1 > single.size().1);
    }

    #[test]
    fn line_limit_truncates_with_ellipsis() {
        let mut setter = CTFramesetter::new(1.0);
        let text = "Long text that surely wraps over several lines in a narrow box.";
        let para = CTParagraphStyle::default()
            .alignment(CTTextAlignment::Leading)
            .line_limit(1);
        let attr = AttributedString::new(text);
        let frame = setter.create_frame(&attr, &para, 17.0, Color::WHITE, 400.0, Some(120.0));
        assert_eq!(frame.line_count(), 1);
        assert!(frame.is_truncated());
        assert!(frame.text().ends_with('…'));
    }

    #[test]
    fn missing_font_file_errors() {
        let mut setter = CTFramesetter::new(1.0);
        assert!(setter
            .register_font_file(std::path::Path::new("/definitely/not/here.ttf"))
            .is_err());
    }

    #[test]
    fn garbage_font_data_registers_nothing() {
        let mut setter = CTFramesetter::new(1.0);
        let names = setter.register_font_data(b"not a font".to_vec());
        assert!(names.is_empty());
    }

    #[test]
    fn real_font_file_registers_family() {
        let path = std::path::Path::new("/usr/share/fonts/TTF/DejaVuSerifCondensed.ttf");
        if !path.exists() {
            return;
        }
        let mut setter = CTFramesetter::new(1.0);
        let names = setter.register_font_file(path).expect("register");
        assert!(!names.is_empty());
        setter.set_family(&names[0]);
        let frame = setter.create_plain_frame(
            "Preview 0123456789",
            &CTParagraphStyle::default(),
            28.0,
            Color::WHITE,
            400.0,
            None,
        );
        assert!(frame.size().0 > 0.0);
    }
}
