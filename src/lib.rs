//! CoreText for TontooOS: Apple CoreText-like crisp text stack.
//!
//! Modules mirror the requested feature set:
//!
//! - [`font`]: loading (TTF/OTF), system font scan, family/weight/style
//!   resolution, fallback chains, runtime registration.
//! - [`attr`]: attributed strings, paragraph style (alignment,
//!   line height, tracking, line break), markdown subset.
//! - [`typeset`]: `CTFont` + `CTParagraphStyle` + `AttributedString` to
//!   `CTLine` / `CTFrame` (measure, wrap, align, justify, truncate).
//! - [`render`]: crisp Vello drawing (pixel-snapped origin, hinted
//!   glyphs), color, gradient, stroke/outline, shadow, bitmap measure.
//! - [`caret`]: caret/pixel mapping, selection rects, hit testing,
//!   grapheme/word helpers, RTL and complex-script safe paths.
//!
//! Shaping and bidi stay on Parley/Swash (correct for RTL, complex
//! scripts and emoji ZWJ sequences). The crisp stage is CoreText
//! owned: scale-quantized layout, pixel-snapped origins and hinted
//! draws, plus SF Pro-first fallback chains.

pub mod attr;
pub mod caret;
pub mod decorate;
pub mod ffi;
pub mod font;
pub mod render;
pub mod typeset;

pub use attr::{
    AttrSpan, AttributedString, CTLineBreakMode, CTParagraphStyle, CTTextAlignment,
    parse_markdown,
};
pub use caret::{
    caret_geometry, caret_to_point, column_at_x, hit_byte, line_for_caret, link_at,
    point_to_caret, selection_rects, word_range,
};
pub use decorate::{Decoration, decorations, run_spans};
pub use font::{
    CTFont, CTFontDescriptor, CTFontStyle, CTFontWeight, FontRegistry, SF_PRO_FAMILY,
    system_font_dirs,
};
pub use render::{CrispOpts, Shadow, StrokeStyle, crisp_affine, draw_frame, draw_frame_gradient, draw_frame_mapped, draw_frame_shadowed, draw_line, gradient_brush, measure_frame, measure_line};
pub use typeset::{CTFrame, CTLine, CTRun, CTRunStyle, CTFramesetter, FrameLine};

/// Library version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
