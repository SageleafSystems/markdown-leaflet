// SPDX-License-Identifier: GPL-3.0-or-later
//! Time-based tweens shared by the resize-cue fade and the gutter slide. Pure: every function
//! takes the current `Instant`, so tests drive the clock.

use std::time::{Duration, Instant};

/// How progress through a tween maps to distance travelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    /// Slow start (quadratic): the resize cue's fade.
    In,
    /// Fast start, gentle landing (cubic): a gutter arriving.
    Out,
    /// Gentle at both ends (smoothstep): a gutter leaving.
    InOut,
}

impl Easing {
    /// Maps linear `t` in `0.0..=1.0` to eased progress in the same range.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::In => t * t,
            Self::Out => 1.0 - (1.0 - t).powi(3),
            Self::InOut => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// A value moving from `from` to `to` over `duration`, starting at `started`.
#[derive(Debug, Clone, Copy)]
pub struct Tween {
    from: f32,
    to: f32,
    started: Instant,
    duration: Duration,
    easing: Easing,
}

impl Tween {
    pub fn new(from: f32, to: f32, started: Instant, duration: Duration, easing: Easing) -> Self {
        Self {
            from,
            to,
            started,
            duration,
            easing,
        }
    }

    /// Where the tween is heading.
    pub fn target(&self) -> f32 {
        self.to
    }

    /// The value at `now`, and whether the tween has finished. A finished tween returns its
    /// target exactly.
    pub fn sample(&self, now: Instant) -> (f32, bool) {
        let elapsed = now.saturating_duration_since(self.started).as_secs_f32();
        let progress = if self.duration.is_zero() {
            1.0
        } else {
            (elapsed / self.duration.as_secs_f32()).min(1.0)
        };
        if progress >= 1.0 {
            (self.to, true)
        } else {
            (
                self.from + (self.to - self.from) * self.easing.apply(progress),
                false,
            )
        }
    }
}

/// A value in `0.0..=1.0` that eases toward a target, for state that is simply "how far along":
/// a focus expansion, say. It holds only a running tween, so it needs no clock when idle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Eased {
    value: f32,
    tween: Option<Tween>,
}

impl Eased {
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn is_animating(&self) -> bool {
        self.tween.is_some()
    }

    /// Starts easing toward `target`; a no-op if already heading there or resting on it.
    pub fn set_target(&mut self, target: f32, now: Instant, duration: Duration, easing: Easing) {
        if self.tween.is_some_and(|tween| tween.target() == target) {
            return;
        }
        if self.value == target {
            // Already there, perhaps because a tween toward the other end had not moved yet:
            // there is nothing to animate, so stop animating rather than tween 0 to 0.
            self.tween = None;
            return;
        }
        self.tween = Some(Tween::new(self.value, target, now, duration, easing));
    }

    /// Jumps to `target` with no animation.
    pub fn snap_to(&mut self, target: f32) {
        self.value = target;
        self.tween = None;
    }

    pub fn advance(&mut self, now: Instant) {
        let Some(tween) = self.tween else {
            return;
        };
        let (value, done) = tween.sample(now);
        self.value = value;
        if done {
            self.tween = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn every_easing_starts_at_zero_and_ends_at_one() {
        for easing in [Easing::In, Easing::Out, Easing::InOut] {
            assert_eq!(easing.apply(0.0), 0.0);
            assert_eq!(easing.apply(1.0), 1.0);
            let mut last = 0.0;
            for step in 1..=20 {
                let value = easing.apply(step as f32 / 20.0);
                assert!(value >= last, "{easing:?} must not go backwards");
                last = value;
            }
        }
    }

    #[test]
    fn easings_differ_at_the_midpoint() {
        assert!(Easing::In.apply(0.5) < 0.5);
        assert!(Easing::Out.apply(0.5) > 0.5);
        assert!((Easing::InOut.apply(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_tween_lands_exactly_on_its_target() {
        let start = Instant::now();
        let tween = Tween::new(0.25, 1.0, start, SECOND, Easing::Out);
        let (mid, done) = tween.sample(start + SECOND / 2);
        assert!(mid > 0.25 && mid < 1.0 && !done);
        assert_eq!(tween.sample(start + SECOND), (1.0, true));
        assert_eq!(tween.sample(start + SECOND * 5), (1.0, true));
    }

    #[test]
    fn a_zero_duration_tween_finishes_at_once() {
        let start = Instant::now();
        let tween = Tween::new(1.0, 0.0, start, Duration::ZERO, Easing::In);
        assert_eq!(tween.sample(start), (0.0, true));
    }

    #[test]
    fn eased_runs_to_its_target_and_reverses_from_where_it_is() {
        let start = Instant::now();
        let mut eased = Eased::default();
        assert!(!eased.is_animating());
        eased.set_target(1.0, start, SECOND, Easing::Out);
        eased.advance(start + SECOND / 2);
        let halfway = eased.value();
        assert!(halfway > 0.0 && halfway < 1.0);
        // Turning around starts from the current value, not from an end.
        eased.set_target(0.0, start + SECOND / 2, SECOND, Easing::InOut);
        eased.advance(start + SECOND / 2);
        assert_eq!(eased.value(), halfway);
        eased.advance(start + SECOND * 2);
        assert_eq!(eased.value(), 0.0);
        assert!(!eased.is_animating());
    }

    #[test]
    fn eased_turned_around_before_it_moves_just_rests() {
        let start = Instant::now();
        let mut eased = Eased::default();
        eased.set_target(1.0, start, SECOND, Easing::Out);
        eased.set_target(0.0, start, SECOND, Easing::InOut);
        assert!(!eased.is_animating());
        assert_eq!(eased.value(), 0.0);
    }

    #[test]
    fn eased_ignores_a_repeated_or_resting_target() {
        let start = Instant::now();
        let mut eased = Eased::default();
        eased.set_target(0.0, start, SECOND, Easing::Out);
        assert!(!eased.is_animating(), "already at the target");
        eased.set_target(1.0, start, SECOND, Easing::Out);
        eased.set_target(1.0, start + SECOND / 2, SECOND, Easing::Out);
        eased.advance(start + SECOND);
        assert_eq!(
            eased.value(),
            1.0,
            "the second request must not restart the clock"
        );
        eased.snap_to(0.0);
        assert!(!eased.is_animating() && eased.value() == 0.0);
    }
}
