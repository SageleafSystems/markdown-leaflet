// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden-ratio leading for the rendered preview. Pure rules with no `iced`
//! dependency.
//!
//! A fixed line height only suits one line length: long lines need more
//! leading so the eye can find the next line, short ones less. The body ratio
//! therefore grows with the measure (the width available to text) and reaches
//! the golden ratio at the optimal measure of φ⁷ ems, about 29 em or 60 to 70
//! characters. Beyond that it stays at φ.

/// The golden ratio, φ.
pub const PHI: f32 = 1.618_034;

/// The measure, in ems, at which body leading reaches φ.
const OPTIMAL_MEASURE_EM: f32 = PHI * PHI * PHI * PHI * PHI * PHI * PHI;

/// Tightest body leading, for very narrow columns.
const MIN_BODY_RATIO: f32 = 1.25;

/// Relative line height (a multiple of the font size) for body text of
/// `text_size` pixels in a column `measure` pixels wide.
pub fn body_ratio(text_size: f32, measure: f32) -> f32 {
    let fill = measure / (text_size * OPTIMAL_MEASURE_EM);
    (1.0 + fill / PHI).clamp(MIN_BODY_RATIO, PHI)
}

/// Relative line height for headings, which are larger and wrap onto fewer
/// lines, so they take the body's leading scaled down by φ.
pub fn heading_ratio(body: f32) -> f32 {
    1.0 + (body - 1.0) / PHI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_reaches_phi_at_the_optimal_measure() {
        let ratio = body_ratio(16.0, 16.0 * OPTIMAL_MEASURE_EM);
        assert!((ratio - PHI).abs() < 1e-4);
    }

    #[test]
    fn leading_grows_with_width_and_stays_within_bounds() {
        let narrow = body_ratio(16.0, 100.0);
        let medium = body_ratio(16.0, 380.0);
        let wide = body_ratio(16.0, 5000.0);
        assert!(narrow < medium && medium < wide);
        assert_eq!(narrow, MIN_BODY_RATIO);
        assert_eq!(wide, PHI);
    }

    #[test]
    fn headings_sit_tighter_than_body() {
        let body = body_ratio(16.0, 5000.0);
        let heading = heading_ratio(body);
        assert!(heading > 1.0 && heading < body);
        assert!((heading - 1.382).abs() < 1e-3);
    }
}
