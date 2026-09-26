# Caret

Editable-text support (`src/caret.rs`): caret mapping, selection
rects, hit testing, grapheme and word helpers. All functions take
logical px and convert internally.

## Mapping

```rust
pub fn point_to_caret(setter: &mut CTFramesetter, text: &str, size: f32, color: Color, goal_x: f32) -> usize
pub fn caret_to_point(setter: &mut CTFramesetter, text: &str, idx: usize, size: f32, color: Color) -> f32
pub fn caret_geometry(setter: &mut CTFramesetter, frame: &CTFrame, text: &str, caret: usize, size: f32, color: Color) -> (f32, f32, f32)
pub fn line_for_caret(setter: &mut CTFramesetter, frame: &CTFrame, text: &str, caret: usize, size: f32, color: Color) -> (usize, f32)
pub fn column_at_x(setter: &mut CTFramesetter, frame: &CTFrame, text: &str, line: usize, goal_x: f32, size: f32, color: Color) -> usize
```

- `point_to_caret` finds the nearest advance boundary (ties to the
  earlier one) with a binary search over char boundaries, then
  snaps to the grapheme boundary (O(log n) layouts).
- `caret_geometry` returns caret `(x, y, height)` relative to the
  frame origin; a caret exactly on a line start belongs to that
  line.
- `column_at_x` treats a trailing newline as the break, not the
  walk; past the last line end lands at the very end.

## Hit Testing

```rust
pub fn hit_byte(frame: &CTFrame, x: f32, y: f32) -> Option<usize>
pub fn link_at(frame: &CTFrame, x: f32, y: f32) -> Option<String>
```

- Exact hits through `Cluster::from_point_exact`: padding never
  counts as content. RTL and ligatures resolve to the right
  cluster; complex scripts stay safe.
- `link_at` maps the hit byte through the frame link ranges.

## Selection

```rust
pub fn selection_rects(setter: &mut CTFramesetter, frame: &CTFrame, text: &str, range: Range<usize>, size: f32, color: Color) -> Vec<(f32, f32, f32, f32)>
pub fn word_range(text: &str, caret: usize) -> (usize, usize)
```

- One `(x0, y, x1, y1)` rect per wrapped line, in logical px
  relative to the frame origin.
- `word_range` covers alphanumeric plus `_` around the caret;
  collapsed between words.

## Usage / Example

```rust
use coretext::{CTFramesetter, caret_to_point, point_to_caret};
use vello::peniko::Color;

let mut setter = CTFramesetter::new(2.0);
let x = caret_to_point(&mut setter, "hello", 3, 13.0, Color::WHITE);
let back = point_to_caret(&mut setter, "hello", 13.0, Color::WHITE, x);
assert_eq!(back, 3);
```

## Cross References

- [Typeset.md](Typeset.md) – frames and line ranges
- [Decorate.md](Decorate.md) – link ranges on frames
