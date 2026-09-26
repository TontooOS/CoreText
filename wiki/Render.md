# Render

Crisp rendering (`src/render.rs`): pixel-snapped, hinted Vello
glyph draws, color, gradient, stroke, shadow and measure helpers.

## Crisp Rules

1. Layouts build at `scale` (physical px) so glyph positions
   quantize to the pixel grid.
2. Draw origins snap to physical px first; a fractional offset
   would push every glyph off-grid (blur at 125%/150% scales).
3. Hinting is always on (`hint(true)`), like native text.

```rust
pub struct CrispOpts {
  pub scale: f32,
  pub hint: bool,
  pub subpixel: bool,
}
```

- `subpixel` off snaps glyph advances to whole physical px
  (crisper on 1x, slightly tighter).

```rust
pub fn snap_origin(x: f32, y: f32, scale: f32) -> (f32, f32)
pub fn crisp_affine(x: f32, y: f32, scale: f32) -> Affine
```

## Draw Functions

```rust
pub fn draw_line(scene: &mut Scene, line: &CTLine, x: f32, y: f32, opts: CrispOpts)
pub fn draw_line_brush(scene: &mut Scene, line: &CTLine, x: f32, y: f32, opts: CrispOpts, brush: &Brush)
pub fn draw_frame(scene: &mut Scene, frame: &CTFrame, x: f32, y: f32, opts: CrispOpts)
pub fn draw_frame_gradient(scene: &mut Scene, frame: &CTFrame, x: f32, y: f32, opts: CrispOpts, brush: &Brush)
pub fn draw_frame_mapped(scene: &mut Scene, frame: &CTFrame, x: f32, y: f32, opts: CrispOpts, map: &dyn Fn(Color) -> Brush)
pub fn draw_frame_shadowed(scene: &mut Scene, frame: &CTFrame, x: f32, y: f32, opts: CrispOpts, shadow: Shadow)
```

- `draw_frame_mapped` paints every run with `map(baked_color)`:
  gradient text maps default runs to the gradient and keeps
  explicit colors, without re-exposing the glyph loop.

## StrokeStyle

```rust
pub struct StrokeStyle {
  pub width: f32,
  pub color: Option<Color>,
}
```

## Shadow

```rust
pub struct Shadow {
  pub dx: f32,
  pub dy: f32,
  pub blur: f32,
  pub color: Color,
}
```

- Default is a `1.0` px drop shadow, `2.0` blur, black at 90/255.

## Helpers

```rust
pub fn gradient_brush(colors: &[Color], x: f32, y: f32, width: f32, height: f32, scale: f32) -> Brush
pub fn measure_line(line: &CTLine) -> (f32, f32)
pub fn measure_frame(frame: &CTFrame) -> (f32, f32)
```

- `gradient_brush` spreads stops evenly as a horizontal gradient
  across the logical text block.

## Usage / Example

```rust
use coretext::{CrispOpts, draw_frame};
use vello::Scene;

fn paint(scene: &mut Scene, frame: &coretext::CTFrame, x: f32, y: f32, scale: f32) {
  draw_frame(scene, frame, x, y, CrispOpts { scale, hint: true, subpixel: true });
}
```

## Cross References

- [Typeset.md](Typeset.md) – frames to draw
- [Decorate.md](Decorate.md) – decoration rects in the same space
