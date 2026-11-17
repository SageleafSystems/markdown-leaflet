// SPDX-License-Identifier: GPL-3.0-or-later
//! Side-gutter state that does not depend on widgets: which side, the drag in
//! progress, the fade of each divider's resize cue, and each card's slide.

pub mod motion;
pub mod resize;

pub use crate::layout::GutterSide;

use crate::layout::{PanelIntent, Placement};
use motion::{Easing, Tween};
use std::time::{Duration, Instant};

pub const FADE_DURATION: Duration = Duration::from_millis(130);
/// A card arriving from its window edge.
pub const SLIDE_OPEN_DURATION: Duration = Duration::from_millis(180);
/// A card leaving toward its window edge.
pub const SLIDE_CLOSE_DURATION: Duration = Duration::from_millis(180);
/// How far into its slide a card is when the toolbar buttons it replaces have finished fading:
/// they fade out over the first 60% of an opening slide and back in over the last 60% of a
/// closing one.
pub const TOOLBAR_FADE_SPAN: f32 = 0.6;
/// The interval of the animation tick, which runs only while something is moving.
pub const ANIMATION_FRAME: Duration = Duration::from_millis(16);

/// Which tab the right gutter shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Outline,
    Details,
}

/// Which tab the left gutter shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftTab {
    Files,
    Recent,
}

/// Everything about the two side gutters: visibility, active side, widths,
/// the drag in progress, and the divider cues.
#[derive(Debug, Clone)]
pub struct GutterState {
    pub sidebar_visible: bool,
    pub inspector_visible: bool,
    pub left_tab: LeftTab,
    pub right_tab: RightTab,
    /// The gutter that wins when both are open but only one fits.
    pub active: GutterSide,
    pub sidebar_width: f32,
    pub right_width: f32,
    pub drag: Option<ResizeDrag>,
    pub hovered: Option<GutterSide>,
    cues: [ResizeCue; 2],
    slides: [Slide; 2],
}

impl Default for GutterState {
    fn default() -> Self {
        Self {
            sidebar_visible: true,
            inspector_visible: false,
            left_tab: LeftTab::Files,
            right_tab: RightTab::Outline,
            active: GutterSide::Left,
            sidebar_width: resize::LEFT_DEFAULT,
            right_width: resize::RIGHT_DEFAULT,
            drag: None,
            hovered: None,
            cues: [ResizeCue::default(); 2],
            slides: [Slide::default(); 2],
        }
    }
}

impl GutterState {
    /// What the user asked for, for the layout planner to resolve against the window.
    pub fn intent(&self) -> PanelIntent {
        PanelIntent {
            left_open: self.sidebar_visible,
            right_open: self.inspector_visible,
            active: self.active,
            left_width: self.sidebar_width,
            right_width: self.right_width,
        }
    }

    /// Opens `side` and makes it the gutter that wins when only one fits.
    pub fn reveal(&mut self, side: GutterSide) {
        match side {
            GutterSide::Left => self.sidebar_visible = true,
            GutterSide::Right => self.inspector_visible = true,
        }
        self.active = side;
        // Revealing a card that is still on its way out turns it around.
        self.slides[side.index()].reopen_if_closing(Instant::now());
    }

    pub fn close(&mut self, side: GutterSide) {
        match side {
            GutterSide::Left => self.sidebar_visible = false,
            GutterSide::Right => self.inspector_visible = false,
        }
    }

    pub fn set_width(&mut self, side: GutterSide, width: f32) {
        match side {
            GutterSide::Left => self.sidebar_width = width,
            GutterSide::Right => self.right_width = width,
        }
    }

    pub fn width(&self, side: GutterSide) -> f32 {
        match side {
            GutterSide::Left => self.sidebar_width,
            GutterSide::Right => self.right_width,
        }
    }

    pub fn cue_opacity(&self, side: GutterSide) -> f32 {
        self.cues[side.index()].opacity()
    }

    pub fn set_cue(&mut self, side: GutterSide, visible: bool) {
        self.cues[side.index()].set_visible(visible, Instant::now());
    }

