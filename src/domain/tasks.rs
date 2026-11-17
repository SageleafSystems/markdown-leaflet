// SPDX-License-Identifier: GPL-3.0-or-later
//! GFM task-list checkboxes: locating a checkbox's exact source byte so a toggle can flip it in
//! place, and the completed/total counts a task list's progress indicator shows.
//!
//! `iced::widget::markdown::parse` throws away the byte ranges pulldown-cmark computed while
//! building its `Item` tree, so there is no source position on a `Bullet::Task` to read back.
//! Rather than re-deriving iced's private top-level-item grouping, [`task_offsets`] (over the
//! already-parsed `Item` tree) and [`marker_ranges`] (over the raw body text) each walk the
//! document in strict source order on their own, independently reaching the same `Bullet::Task`
//! at the same position: pulldown-cmark's own event stream is what built the `Item` tree, so the
//! Nth task counted either way names the same checkbox.

use iced::widget::markdown::{Bullet, Item};
use pulldown_cmark::{Event, Options, Parser};
use std::ops::Range;

/// One checkbox, named the way `domain::selection::BlockId` names a selectable leaf: by its
/// top-level `Item` and a sequential index within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId {
    /// Index of the top-level `markdown::Item` this checkbox belongs to.
    pub item: usize,
    /// Index of this checkbox within that item, in document order (depth-first over nested
    /// lists and quotes).
    pub task: usize,
}

/// How many tasks appear in each top-level item, as a prefix sum: `task_offsets(items)[i]` is
/// the number of tasks in items before index `i`, so `offsets[id.item] + id.task` is a
/// whole-document, flat task index lining up with [`marker_ranges`].
pub fn task_offsets(items: &[Item]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(items.len());
    let mut total = 0;
    for item in items {
        offsets.push(total);
        total += count_in_item(item);
    }
    offsets
}

fn count_in_item(item: &Item) -> usize {
    counts_in_item(item).1
}

/// `(done, total)` of every task in `item`'s own subtree, recursing depth-first the same way
/// [`toggle`] walks and pulldown-cmark emits `TaskListMarker` events, so this and
/// [`marker_ranges`] always agree on order.
fn counts_in_item(item: &Item) -> (usize, usize) {
    match item {
        Item::List { bullets, .. } => counts_in_bullets(bullets),
        Item::Quote(items) => items.iter().fold((0, 0), |(done, total), item| {
            let (d, t) = counts_in_item(item);
            (done + d, total + t)
        }),
        _ => (0, 0),
    }
}

fn counts_in_bullets(bullets: &[Bullet]) -> (usize, usize) {
    bullets.iter().fold((0, 0), |(done, total), bullet| {
        let (own_done, own_total) = match bullet {
            Bullet::Task { done: true, .. } => (1, 1),
            Bullet::Task { done: false, .. } => (0, 1),
            Bullet::Point { .. } => (0, 0),
        };
        let (nested_done, nested_total) =
            bullet_items(bullet).iter().fold((0, 0), |(d, t), item| {
                let (id, it) = counts_in_item(item);
                (d + id, t + it)
            });
        (
            done + own_done + nested_done,
            total + own_total + nested_total,
        )
    })
}

fn bullet_items(bullet: &Bullet) -> &[Item] {
    match bullet {
        Bullet::Point { items } | Bullet::Task { items, .. } => items,
    }
}

/// The exact byte range of every task marker's checked/unchecked character (the single char
/// between `[` and `]`) in `body`, in document order. Index `i` is the same checkbox
/// `task_offsets` counts as flat index `i`.
pub fn marker_ranges(body: &str) -> Vec<Range<usize>> {
    let options = Options::ENABLE_TASKLISTS | Options::ENABLE_TABLES;
    Parser::new_ext(body, options)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::TaskListMarker(_) => {
                // `range` spans from the list marker's optional leading indent through the
                // closing `]`; find the opening `[` rather than assume no indent.
                let slice = &body[range.clone()];
                let open = slice.find('[')?;
                let at = range.start + open + 1;
                Some(at..at + 1)
            }
            _ => None,
        })
        .collect()
}

/// Flips the `done` field of the `index`th `Bullet::Task` in `items`, in document order (the
/// same order [`task_offsets`] counts). Returns `false` if `index` is out of range.
pub fn toggle(items: &mut [Item], index: usize) -> bool {
    let mut remaining = index;
    for item in items {
        if toggle_in_item(item, &mut remaining) {
            return true;
        }
    }
    false
}

fn toggle_in_item(item: &mut Item, remaining: &mut usize) -> bool {
    match item {
        Item::List { bullets, .. } => {
            for bullet in bullets {
                if let Bullet::Task { done, .. } = bullet {
                    if *remaining == 0 {
                        *done = !*done;
                        return true;
                    }
                    *remaining -= 1;
                }
                let (Bullet::Point { items } | Bullet::Task { items, .. }) = bullet;
                for nested in items {
                    if toggle_in_item(nested, remaining) {
                        return true;
                    }
                }
            }
            false
        }
        Item::Quote(items) => items.iter_mut().any(|item| toggle_in_item(item, remaining)),
        _ => false,
    }
}

