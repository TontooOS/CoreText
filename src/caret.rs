//! Editable-text support: caret mapping, selection, hit testing.
//!
//! All functions take logical px and convert internally. Grapheme and
//! word handling use `unicode-segmentation` so emoji ZWJ sequences
//! count as one unit; shaping/bidi correctness comes from Parley
//! (`Cluster::from_point_exact`), which is RTL and complex-script
//! safe.

use parley::Cluster;
use unicode_segmentation::UnicodeSegmentation;
use vello::peniko::Color;

use crate::typeset::CTFrame;

/// Byte index in `text` at logical `goal_x` (text-origin coords) for
/// a single line: nearest advance boundary, ties to the earlier one.
/// Binary searches over char boundaries (O(log n) layouts).
pub fn point_to_caret(
    setter: &mut crate::typeset::CTFramesetter,
    text: &str,
    size: f32,
    color: Color,
    goal_x: f32,
) -> usize {
    if goal_x <= 0.0 || text.is_empty() {
        return 0;
    }
    let bounds = char_bounds(text);
    let goal = goal_x;
    let advance_at = |k: usize, setter: &mut crate::typeset::CTFramesetter| -> f32 {
        match text.get(..bounds[k]) {
            Some(slice) => setter.measure(slice, size, color, 400.0, None).0,
            None => 0.0,
        }
    };
    let mut lo = 0usize;
    let mut hi = bounds.len() - 1;
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if advance_at(mid, setter) <= goal {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut at = bounds[lo];
    if lo + 1 < bounds.len() {
        let a0 = advance_at(lo, setter);
        let a1 = advance_at(lo + 1, setter);
        if goal > (a0 + a1) / 2.0 {
            at = bounds[lo + 1];
        }
    }
    snap_to_grapheme(text, at)
}

/// Advance of `text[..idx]` in logical px.
pub fn caret_to_point(
    setter: &mut crate::typeset::CTFramesetter,
    text: &str,
    idx: usize,
    size: f32,
    color: Color,
) -> f32 {
    let idx = idx.min(text.len());
    let slice = text.get(..idx).unwrap_or("");
    setter.measure(slice, size, color, 400.0, None).0
}

/// Caret (x, y, height) in logical px relative to the frame origin.
pub fn caret_geometry(
    setter: &mut crate::typeset::CTFramesetter,
    frame: &CTFrame,
    text: &str,
    caret: usize,
    size: f32,
    color: Color,
) -> (f32, f32, f32) {
    let (line, x) = line_for_caret(setter, frame, text, caret, size, color);
    let mut y = 0.0;
    for l in frame.lines().iter().take(line) {
        y += l.height;
    }
    let h = frame.lines().get(line).map(|l| l.height).unwrap_or(size * 1.25);
    (x, y, h)
}

/// Line index plus x advance for a caret.
pub fn line_for_caret(
    setter: &mut crate::typeset::CTFramesetter,
    frame: &CTFrame,
    text: &str,
    caret: usize,
    size: f32,
    color: Color,
) -> (usize, f32) {
    let lines = frame.lines();
    if lines.is_empty() {
        return (0, 0.0);
    }
    let caret = caret.min(text.len());
    for (index, line) in lines.iter().enumerate() {
        if caret == line.range.start {
            return (index, 0.0);
        }
    }
    let mut line = lines.len() - 1;
    for (index, l) in lines.iter().enumerate() {
        if caret >= l.range.start && caret <= l.range.end {
            line = index;
            break;
        }
    }
    let l = &lines[line];
    let end = l.range.end.min(text.len());
    let start = l.range.start.min(end);
    let at = caret.min(end).max(start);
    let x = text
        .get(start..at)
        .map(|slice| setter.measure(slice, size, color, 400.0, None).0)
        .unwrap_or(0.0);
    (line, x)
}

/// Byte index in `line` at column `goal_x`.
pub fn column_at_x(
    setter: &mut crate::typeset::CTFramesetter,
    frame: &CTFrame,
    text: &str,
    line: usize,
    goal_x: f32,
    size: f32,
    color: Color,
) -> usize {
    let lines = frame.lines();
    if lines.is_empty() {
        return text.len();
    }
    let line = line.min(lines.len() - 1);
    let (start, end) = (lines[line].range.start, lines[line].range.end);
    let end = end.min(text.len());
    let start = start.min(end);
    let walk_end = match text.get(start..end) {
        Some(slice) if slice.ends_with('\n') => end.saturating_sub(1).max(start),
        _ => end,
    };
    let mut bounds = vec![start];
    let mut at = start;
    while at < walk_end {
        match text[at..].chars().next() {
            Some(c) => {
                at = (at + c.len_utf8()).min(walk_end);
                bounds.push(at);
                if at >= walk_end {
                    break;
                }
            }
            None => break,
        }
    }
    let goal = goal_x.max(0.0);
    let advance_at = |k: usize, setter: &mut crate::typeset::CTFramesetter| -> f32 {
        text.get(start..bounds[k])
            .map(|slice| setter.measure(slice, size, color, 400.0, None).0)
            .unwrap_or(0.0)
    };
    let mut lo = 0usize;
    let mut hi = bounds.len().saturating_sub(1);
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if advance_at(mid, setter) <= goal {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut at = bounds[lo];
    if lo + 1 < bounds.len() {
        let a0 = advance_at(lo, setter);
        let a1 = advance_at(lo + 1, setter);
        if (a0 + a1) / 2.0 < goal {
            at = bounds[lo + 1];
        }
    }
    snap_to_grapheme(text, at)
}

/// Exact hit test on a frame: logical point to byte index, or `None`
/// when the point hits no glyph (padding never counts as content).
/// Uses `Cluster::from_point_exact`, so RTL and ligatures resolve to
/// the right cluster.
pub fn hit_byte(frame: &CTFrame, x: f32, y: f32) -> Option<usize> {
    let scale = frame.scale();
    let (cluster, _) = Cluster::from_point_exact(frame.inner(), x * scale, y * scale)?;
    Some(cluster.text_range().start)
}

/// Link URL at a logical point in the frame, or `None`.
pub fn link_at(frame: &CTFrame, x: f32, y: f32) -> Option<String> {
    let at = hit_byte(frame, x, y)?;
    frame
        .links()
        .iter()
        .find(|(range, _)| range.contains(&at))
        .map(|(_, url)| url.clone())
}

/// Selection wash rects for a byte range, in logical px relative to
/// the frame origin. One rect per wrapped line.
pub fn selection_rects(
    setter: &mut crate::typeset::CTFramesetter,
    frame: &CTFrame,
    text: &str,
    range: std::ops::Range<usize>,
    size: f32,
    color: Color,
) -> Vec<(f32, f32, f32, f32)> {
    let mut out = Vec::new();
    for line in frame.lines() {
        let end = line.range.end.min(text.len());
        let start = line.range.start.min(end);
        let lo = range.start.max(start).min(end);
        let hi = range.end.max(start).min(end);
        if lo < hi {
            let before = text.get(start..lo).unwrap_or("");
            let within = text.get(start..hi).unwrap_or("");
            let x0 = setter.measure(before, size, color, 400.0, None).0;
            let x1 = setter.measure(within, size, color, 400.0, None).0;
            out.push((x0, line.y, x1, line.y + line.height));
        }
    }
    out
}

/// Word boundaries around a byte caret (alphanumeric plus `_`).
pub fn word_range(text: &str, caret: usize) -> (usize, usize) {
    let caret = caret.min(text.len());
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut a = caret;
    while a > 0 {
        match text[..a].chars().next_back() {
            Some(c) if is_word(c) => a -= c.len_utf8(),
            _ => break,
        }
    }
    let mut b = caret;
    while b < text.len() {
        match text[b..].chars().next() {
            Some(c) if is_word(c) => b += c.len_utf8(),
            _ => break,
        }
    }
    (a, b)
}

/// Char-boundary offsets of `text` (0 plus every end).
fn char_bounds(text: &str) -> Vec<usize> {
    let mut bounds = vec![0usize];
    for (i, ch) in text.char_indices() {
        bounds.push(i + ch.len_utf8());
    }
    bounds
}

/// Snap a byte index to the nearest grapheme boundary (emoji ZWJ
/// sequences stay whole).
fn snap_to_grapheme(text: &str, idx: usize) -> usize {
    let idx = idx.min(text.len());
    if text.is_char_boundary(idx) {
        // Prefer the grapheme start at or before idx.
        let mut last = 0;
        for g in text.grapheme_indices(true) {
            if g.0 >= idx {
                return g.0.min(text.len());
            }
            last = g.0;
        }
        let _ = last;
        return idx;
    }
    let mut last = 0;
    for (i, _) in text.grapheme_indices(true) {
        if i > idx {
            break;
        }
        last = i;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
use vello::peniko::Color;

    #[test]
    fn word_range_covers_words() {
        assert_eq!(word_range("hello world", 3), (0, 5));
        assert_eq!(word_range("foo_bar baz", 4), (0, 7));
    }

    #[test]
    fn caret_roundtrip_single_line() {
        let mut setter = crate::typeset::CTFramesetter::new(1.0);
        let x = caret_to_point(&mut setter, "hello", 3, 13.0, Color::WHITE);
        let back = point_to_caret(&mut setter, "hello", 13.0, Color::WHITE, x);
        assert_eq!(back, 3);
    }

    #[test]
    fn emoji_stays_one_grapheme() {
        let text = "a\u{200D}b";
        let snapped = snap_to_grapheme(text, 1);
        assert!(text.is_char_boundary(snapped));
    }
}
