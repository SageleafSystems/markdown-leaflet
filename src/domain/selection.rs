// SPDX-License-Identifier: GPL-3.0-or-later
//! A drag-selected range over the preview's selectable leaves (one per
//! rendered heading, paragraph, code line, or image caption; see
//! `ui::selectable_text`), kept independent of `iced` so the range math is
//! cheap to test. Offsets are byte offsets into a leaf's own text, matching
//! how `ui::find_viewer` already slices spans for find-match highlighting.

use std::ops::Range;

/// One selectable run of text, in document render order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId {
    /// Index of the top-level `markdown::Item` this leaf belongs to.
    pub item: usize,
    /// Index of this leaf within that item: 0 for a heading, paragraph, or
    /// image caption; the line number for a code block.
    pub leaf: usize,
}

/// A byte offset within one [`BlockId`]'s text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub block: BlockId,
    pub offset: usize,
}

/// A drag selection, kept as the original anchor and the current head so the
/// direction the user dragged doesn't matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn at(position: Position) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    /// A selection over every leaf, from the start of the first to the end of
    /// the last; `None` for a document with no text.
    pub fn all<'a>(leaves: impl IntoIterator<Item = (BlockId, &'a str)>) -> Option<Self> {
        let mut leaves = leaves.into_iter();
        let (first, first_text) = leaves.next()?;
        let (last, last_text) = leaves.fold((first, first_text), std::cmp::Ord::max);
        let start = Position {
            block: first,
            offset: 0,
        };
        let end = Position {
            block: last,
            offset: last_text.len(),
        };
        (start != end).then_some(Self {
            anchor: start,
            head: end,
        })
    }

    pub fn extend(&mut self, to: Position) {
        self.head = to;
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    fn ordered(&self) -> (Position, Position) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    /// Whether this selection covers any part of `block`.
    pub fn touches(&self, block: BlockId) -> bool {
        let (start, end) = self.ordered();
        block >= start.block && block <= end.block
    }

    /// The selected byte range within `block`, whose text is `len` bytes
    /// long, if this selection covers any of it.
    pub fn range_in(&self, block: BlockId, len: usize) -> Option<Range<usize>> {
        let (start, end) = self.ordered();
        if block < start.block || block > end.block {
            return None;
        }
        let from = if block == start.block {
            start.offset.min(len)
        } else {
            0
        };
        let to = if block == end.block {
            end.offset.min(len)
        } else {
            len
        };
        (from < to).then_some(from..to)
    }
}

/// The word around byte `offset` in `text`, for double-click: a run of
/// alphanumerics and `_`, or else the single character there. Empty only for
/// empty text.
pub fn word_at(text: &str, offset: usize) -> Range<usize> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let Some(at) = text[offset..]
        .chars()
        .next()
        .or_else(|| text[..offset].chars().next_back())
    else {
        return 0..0;
    };
    if offset == text.len() {
        offset -= at.len_utf8();
    }
    if !is_word(at) {
        return offset..offset + at.len_utf8();
    }
    let start = text[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word(*c))
        .last()
        .map_or(offset, |(index, _)| index);
    let end = text[offset..]
        .char_indices()
        .find(|(_, c)| !is_word(*c))
        .map_or(text.len(), |(index, _)| offset + index);
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(item: usize, leaf: usize, offset: usize) -> Position {
        Position {
            block: BlockId { item, leaf },
            offset,
        }
    }

    #[test]
    fn single_block_range_is_exact() {
        let selection = Selection {
            anchor: pos(0, 0, 2),
            head: pos(0, 0, 5),
        };
        assert_eq!(
            selection.range_in(BlockId { item: 0, leaf: 0 }, 10),
            Some(2..5)
        );
        assert_eq!(selection.range_in(BlockId { item: 1, leaf: 0 }, 10), None);
    }

    #[test]
    fn reversed_drag_normalizes() {
        let selection = Selection {
            anchor: pos(1, 0, 5),
            head: pos(0, 0, 2),
        };
        assert_eq!(
            selection.range_in(BlockId { item: 0, leaf: 0 }, 10),
            Some(2..10)
        );
        assert_eq!(
            selection.range_in(BlockId { item: 1, leaf: 0 }, 10),
            Some(0..5)
        );
    }

    #[test]
    fn blocks_strictly_between_anchor_and_head_are_selected_whole() {
        let selection = Selection {
            anchor: pos(0, 0, 3),
            head: pos(2, 0, 1),
        };
        assert_eq!(
            selection.range_in(BlockId { item: 1, leaf: 0 }, 7),
            Some(0..7)
        );
        assert!(!selection.touches(BlockId { item: 3, leaf: 0 }));
        assert!(selection.touches(BlockId { item: 1, leaf: 0 }));
    }

    #[test]
    fn empty_selection_has_no_range() {
        let selection = Selection::at(pos(0, 0, 4));
        assert!(selection.is_empty());
        assert_eq!(selection.range_in(BlockId { item: 0, leaf: 0 }, 10), None);
    }

    #[test]
    fn all_spans_first_leaf_start_to_last_leaf_end() {
        let a = BlockId { item: 0, leaf: 0 };
        let b = BlockId { item: 2, leaf: 1 };
        let selection = Selection::all([(a, "Title"), (b, "last line")]).unwrap();
        assert_eq!(selection.range_in(a, 5), Some(0..5));
        assert_eq!(selection.range_in(b, 9), Some(0..9));
        assert!(Selection::all([]).is_none());
        assert!(Selection::all([(a, "")]).is_none());
    }

    #[test]
    fn word_at_finds_the_surrounding_word() {
        let text = "say hello_world, ok";
        assert_eq!(&text[word_at(text, 6)], "hello_world");
        assert_eq!(&text[word_at(text, 4)], "hello_world");
        assert_eq!(&text[word_at(text, 15)], ",");
        assert_eq!(&text[word_at(text, 3)], " ");
        assert_eq!(&text[word_at(text, text.len())], "ok");
        assert_eq!(word_at("", 0), 0..0);
    }

    #[test]
    fn word_at_is_utf8_safe() {
        let text = "café ünïcode";
        assert_eq!(&text[word_at(text, 4)], "café");
        assert_eq!(&text[word_at(text, 8)], "ünïcode");
    }

    #[test]
    fn offsets_past_the_leaf_length_clamp() {
        let selection = Selection {
            anchor: pos(0, 0, 2),
            head: pos(0, 0, 999),
        };
        assert_eq!(
            selection.range_in(BlockId { item: 0, leaf: 0 }, 6),
            Some(2..6)
        );
    }
}
