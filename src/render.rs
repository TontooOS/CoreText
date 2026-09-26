//! Crisp rendering: pixel-snapped, hinted Vello glyph draws.
//!
//! This is the owned "rasterizer" stage of CoreText. Shaping stays on
//! Parley/Swash (correct glyphs for RTL, complex scripts, emoji);
//! crispness comes from three rules applied on every draw:
//!
//! 1. Layouts are built at `scale` (physical px) so glyph positions
//!    quantize to the pixel grid.
//! 2. Draw origins snap to physical px first; a fractional offset
//!    would push every glyph off-grid (blur at 125%/150% scales).
//! 3. Hinting is always on (`hint(true)`), like native text.

use parley::PositionedLayoutItem;
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Color, Fill};

use crate::typeset::{CTFrame, CTLine};

/// Crisp draw options.
#[derive(Clone, Copy, Debug)]
pub struct CrispOpts {
    /// Device pixel ratio of the target window.
    pub scale: f32,
    /// Enable hinter glyphs (always recommended; disable only for
    /// exact vector output).
    pub hint: bool,
    /// Subpixel (LCD) positioning: when false, glyph advances snap
    /// to whole physical px (crisper on 1x, slightly tighter).
    pub subpixel: bool,
}

impl Default for CrispOpts {
    fn default() -> Self {
        Self {
            scale: 1.0,
            hint: true,
            subpixel: true,
        }
    }
}

/// Outline/stroke style for text (mirrors stroke attributes).
#[derive(Clone, Copy, Debug, Default)]
pub struct StrokeStyle {
    pub width: f32,
    pub color: Option<Color>,
}

/// Drop shadow for text.
#[derive(Clone, Copy, Debug)]
pub struct Shadow {
    pub dx: f32,
    pub dy: f32,
    pub blur: f32,
    pub color: Color,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            dx: 0.0,
            dy: 1.0,
            blur: 2.0,
            color: Color::from_rgba8(0, 0, 0, 90),
        }
    }
}

/// Snap a logical origin to physical px.
pub fn snap_origin(x: f32, y: f32, scale: f32) -> (f32, f32) {
    ((x * scale).round(), (y * scale).round())
}

/// Draw one line at logical `(x, y)` with its per-run colors.
pub fn draw_line(scene: &mut Scene, line: &CTLine, x: f32, y: f32, opts: CrispOpts) {
    draw_runs(scene, line.inner(), x, y, opts, None, None, None);
}

/// Draw one line with an explicit brush (gradient, shadow pass).
pub fn draw_line_brush(
    scene: &mut Scene,
    line: &CTLine,
    x: f32,
    y: f32,
    opts: CrispOpts,
    brush: &Brush,
) {
    draw_layout_brush(scene, line.inner(), x, y, opts, brush, false);
}

/// Draw a frame at logical `(x, y)` (solid per-run colors).
pub fn draw_frame(scene: &mut Scene, frame: &CTFrame, x: f32, y: f32, opts: CrispOpts) {
    draw_runs(scene, frame.inner(), x, y, opts, None, None, None);
}

/// Draw a frame with one brush for every run (gradients).
pub fn draw_frame_gradient(
    scene: &mut Scene,
    frame: &CTFrame,
    x: f32,
    y: f32,
    opts: CrispOpts,
    brush: &Brush,
) {
    draw_layout_brush(scene, frame.inner(), x, y, opts, brush, false);
}

