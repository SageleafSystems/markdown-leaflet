// SPDX-License-Identifier: GPL-3.0-or-later
//! The minimap's geometry: where each block's bar and the viewport slider sit,
//! and which scroll offset a click on the minimap asks for. Pure arithmetic,
//! so it runs without an `iced` window.

/// Largest scale the minimap draws at, so a short document stays a small
/// sketch at the top instead of stretching to fill the track.
pub const MAX_SCALE: f32 = 0.15;

/// Shortest bar drawn, so a one-line block stays visible.
pub const MIN_BAR_HEIGHT: f32 = 1.0;

/// What a block looks like in the minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Heading(u8),
    Paragraph,
    Code,
    Image,
    List,
    Quote,
    Table,
    Rule,
}

impl BlockKind {
    /// How a rendered Markdown item is drawn in the minimap.
    pub fn of(item: &iced::widget::markdown::Item) -> Self {
        use iced::widget::markdown::{HeadingLevel, Item};
        match item {
            Item::Heading(level, _) => Self::Heading(match level {
                HeadingLevel::H1 => 1,
                HeadingLevel::H2 => 2,
                HeadingLevel::H3 => 3,
                HeadingLevel::H4 => 4,
                HeadingLevel::H5 => 5,
                HeadingLevel::H6 => 6,
            }),
            Item::Paragraph(_) => Self::Paragraph,
            Item::Table { .. } => Self::Table,
            Item::CodeBlock { .. } => Self::Code,
            Item::Image { .. } => Self::Image,
            Item::List { .. } => Self::List,
            Item::Quote(_) => Self::Quote,
            Item::Rule => Self::Rule,
        }
    }
}

/// Vertical distance between text lines drawn inside a tall block.
pub const LINE_PITCH: f32 = 3.0;

/// How many text lines a bar of this height holds, at least one.
pub fn line_count(height: f32) -> usize {
    (height / LINE_PITCH).floor().max(1.0) as usize
}

/// Length of text line `index` of `count` as a fraction of the column. Lines
/// are ragged so a block reads as text, the last line is shorter, and the
/// result depends only on its inputs so the picture does not shimmer.
pub fn line_length(seed: usize, index: usize, count: usize) -> f32 {
    let hash = (seed as u32)
        .wrapping_mul(31)
        .wrapping_add((index as u32).wrapping_mul(17))
        .wrapping_add(7)
        .wrapping_mul(2_654_435_761);
    let ragged = f32::from(((hash >> 16) & 0xff) as u8) / 255.0;
    let length = 0.55 + 0.45 * ragged;
    if count > 1 && index + 1 == count {
        length * 0.5
    } else {
        length
    }
}

/// A rendered block, in content coordinates (pixels from the top of the document).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// Index of the Markdown item this block renders, which find matches refer to.
    pub item: usize,
    pub kind: BlockKind,
    pub y: f32,
    pub height: f32,
}

/// How far below the viewport's top a heading may sit and still count as the
/// one the reader is in.
pub const HEADING_LEEWAY: f32 = 40.0;

/// The item index of the heading the reader is in: the last one whose top is
/// at or above the viewport's top (give or take [`HEADING_LEEWAY`]).
pub fn current_heading(blocks: &[Block], offset: f32) -> Option<usize> {
    blocks
        .iter()
        .rfind(|block| {
            matches!(block.kind, BlockKind::Heading(_)) && block.y <= offset + HEADING_LEEWAY
        })
        .map(|block| block.item)
}

/// What the strip highlights besides the blocks themselves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Marks {
    /// Items holding a find match, sorted and without repeats.
    pub found: Vec<usize>,
    /// The item holding the active find match.
    pub active: Option<usize>,
    /// The heading the reader is in.
    pub heading: Option<usize>,
}

