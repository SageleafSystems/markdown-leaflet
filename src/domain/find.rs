// SPDX-License-Identifier: GPL-3.0-or-later
//! In-document text search: ASCII case-insensitive, safe on UTF-8 boundaries.

use std::ops::Range;

/// Byte offset of the first match of `needle` in `haystack`, if any.
pub fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    haystack.char_indices().map(|(i, _)| i).find(|&i| {
        haystack
            .get(i..i + needle.len())
            .is_some_and(|part| part.eq_ignore_ascii_case(needle))
    })
}

/// Byte ranges of every non-overlapping match of `needle` in `haystack`.
pub fn match_ranges(haystack: &str, needle: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = find_ascii_case_insensitive(&haystack[cursor..], needle) {
        let start = cursor + offset;
        let end = start + needle.len();
        ranges.push(start..end);
        cursor = end;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_is_ascii_case_insensitive_and_unicode_safe() {
        assert_eq!(find_ascii_case_insensitive("Café FIND", "find"), Some(6));
        assert_eq!(find_ascii_case_insensitive("Café", "afé"), Some(1));
        assert_eq!(find_ascii_case_insensitive("Café", "fe"), None);
    }

    #[test]
    fn match_ranges_are_non_overlapping_and_empty_needle_matches_nothing() {
        assert_eq!(match_ranges("aaaa", "aa"), vec![0..2, 2..4]);
        assert_eq!(match_ranges("Find find", "FIND"), vec![0..4, 5..9]);
        assert!(match_ranges("anything", "").is_empty());
    }
}