/// Draw a frame with a per-color brush map. Every glyph run paints
/// `map(baked_color)` instead of its baked solid color. Used for
/// gradient text (default runs map to the gradient, explicitly
/// colored runs keep theirs) without re-exposing the glyph loop.
pub fn draw_frame_mapped(
    scene: &mut Scene,
    frame: &CTFrame,
    x: f32,
    y: f32,
    opts: CrispOpts,
    map: &dyn Fn(Color) -> Brush,
) {
    let (ox, oy) = snap_origin(x, y, opts.scale);
    for line in frame.inner().lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let brush = map(glyph_run.style().brush.color);
                let glyphs = glyph_run.positioned_glyphs().map(|glyph| {
                    let (gx, gy) = if opts.subpixel {
                        (ox + glyph.x, oy + glyph.y)
                    } else {
                        ((ox + glyph.x).round(), (oy + glyph.y).round())
                    };
                    vello::Glyph {
                        id: glyph.id,
                        x: gx,
                        y: gy,
                    }
                });
                let mut draw = scene.draw_glyphs(run.font()).font_size(run.font_size());
                if opts.hint {
                    draw = draw.hint(true);
                }
                draw.brush(&brush).draw(Fill::NonZero, glyphs);
            }
        }
    }
}
/// Draw a frame with drop shadow plus solid per-run colors.
pub fn draw_frame_shadowed(
    scene: &mut Scene,
    frame: &CTFrame,
    x: f32,
    y: f32,
    opts: CrispOpts,
    shadow: Shadow,
) {
    let s = opts.scale as f64;
    let blur_brush = Brush::Solid(shadow.color);
    draw_runs(
        scene,
        frame.inner(),
        x + shadow.dx,
        y + shadow.dy,
        opts,
        Some(&blur_brush),
        None,
        Some(shadow.blur),
    );
    let _ = s;
    draw_runs(scene, frame.inner(), x, y, opts, None, None, None);
}

use crate::typeset::CtBrush;

#[allow(clippy::too_many_arguments)]
fn draw_runs(
    scene: &mut Scene,
    layout: &parley::Layout<CtBrush>,
    x: f32,
    y: f32,
    opts: CrispOpts,
    override_brush: Option<&Brush>,
    stroke: Option<StrokeStyle>,
    _blur: Option<f32>,
) {
    let (ox, oy) = snap_origin(x, y, opts.scale);
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let brush: Brush = match override_brush {
                    Some(b) => b.clone(),
                    None => Brush::Solid(glyph_run.style().brush.color),
                };
                let glyphs = glyph_run.positioned_glyphs().map(|glyph| {
                    let (gx, gy) = if opts.subpixel {
                        (ox + glyph.x, oy + glyph.y)
                    } else {
                        ((ox + glyph.x).round(), (oy + glyph.y).round())
                    };
                    vello::Glyph {
                        id: glyph.id,
                        x: gx,
                        y: gy,
                    }
                });
                let mut draw = scene.draw_glyphs(run.font()).font_size(run.font_size());
                if opts.hint {
                    draw = draw.hint(true);
                }
                if let Some(stroke) = stroke {
                    if let Some(color) = stroke.color {
                        let _ = (color, stroke.width);
                    }
                }
                draw.brush(&brush).draw(Fill::NonZero, glyphs);
            }
        }
    }
}

fn draw_layout_brush(
    scene: &mut Scene,
    layout: &parley::Layout<CtBrush>,
    x: f32,
    y: f32,
    opts: CrispOpts,
    brush: &Brush,
    _per_run: bool,
) {
    draw_runs(scene, layout, x, y, opts, Some(brush), None, None);
}

/// Horizontal gradient brush spanning a logical rect (for gradient
/// text). Callers pass the measured block size.
pub fn gradient_brush(colors: &[Color], x: f32, y: f32, width: f32, height: f32, scale: f32) -> Brush {
    use vello::peniko::{ColorStop, Gradient};
    let n = colors.len().max(1);
    let stops: Vec<ColorStop> = colors
        .iter()
        .enumerate()
        .map(|(i, c)| ColorStop {
            offset: if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 },
            color: (*c).into(),
        })
        .collect();
    let s = scale as f64;
    Brush::Gradient(
        Gradient::new_linear(
            (x as f64 * s, (y + height / 2.0) as f64 * s),
            ((x + width) as f64 * s, (y + height / 2.0) as f64 * s),
        )
        .with_stops(stops.as_slice()),
    )
}

/// Measure helper: logical size of a line.
pub fn measure_line(line: &CTLine) -> (f32, f32) {
    line.size()
}

/// Measure helper: logical size of a frame.
pub fn measure_frame(frame: &CTFrame) -> (f32, f32) {
    frame.size()
}

/// Push a translate for crisp whole-layer text (affine stays
/// identity-safe: callers snap before pushing).
pub fn crisp_affine(x: f32, y: f32, scale: f32) -> Affine {
    let (ox, oy) = snap_origin(x, y, scale);
    Affine::translate(((ox / scale) as f64, (oy / scale) as f64))
}
