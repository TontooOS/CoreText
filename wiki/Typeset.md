# Typeset

Typesetting (`src/typeset.rs`): `CTFont` + `CTParagraphStyle` +
`AttributedString` to `CTLine` / `CTFrame`, with measure, wrap,
align, justify and ellipsis truncation.

## CtBrush

```rust
pub struct CtBrush {
  pub color: Color,
}
```

- Solid-color brush stored in Parley layouts; converted to a
  Peniko brush when drawing.

## CTRun

```rust
pub struct CTRun {
  pub range: Range<usize>,
  pub style: CTRunStyle,
}
```

- One styled run inside a line (bold, italic, monospace, color,
  underline, strikethrough, link).

## CTLine

```rust
pub fn size(&self) -> (f32, f32)
pub fn width(&self) -> f32
pub fn height(&self) -> f32
pub fn scale(&self) -> f32
pub fn inner(&self) -> &Layout<CtBrush>
```

- Sizes are logical px; `inner` exposes the Parley layout for
  CoreText-internal passes (render, caret, decorations).

## FrameLine

```rust
pub struct FrameLine {
  pub index: usize,
  pub range: Range<usize>,
  pub y: f32,
  pub height: f32,
}
```

- `range` is the byte range in the frame text; `y` and `height`
  are logical px.

## CTFrame

```rust
pub fn size(&self) -> (f32, f32)
pub fn lines(&self) -> &[FrameLine]
pub fn line_count(&self) -> usize
pub fn text(&self) -> &str
pub fn links(&self) -> &[(Range<usize>, String)]
pub fn is_truncated(&self) -> bool
pub fn scale(&self) -> f32
pub fn inner(&self) -> &Layout<CtBrush>
```

- `links` are clamped to the visible (possibly truncated) text.
- `is_truncated` reports `line_limit` ellipsis truncation.

## CTFramesetter

```rust
pub fn new(scale: f32) -> Self
pub fn set_scale(&mut self, scale: f32)
pub fn set_family(&mut self, family: impl Into<String>)
pub fn register_font_file(&mut self, path: &Path) -> std::io::Result<Vec<String>>
pub fn register_font_data(&mut self, data: Vec<u8>) -> Vec<String>
pub fn create_line(&mut self, text: &str, size: f32, color: Color, weight: f32, italic: bool, tracking: f32) -> CTLine
pub fn create_frame(&mut self, string: &AttributedString, paragraph: &CTParagraphStyle, base_size: f32, base_color: Color, base_weight: f32, max_width: Option<f32>) -> CTFrame
pub fn create_plain_frame(&mut self, text: &str, paragraph: &CTParagraphStyle, size: f32, color: Color, weight: f32, max_width: Option<f32>) -> CTFrame
pub fn measure(&mut self, text: &str, size: f32, color: Color, weight: f32, max_width: Option<f32>) -> (f32, f32)
```

- All sizes are logical px; the framesetter applies `scale`
  internally so glyphs quantize to physical pixels.
- `set_family` selects the layout family: every build pushes
  `"family", system-ui`, so unknown families fall back to the
  system font instead of `.notdef` boxes.
- `register_font_file` reads a font file into the layout context
  (no system install needed); `register_font_data` takes raw
  bytes. Both return the registered family names from the font
  name tables (empty when the data parses to no fonts).
  `register_font_file` returns `Err` when the file cannot be read.
- `measure` returns logical px without keeping the layout.
- `line_limit` keeps the longest char-prefix plus `…` fitting the
  limit (binary search over re-layouts); link ranges clamp to the
  cut.
- Shaping and bidi stay on Parley/Swash (RTL, complex scripts,
  emoji ZWJ safe); the crisp stage (scale quantization,
  pixel-snapped origins, hinted draws) is CoreText owned.

## Usage / Example

```rust
use coretext::{AttributedString, CTFramesetter, CTParagraphStyle};
use vello::peniko::Color;

let mut setter = CTFramesetter::new(2.0);
let para = CTParagraphStyle::default().line_limit(2);
let frame = setter.create_frame(
  &AttributedString::markdown("**Hi** there"),
  &para,
  17.0,
  Color::WHITE,
  400.0,
  Some(200.0),
);
assert!(frame.line_count() <= 2);
```

## Cross References

- [Font.md](Font.md) – families and fallback chains
- [Render.md](Render.md) – drawing frames crisply
- [Caret.md](Caret.md) – caret geometry on frames