impl Marks {
    /// `matches` lists the item of every find match in order; `active` indexes it.
    pub fn new(blocks: &[Block], offset: f32, matches: &[usize], active: usize) -> Self {
        let mut found = matches.to_vec();
        found.sort_unstable();
        found.dedup();
        Self {
            found,
            active: matches.get(active).copied(),
            heading: current_heading(blocks, offset),
        }
    }

    pub fn is_found(&self, item: usize) -> bool {
        self.found.binary_search(&item).is_ok()
    }
}

/// A bar in minimap coordinates (pixels from the top of the track).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub item: usize,
    pub kind: BlockKind,
    pub y: f32,
    pub height: f32,
}

/// The preview's scroll state and the minimap's track, all in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Geometry {
    pub content_height: f32,
    pub viewport_height: f32,
    pub offset: f32,
    pub track_height: f32,
}

impl Geometry {
    /// Minimap pixels per content pixel: the whole document fits the track,
    /// capped at [`MAX_SCALE`]. Zero when there is nothing to draw.
    pub fn scale(&self) -> f32 {
        if self.content_height <= 0.0 || self.track_height <= 0.0 {
            return 0.0;
        }
        (self.track_height / self.content_height).min(MAX_SCALE)
    }

    /// The largest offset the preview can scroll to.
    pub fn max_offset(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    /// The viewport slider as `(y, height)` in minimap coordinates.
    pub fn slider(&self) -> (f32, f32) {
        let scale = self.scale();
        let offset = self.offset.clamp(0.0, self.max_offset());
        let visible = self.viewport_height.min(self.content_height);
        (offset * scale, (visible * scale).max(MIN_BAR_HEIGHT))
    }

    /// The scroll offset that centres the viewport on a click at `y` (minimap
    /// coordinates), clamped to the scrollable range.
    pub fn offset_for_click(&self, y: f32) -> f32 {
        let scale = self.scale();
        if scale <= 0.0 {
            return 0.0;
        }
        (y / scale - self.viewport_height / 2.0).clamp(0.0, self.max_offset())
    }

    /// Scales content-space blocks into minimap bars.
    pub fn bars(&self, blocks: &[Block]) -> Vec<Bar> {
        let scale = self.scale();
        blocks
            .iter()
            .map(|block| Bar {
                item: block.item,
                kind: block.kind,
                y: block.y * scale,
                height: (block.height * scale).max(MIN_BAR_HEIGHT),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(content: f32, viewport: f32, offset: f32, track: f32) -> Geometry {
        Geometry {
            content_height: content,
            viewport_height: viewport,
            offset,
            track_height: track,
        }
    }

    #[test]
    fn a_long_document_is_scaled_to_fit_the_track() {
        assert_eq!(geometry(10_000.0, 800.0, 0.0, 500.0).scale(), 0.05);
    }

    #[test]
    fn a_short_document_keeps_the_maximum_scale() {
        assert_eq!(geometry(1_000.0, 800.0, 0.0, 500.0).scale(), MAX_SCALE);
    }

    #[test]
    fn an_empty_document_or_track_has_no_scale() {
        assert_eq!(geometry(0.0, 800.0, 0.0, 500.0).scale(), 0.0);
        assert_eq!(geometry(1_000.0, 800.0, 0.0, 0.0).scale(), 0.0);
    }

    #[test]
    fn the_slider_tracks_the_offset_and_viewport() {
        let (y, height) = geometry(10_000.0, 1_000.0, 2_000.0, 500.0).slider();
        assert_eq!((y, height), (100.0, 50.0));
    }

    #[test]
    fn the_slider_stays_inside_the_scrollable_range() {
        let (y, _) = geometry(10_000.0, 1_000.0, 99_999.0, 500.0).slider();
        assert_eq!(y, 9_000.0 * 0.05);
    }

    #[test]
    fn a_click_centres_the_viewport_on_that_point() {
        // Scale 0.05: y=250 is content 5000; centring a 1000px viewport gives 4500.
        assert_eq!(
            geometry(10_000.0, 1_000.0, 0.0, 500.0).offset_for_click(250.0),
            4_500.0
        );
    }

    #[test]
    fn a_click_near_either_end_clamps_to_the_scrollable_range() {
        let g = geometry(10_000.0, 1_000.0, 0.0, 500.0);
        assert_eq!(g.offset_for_click(0.0), 0.0);
        assert_eq!(g.offset_for_click(500.0), 9_000.0);
    }

    #[test]
    fn a_click_on_an_empty_minimap_scrolls_to_the_top() {
        assert_eq!(
            geometry(0.0, 800.0, 0.0, 500.0).offset_for_click(100.0),
            0.0
        );
    }

    #[test]
    fn a_short_bar_holds_one_line_and_a_tall_one_holds_many() {
        assert_eq!(line_count(2.0), 1);
        assert_eq!(line_count(0.0), 1);
        assert_eq!(line_count(30.0), 10);
    }

    #[test]
    fn line_lengths_are_stable_ragged_and_inside_the_column() {
        let lengths: Vec<f32> = (0..8).map(|i| line_length(3, i, 8)).collect();
        assert_eq!(
            lengths,
            (0..8).map(|i| line_length(3, i, 8)).collect::<Vec<_>>()
        );
        assert!(lengths.iter().all(|&l| (0.25..=1.0).contains(&l)));
        assert!(lengths[..7].windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn the_last_line_of_a_block_is_half_as_long_as_it_would_be() {
        assert_eq!(line_length(5, 3, 4), line_length(5, 3, 99) * 0.5);
        assert_eq!(line_length(5, 0, 1), line_length(5, 0, 99));
    }

    #[test]
    fn bars_scale_position_and_keep_a_minimum_height() {
        let g = geometry(10_000.0, 800.0, 0.0, 500.0);
        let bars = g.bars(&[
            Block {
                item: 0,
                kind: BlockKind::Heading(1),
                y: 0.0,
                height: 10.0,
            },
            Block {
                item: 1,
                kind: BlockKind::Paragraph,
                y: 2_000.0,
                height: 400.0,
            },
        ]);
        assert_eq!(bars[0].height, MIN_BAR_HEIGHT);
        assert_eq!((bars[1].y, bars[1].height), (100.0, 20.0));
    }

    fn block(item: usize, kind: BlockKind, y: f32) -> Block {
        Block {
            item,
            kind,
            y,
            height: 20.0,
        }
    }

    #[test]
    fn the_current_heading_is_the_last_one_at_or_above_the_viewport_top() {
        let blocks = [
            block(0, BlockKind::Heading(1), 0.0),
            block(1, BlockKind::Paragraph, 40.0),
            block(2, BlockKind::Heading(2), 500.0),
            block(3, BlockKind::Heading(2), 900.0),
        ];
        assert_eq!(current_heading(&blocks, 0.0), Some(0));
        assert_eq!(current_heading(&blocks, 480.0), Some(2));
        assert_eq!(current_heading(&blocks, 2_000.0), Some(3));
    }

    #[test]
    fn there_is_no_current_heading_before_the_first_one() {
        let blocks = [block(0, BlockKind::Heading(1), 300.0)];
        assert_eq!(current_heading(&blocks, 0.0), None);
        assert_eq!(current_heading(&[], 0.0), None);
    }

    #[test]
    fn marks_dedupe_matches_and_pick_the_active_item() {
        let blocks = [block(0, BlockKind::Heading(1), 0.0)];
        let marks = Marks::new(&blocks, 0.0, &[4, 2, 4, 9], 2);
        assert_eq!(marks.found, vec![2, 4, 9]);
        assert_eq!(marks.active, Some(4));
        assert_eq!(marks.heading, Some(0));
        assert!(marks.is_found(9) && !marks.is_found(3));
    }

    #[test]
    fn marks_with_no_matches_are_empty() {
        let marks = Marks::new(&[], 0.0, &[], 0);
        assert!(marks.found.is_empty() && marks.active.is_none());
    }
}