    /// Starts `side`'s card sliding in from its window edge.
    pub fn slide_open(&mut self, side: GutterSide) {
        self.slides[side.index()].open(Instant::now());
    }

    /// Starts `side`'s card sliding out. `held` is where the plan had it, so the view can keep
    /// drawing it until it is gone.
    pub fn slide_close(&mut self, side: GutterSide, held: Held) {
        self.slides[side.index()].close(held, Instant::now());
    }

    /// Ends every slide where it stands, as a window resize does.
    pub fn snap_slides(&mut self) {
        for slide in &mut self.slides {
            slide.snap();
        }
    }

    /// How far `side`'s card has arrived: `0.0` off the window edge, `1.0` in place.
    pub fn slide_progress(&self, side: GutterSide) -> f32 {
        self.slides[side.index()].progress()
    }

    /// How visible the toolbar's own copy of this side's buttons is while the card slides: `1.0`
    /// with the card away, `0.0` once it has covered them. `None` when no slide is running (the
    /// toolbar then shows them, or not, as the plan says). It follows the card's position, so it
    /// reverses with it.
    pub fn toolbar_fade(&self, side: GutterSide) -> Option<f32> {
        let slide = &self.slides[side.index()];
        slide
            .is_animating()
            .then(|| (1.0 - slide.progress() / TOOLBAR_FADE_SPAN).clamp(0.0, 1.0))
    }

    /// Where a card that has left the plan is still sliding out, if it is.
    pub fn slide_held(&self, side: GutterSide) -> Option<Held> {
        self.slides[side.index()].held()
    }

    /// Moves every running cue and slide to the current time.
    pub fn advance_animations(&mut self) {
        let now = Instant::now();
        for cue in &mut self.cues {
            cue.advance(now);
        }
        for slide in &mut self.slides {
            slide.advance(now);
        }
    }

    pub fn is_animating(&self) -> bool {
        self.cues.iter().any(ResizeCue::is_animating) || self.slides.iter().any(Slide::is_animating)
    }
}

/// A divider drag: the pointer x and width captured on press.
#[derive(Debug, Clone, Copy)]
pub struct ResizeDrag {
    pub side: GutterSide,
    pub origin_x: f32,
    pub initial_width: f32,
    pub max_width: f32,
}

/// Animated opacity of a divider's line, grip, and edge highlight.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResizeCue {
    opacity: f32,
    tween: Option<Tween>,
}

impl ResizeCue {
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn is_animating(&self) -> bool {
        self.tween.is_some()
    }

    /// Starts fading toward visible or hidden; a no-op if already heading there.
    pub fn set_visible(&mut self, visible: bool, now: Instant) {
        let to = if visible { 1.0 } else { 0.0 };
        if self.tween.is_some_and(|tween| tween.target() == to)
            || (self.tween.is_none() && self.opacity == to)
        {
            return;
        }
        self.tween = Some(Tween::new(self.opacity, to, now, FADE_DURATION, Easing::In));
    }

    pub fn advance(&mut self, now: Instant) {
        let Some(tween) = self.tween else {
            return;
        };
        let (value, done) = tween.sample(now);
        self.opacity = value;
        if done {
            self.tween = None;
        }
    }
}

/// Where the plan had a card when it began to leave, so the view can keep drawing it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    pub placement: Placement,
    pub width: f32,
}

/// One card's slide between its window edge and its place. Only a running slide has a position;
/// an idle one is fully in place (when the plan shows the card) or not drawn (when it does not),
/// so state set directly, such as a restored workspace, needs no matching slide state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Slide {
    /// The position while a tween runs: `0.0` off the edge, `1.0` in place.
    value: f32,
    tween: Option<Tween>,
    held: Option<Held>,
}

impl Slide {
    pub fn progress(&self) -> f32 {
        if self.tween.is_some() {
            self.value
        } else {
            1.0
        }
    }

    pub fn is_animating(&self) -> bool {
        self.tween.is_some()
    }

    pub fn held(&self) -> Option<Held> {
        self.held
    }

    fn closing(&self) -> bool {
        self.tween.is_some_and(|tween| tween.target() == 0.0)
    }

