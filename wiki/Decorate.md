# Decorate

Underline/strikethrough geometry and run spans
(`src/decorate.rs`). Decorations paint from Parley run metrics, so
they track the real shaped glyphs (RTL, ligatures, mixed fonts).

## Decoration

```rust
pub struct Decoration {
  pub x0: f32,
  pub y0: f32,
  pub x1: f32,
  pub y1: f32,
  pub color: Color,
  pub strikethrough: bool,
}
```

- Rects are physical px relative to the frame origin; callers add
  the snapped draw origin, like the glyph path does.
- `strikethrough` false means underline.

```rust
pub fn decorations(frame: &CTFrame) -> Vec<Decoration>
```

## Run Spans

```rust
pub fn run_spans(frame: &CTFrame) -> Vec<(Range<usize>, Color)>
```

- Per-run (byte range, baked color) spans in layout order. Used to
  tell explicitly colored runs apart from default runs (e.g. for
  gradient text, where default runs take the gradient brush).

## Usage / Example

```rust
use coretext::decorations;
use vello::kurbo::Rect;
use vello::peniko::{Brush, Fill};

fn paint_decos(scene: &mut vello::Scene, frame: &coretext::CTFrame, ox: f32, oy: f32) {
  for deco in decorations(frame) {
    let rect = Rect::new(
      (ox + deco.x0) as f64,
      (oy + deco.y0) as f64,
      (ox + deco.x1) as f64,
      (oy + deco.y1) as f64,
    );
    scene.fill(Fill::NonZero, vello::kurbo::Affine::IDENTITY, &Brush::Solid(deco.color), None, &rect);
  }
}
```

## Cross References

- [Render.md](Render.md) – glyph draws in the same space
- [Attr.md](Attr.md) – span flags behind decorations