/// A task list's completed/total count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

impl Progress {
    /// Completion as a percentage in `0.0..=100.0`. `0.0` for a zero-task list rather than
    /// dividing by zero (callers skip that case via [`progress_of`] returning `None`, but the
    /// math stays safe either way).
    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            100.0 * self.done as f32 / self.total as f32
        }
    }
}

/// The completed/total count across `bullets`' own subtree, including nested task lists.
/// `None` when the list holds no tasks at all, so an ordinary list shows no indicator.
pub fn progress_of(bullets: &[Bullet]) -> Option<Progress> {
    let (done, total) = counts_in_bullets(bullets);
    (total > 0).then_some(Progress { done, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::markdown;

    fn parse(source: &str) -> Vec<Item> {
        markdown::parse(source).collect()
    }

    #[test]
    fn offsets_and_marker_ranges_agree_on_a_flat_list() {
        let source = "- [x] one\n- [ ] two\n- [x] three\n";
        let items = parse(source);
        let offsets = task_offsets(&items);
        assert_eq!(offsets, vec![0]);
        let ranges = marker_ranges(source);
        assert_eq!(ranges.len(), 3);
        for range in &ranges {
            assert_eq!(range.end - range.start, 1);
        }
        assert_eq!(&source[ranges[0].clone()], "x");
        assert_eq!(&source[ranges[1].clone()], " ");
        assert_eq!(&source[ranges[2].clone()], "x");
    }

    #[test]
    fn offsets_and_marker_ranges_agree_across_nested_and_mixed_lists() {
        let source = "\
# Heading

- [ ] a
  - [x] a.1
  - [ ] a.2
- [x] b

1. [ ] c
2. [x] d
   1. [ ] d.1
";
        let items = parse(source);
        let offsets = task_offsets(&items);
        // item 0 is the heading (no tasks), item 1 the unordered list (4 tasks: a, a.1, a.2, b),
        // item 2 the ordered list (3 tasks: c, d, d.1).
        assert_eq!(offsets, vec![0, 0, 4]);
        let ranges = marker_ranges(source);
        assert_eq!(ranges.len(), 7);
        let chars: Vec<&str> = ranges.iter().map(|r| &source[r.clone()]).collect();
        assert_eq!(chars, vec![" ", "x", " ", "x", " ", "x", " "]);
    }

    #[test]
    fn toggle_flips_only_the_targeted_bullet() {
        let source = "- [ ] a\n  - [x] a.1\n- [x] b\n";
        let mut items = parse(source);
        assert!(toggle(&mut items, 1));
        let Item::List { bullets, .. } = &items[0] else {
            panic!("expected a list");
        };
        let Bullet::Task { done, items } = &bullets[0] else {
            panic!("expected a task");
        };
        assert!(!done, "the parent task is untouched");
        let Item::List {
            bullets: nested, ..
        } = items
            .iter()
            .find(|item| matches!(item, Item::List { .. }))
            .expect("the nested list")
        else {
            unreachable!()
        };
        let Bullet::Task { done, .. } = &nested[0] else {
            panic!("expected a nested task");
        };
        assert!(!done, "a.1 flipped from checked to unchecked");
    }

    #[test]
    fn toggle_out_of_range_changes_nothing() {
        let mut items = parse("- [ ] a\n");
        assert!(!toggle(&mut items, 5));
    }

    #[test]
    fn progress_counts_nested_tasks_without_double_counting() {
        let items = parse("- [x] a\n  - [x] a.1\n  - [ ] a.2\n- [ ] b\n");
        let Item::List { bullets, .. } = &items[0] else {
            panic!("expected a list");
        };
        let progress = progress_of(bullets).expect("a task list");
        assert_eq!(progress.done, 2);
        assert_eq!(progress.total, 4);
        assert!((progress.percent() - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn an_ordinary_list_has_no_progress() {
        let items = parse("- a\n- b\n");
        let Item::List { bullets, .. } = &items[0] else {
            panic!("expected a list");
        };
        assert!(progress_of(bullets).is_none());
    }

    #[test]
    fn a_nested_list_reports_its_own_progress_independently() {
        let items = parse("- [ ] a\n  - [x] a.1\n  - [x] a.2\n");
        let Item::List { bullets, .. } = &items[0] else {
            panic!("expected a list");
        };
        let Bullet::Task {
            items: nested_items,
            ..
        } = &bullets[0]
        else {
            panic!("expected a task");
        };
        let Item::List {
            bullets: nested, ..
        } = nested_items
            .iter()
            .find(|item| matches!(item, Item::List { .. }))
            .expect("the nested list")
        else {
            unreachable!()
        };
        let nested_progress = progress_of(nested).expect("nested tasks");
        assert_eq!((nested_progress.done, nested_progress.total), (2, 2));
        let parent_progress = progress_of(bullets).expect("parent tasks");
        assert_eq!((parent_progress.done, parent_progress.total), (2, 3));
    }
}
