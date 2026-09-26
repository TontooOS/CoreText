//! Underline/strikethrough geometry and run spans.
//!
//! Decorations paint from Parley run metrics, so they track the real
//! shaped glyphs (RTL, ligatures, mixed fonts included).

use std::ops::Range;

use parley::PositionedLayoutItem;
use vello::peniko::Color;

use crate::typeset::{CTFrame, CtBrush};

/// One underline or strikethrough rect in physical px relative to
/// the frame origin, plus its color.
#[derive(Clone, Copy, Debug)]
pub struct Decoration {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color: Color,
    pub strikethrough: bool,
}

/// Underline/strikethrough rects for every glyph run in `frame`, in
/// physical px relative to the frame origin. Callers add the snapped
/// draw origin, like the glyph path does.
pub fn decorations(frame: &CTFrame) -> Vec<Decoration> {
    let mut out = Vec::new();
    let layout = frame.inner();
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let metrics = run.metrics();
                let style = glyph_run.style();
                // Both offsets live in physical px already.
                let x0 = glyph_run.offset();
                let x1 = x0 + glyph_run.advance();
                if x1 <= x0 {
                    continue;
                }
                let base = glyph_run.baseline();
                if let Some(underline) = &style.underline {
                    let size = underline.size.unwrap_or(metrics.underline_size).max(1.0);
                    let top = base + underline.offset.unwrap_or(metrics.underline_offset);
                    out.push(Decoration {
                        x0,
                        y0: top,
                        x1,
                        y1: top + size,
                        color: underline.brush.color,
                        strikethrough: false,
                    });
                }
                if let Some(strike) = &style.strikethrough {
                    let size = strike
                        .size
                        .unwrap_or(metrics.strikethrough_size)
                        .max(1.0);
                    let top = base + strike.offset.unwrap_or(metrics.strikethrough_offset);
                    out.push(Decoration {
                        x0,
                        y0: top,
                        x1,
                        y1: top + size,
                        color: strike.brush.color,
                        strikethrough: true,
                    });
                }
            }
        }
    }
    out
}

/// Per-run (byte range, baked color) spans in layout order. Used to
/// tell explicitly colored runs apart from default runs (e.g. for
/// gradient text, where default runs take the gradient brush).
pub fn run_spans(frame: &CTFrame) -> Vec<(Range<usize>, Color)> {
    let mut out = Vec::new();
    for line in frame.inner().lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                out.push((
                    glyph_run.run().text_range(),
                    glyph_run.style().brush.color,
                ));
            }
        }
    }
    out
}

#[allow(dead_code)]
fn _brush_marker(_: &CtBrush) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attr::AttributedString;
    use crate::typeset::CTFramesetter;
    use vello::peniko::Color;

    #[test]
    fn underline_produces_rect() {
        let mut setter = CTFramesetter::new(1.0);
        let mut attr = AttributedString::new("underlined");
        let mut span = crate::attr::AttrSpan::new(0..10);
        span.underline = true;
        attr.push_span(span);
        let frame = setter.create_frame(
            &attr,
            &crate::attr::CTParagraphStyle::default(),
            17.0,
            Color::WHITE,
            400.0,
            None,
        );
        assert!(!decorations(&frame).is_empty());
    }

    #[test]
    fn markdown_links_recorded() {
        let mut setter = CTFramesetter::new(1.0);
        let attr = AttributedString::markdown("[a](https://x.test)");
        let frame = setter.create_frame(
            &attr,
            &crate::attr::CTParagraphStyle::default(),
            17.0,
            Color::WHITE,
            400.0,
            None,
        );
        assert_eq!(frame.links().len(), 1);
        assert_eq!(frame.links()[0].1, "https://x.test");
    }
}
