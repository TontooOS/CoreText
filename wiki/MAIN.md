# CoreText – Wiki

Apple CoreText-like crisp text stack for TontooOS: font management,
layout, crisp rendering, caret mapping and decorations.

- Repository: https://github.com/TontooOS/Libs
- License: TCL
- Version: 26.1.0

## Feature Index

| Feature | File | Description |
|---|---|---|
| Main index | [MAIN.md](MAIN.md) | This page |
| Rules | [RULE.md](RULE.md) | Development and usage rules |
| Font | [Font.md](Font.md) | Loading, system scan, weights, fallback, registration |
| Attr | [Attr.md](Attr.md) | Attributed strings, paragraph style, markdown |
| Typeset | [Typeset.md](Typeset.md) | CTLine, CTFrame, CTFramesetter, measure, truncate |
| Render | [Render.md](Render.md) | Crisp Vello draws, gradient, shadow, stroke |
| Caret | [Caret.md](Caret.md) | Caret mapping, selection, hit testing, graphemes |
| Decorate | [Decorate.md](Decorate.md) | Underline, strikethrough, run spans, links |
| Ffi | [Ffi.md](Ffi.md) | C ABI: version, measure |

## Quick Start

```rust
use coretext::{CTFramesetter, CTParagraphStyle};
use vello::peniko::Color;

let mut setter = CTFramesetter::new(2.0);
let frame = setter.create_plain_frame(
  "Hello, TontooOS",
  &CTParagraphStyle::default(),
  17.0,
  Color::WHITE,
  400.0,
  None,
);
let (w, h) = frame.size();
```

See [Typeset.md](Typeset.md) for layout and [Render.md](Render.md)
for crisp drawing.

## Changelog

- 2026-09-28: `CTFramesetter::register_font_file` and
  `register_font_data` load font files/bytes into the layout
  context (family names returned from the name tables).
- 2026-09-28: `CTFramesetter::build` honors `set_family` (pushes
  `"family", system-ui` with a system fallback); previews render
  the named family.
- 2026-09-26: Initial CoreText release (font, attr, typeset, render,
  caret, decorate, ffi). TontooUI text stack moved to CoreText.
