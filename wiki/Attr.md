# Attr

Attributed strings and paragraph style (`src/attr.rs`): mixed styles
inside one string, paragraph alignment, line height, tracking, line
break and a markdown subset parser.

## CTTextAlignment

```rust
pub enum CTTextAlignment {
  Leading,
  Center,
  Trailing,
  Justified,
}
```

## CTLineBreakMode

```rust
pub enum CTLineBreakMode {
  ByWordWrapping,
  ByCharWrapping,
  ByClipping,
  ByTruncatingTail,
}
```

- `ByClipping` and `ByTruncatingTail` lay out a single line without
  wrapping; truncation with an ellipsis happens through
  `CTParagraphStyle::line_limit` in [Typeset.md](Typeset.md).

## CTParagraphStyle

```rust
pub struct CTParagraphStyle {
  pub alignment: CTTextAlignment,
  pub line_break: CTLineBreakMode,
  pub line_height: f32,
  pub line_spacing: f32,
  pub tracking: f32,
  pub line_limit: Option<usize>,
}
```

```rust
pub fn alignment(mut self, alignment: CTTextAlignment) -> Self
pub fn line_height(mut self, value: f32) -> Self
pub fn line_spacing(mut self, value: f32) -> Self
pub fn tracking(mut self, value: f32) -> Self
pub fn line_break(mut self, mode: CTLineBreakMode) -> Self
pub fn line_limit(mut self, lines: usize) -> Self
```

- Defaults match TontooUI: leading alignment, word wrapping,
  `line_height` `1.25`, no spacing, no tracking, no limit.
- `tracking` is letter spacing in logical px (negative tightens).
- `line_limit` truncates with an ellipsis (see
  [Typeset.md](Typeset.md)).

## AttrSpan

```rust
pub struct AttrSpan {
  pub range: Range<usize>,
  pub bold: bool,
  pub italic: bool,
  pub monospace: bool,
  pub color: Option<[u8; 4]>,
  pub underline: bool,
  pub underline_color: Option<[u8; 4]>,
  pub strikethrough: bool,
  pub strikethrough_color: Option<[u8; 4]>,
  pub link: Option<String>,
}
```

```rust
pub fn new(range: Range<usize>) -> Self
```

- Colors are packed RGBA8; `None` keeps the base color.
- Decoration colors fall back to the span color, then the base.
- CoreText stays neutral on links: no automatic underline or
  color; callers (like TontooUI `FormattedText`) style link runs.

## AttributedString

```rust
pub fn new(text: impl Into<String>) -> Self
pub fn markdown(source: impl Into<String>) -> Self
pub fn push_span(&mut self, span: AttrSpan)
pub fn text(&self) -> &str
pub fn spans(&self) -> &[AttrSpan]
pub fn links(&self) -> Vec<(Range<usize>, String)>
```

- Empty ranges are skipped by `push_span`.
- Markdown subset, single paragraph: `**bold**`, `*italic*`,
  `_italic_` (needs word flanking), `***bold italic***`,
  `~~strikethrough~~`, `` `code` `` (monospace), `[label](url)`,
  `\` escapes. Unmatched markers stay literal.

```rust
pub fn parse_markdown(source: &str) -> (String, Vec<AttrSpan>, Vec<(Range<usize>, String)>)
```

## Usage / Example

```rust
use coretext::{AttrSpan, AttributedString, CTParagraphStyle, CTTextAlignment};

let mut string = AttributedString::markdown("**Bold** and [link](https://example.com)");
let mut accent = AttrSpan::new(0..4);
accent.underline = true;
string.push_span(accent);
let para = CTParagraphStyle::default().alignment(CTTextAlignment::Center);
```

## Cross References

- [Typeset.md](Typeset.md) – spans to `CTLine`/`CTFrame`
- [Decorate.md](Decorate.md) – decoration geometry of spans