    /// Slides in from the current position, or from the edge when idle.
    pub fn open(&mut self, now: Instant) {
        if self.tween.is_some_and(|tween| tween.target() == 1.0) {
            return;
        }
        let from = if self.tween.is_some() {
            self.value
        } else {
            0.0
        };
        self.value = from;
        self.held = None;
        self.tween = Some(Tween::new(from, 1.0, now, SLIDE_OPEN_DURATION, Easing::Out));
    }

    /// Slides out from the current position, or from in place when idle. Closing twice keeps the
    /// first `held`.
    pub fn close(&mut self, held: Held, now: Instant) {
        if self.closing() {
            return;
        }
        let from = if self.tween.is_some() {
            self.value
        } else {
            1.0
        };
        self.value = from;
        self.held = Some(held);
        self.tween = Some(Tween::new(
            from,
            0.0,
            now,
            SLIDE_CLOSE_DURATION,
            Easing::InOut,
        ));
    }

    /// Turns a card that is leaving back around, from where it is.
    pub fn reopen_if_closing(&mut self, now: Instant) {
        if self.closing() {
            self.open(now);
        }
    }

    /// Ends the slide where it stands: a card that was leaving is gone, one arriving is in place.
    pub fn snap(&mut self) {
        self.tween = None;
        self.held = None;
    }

    pub fn advance(&mut self, now: Instant) {
        let Some(tween) = self.tween else {
            return;
        };
        let (value, done) = tween.sample(now);
        self.value = value;
        if done {
            self.tween = None;
            if tween.target() == 0.0 {
                self.held = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_fades_in_and_settles() {
        let start = Instant::now();
        let mut cue = ResizeCue::default();
        cue.set_visible(true, start);
        assert!(cue.is_animating());
        cue.advance(start + FADE_DURATION / 2);
        assert!(cue.opacity() > 0.0 && cue.opacity() < 1.0);
        cue.advance(start + FADE_DURATION);
        assert_eq!(cue.opacity(), 1.0);
        assert!(!cue.is_animating());
    }

    #[test]
    fn gutter_state_starts_with_the_files_gutter_and_reports_animation() {
        let mut gutters = GutterState::default();
        assert!(gutters.sidebar_visible && !gutters.inspector_visible);
        assert_eq!(gutters.active, GutterSide::Left);
        assert!(!gutters.is_animating());
        gutters.set_cue(GutterSide::Right, true);
        assert!(gutters.is_animating());
        assert_eq!(gutters.cue_opacity(GutterSide::Left), 0.0);
    }

    #[test]
    fn reveal_opens_a_side_and_makes_it_active() {
        let mut gutters = GutterState::default();
        gutters.reveal(GutterSide::Right);
        assert!(gutters.inspector_visible && gutters.sidebar_visible);
        assert_eq!(gutters.active, GutterSide::Right);
        assert_eq!(gutters.intent().active, GutterSide::Right);
    }

    #[test]
    fn cue_ignores_a_redundant_target() {
        let start = Instant::now();
        let mut cue = ResizeCue::default();
        cue.set_visible(false, start);
        assert!(!cue.is_animating());
        cue.set_visible(true, start);
        // A second request to show must not restart the clock.
        cue.set_visible(true, start + ANIMATION_FRAME);
        cue.advance(start + FADE_DURATION);
        assert_eq!(cue.opacity(), 1.0);
        assert!(!cue.is_animating());
    }

    fn held() -> Held {
        Held {
            placement: Placement::Docked,
            width: 260.0,
        }
    }

    #[test]
    fn an_idle_slide_is_in_place() {
        let slide = Slide::default();
        assert_eq!(slide.progress(), 1.0);
        assert!(!slide.is_animating() && slide.held().is_none());
    }

    #[test]
    fn opening_slides_in_from_the_edge_and_settles() {
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.open(start);
        assert_eq!(slide.progress(), 0.0);
        slide.advance(start + SLIDE_OPEN_DURATION / 2);
        assert!(slide.progress() > 0.0 && slide.progress() < 1.0);
        slide.advance(start + SLIDE_OPEN_DURATION);
        assert_eq!(slide.progress(), 1.0);
        assert!(!slide.is_animating());
    }

    #[test]
    fn closing_keeps_the_card_held_until_it_is_gone() {
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.close(held(), start);
        assert_eq!(slide.progress(), 1.0);
        slide.advance(start + SLIDE_CLOSE_DURATION / 2);
        assert!(slide.progress() > 0.0 && slide.progress() < 1.0);
        assert_eq!(slide.held(), Some(held()));
        slide.advance(start + SLIDE_CLOSE_DURATION);
        assert!(!slide.is_animating());
        assert!(slide.held().is_none());
    }

    #[test]
    fn toggling_mid_slide_reverses_from_where_it_is() {
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.close(held(), start);
        let halfway = start + SLIDE_CLOSE_DURATION / 2;
        slide.advance(halfway);
        let before = slide.progress();
        slide.open(halfway);
        assert_eq!(slide.progress(), before, "reversing must not jump");
        assert!(slide.held().is_none());
        slide.advance(halfway + SLIDE_OPEN_DURATION);
        assert_eq!(slide.progress(), 1.0);
    }

    #[test]
    fn closing_twice_keeps_the_first_hold_and_clock() {
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.close(held(), start);
        let other = Held {
            placement: Placement::Overlay,
            width: 1.0,
        };
        slide.close(other, start + ANIMATION_FRAME);
        assert_eq!(slide.held(), Some(held()));
        slide.advance(start + SLIDE_CLOSE_DURATION);
        assert!(!slide.is_animating());
    }

    #[test]
    fn reopen_only_turns_a_closing_card_around() {
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.reopen_if_closing(start);
        assert!(!slide.is_animating(), "an idle card has nothing to reverse");
        slide.close(held(), start);
        slide.reopen_if_closing(start + ANIMATION_FRAME);
        slide.advance(start + ANIMATION_FRAME + SLIDE_OPEN_DURATION);
        assert_eq!(slide.progress(), 1.0);
        assert!(slide.held().is_none());
    }

    #[test]
    fn snap_ends_a_slide_where_it_stands() {
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.close(held(), start);
        slide.snap();
        assert!(!slide.is_animating() && slide.held().is_none());
    }

    #[test]
    fn gutter_state_reports_a_slide_as_animation() {
        let mut gutters = GutterState::default();
        assert!(!gutters.is_animating());
        gutters.slide_close(GutterSide::Left, held());
        assert!(gutters.is_animating());
        assert_eq!(gutters.slide_held(GutterSide::Left), Some(held()));
        assert!(gutters.slide_held(GutterSide::Right).is_none());
        gutters.snap_slides();
        assert!(!gutters.is_animating());
    }

    #[test]
    fn toolbar_buttons_fade_out_as_a_card_arrives_and_back_in_as_it_leaves() {
        let mut gutters = GutterState::default();
        assert_eq!(gutters.toolbar_fade(GutterSide::Left), None);
        let start = Instant::now();
        let mut slide = Slide::default();
        slide.open(start);
        gutters.slides[0] = slide;
        // At the start of an arrival the buttons are still fully there; by the end, gone.
        assert_eq!(gutters.toolbar_fade(GutterSide::Left), Some(1.0));
        gutters.slides[0].advance(start + SLIDE_OPEN_DURATION / 2);
        let mid = gutters.toolbar_fade(GutterSide::Left).unwrap();
        assert!(mid < 1.0);
        gutters.slides[0].advance(start + SLIDE_OPEN_DURATION);
        assert_eq!(gutters.toolbar_fade(GutterSide::Left), None);

        // Leaving: the card covers them at first, then they return.
        gutters.slide_close(GutterSide::Right, held());
        assert_eq!(gutters.toolbar_fade(GutterSide::Right), Some(0.0));
        gutters.slides[1].advance(start + SLIDE_CLOSE_DURATION * 100);
        assert_eq!(gutters.toolbar_fade(GutterSide::Right), None);
    }

    #[test]
    fn revealing_a_leaving_card_reverses_it() {
        let mut gutters = GutterState::default();
        gutters.slide_close(GutterSide::Left, held());
        gutters.reveal(GutterSide::Left);
        assert!(gutters.slide_held(GutterSide::Left).is_none());
        assert!(gutters.is_animating());
    }
}
