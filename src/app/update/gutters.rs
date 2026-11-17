// SPDX-License-Identifier: GPL-3.0-or-later
//! Side gutters: visibility, tabs, and resizing.

use crate::app::{Message, State};
use crate::app::{SEARCH_COLLAPSE_DURATION, SEARCH_EXPAND_DURATION};
use crate::gutter::motion::Easing;
use crate::gutter::{GutterSide, Held, ResizeDrag, resize};
use crate::layout::{Placement, budget};
use iced::Task;

impl State {
    /// Whether a card in this placement slides. The compact strip is not a card, and a disabled
    /// `[animation]` setting makes every change instant.
    fn slides(&self, placement: Placement) -> bool {
        self.config.animation.enabled && matches!(placement, Placement::Docked | Placement::Overlay)
    }

    /// Starts `side`'s card sliding out of the window, if the plan shows it as a card. Call it
    /// before the gutter is closed, while the plan still has the card.
    fn slide_out(&mut self, side: GutterSide) {
        let plan = self.layout_plan();
        let placement = plan.placement(side);
        if self.slides(placement) {
            self.gutters.slide_close(
                side,
                Held {
                    placement,
                    width: plan.width(side),
                },
            );
        }
    }

    /// Opens `side` and, if that brings in a card that was not shown, slides it in.
    fn reveal_with_slide(&mut self, side: GutterSide) {
        let was_shown = self.layout_plan().shows(side);
        self.gutters.reveal(side);
        if !was_shown && self.slides(self.layout_plan().placement(side)) {
            self.gutters.slide_open(side);
        }
    }

    pub(crate) fn update_gutters(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePanel(side) => {
                let plan = self.layout_plan();
                if plan.shows(side) {
                    self.slide_out(side);
                    self.gutters.close(side);
                    // The other gutter may be open but squeezed out of the window. Closing
                    // the one in view must not make it pop up in its place.
                    if !plan.shows(side.other()) {
                        self.gutters.close(side.other());
                    }
                } else {
                    self.reveal_with_slide(side);
                }
                Task::none()
            }
            Message::SelectRightTab(tab) => {
                self.gutters.right_tab = tab;
                self.reveal_with_slide(GutterSide::Right);
                Task::none()
            }
            Message::SelectLeftTab(tab) => {
                // Unlike the right gutter's tabs, this doubles as the Files/Recent toggle
                // button: reclicking the active tab closes the gutter instead of being a no-op.
                let already_showing =
                    self.layout_plan().shows(GutterSide::Left) && self.gutters.left_tab == tab;
                self.gutters.left_tab = tab;
                if already_showing {
                    self.slide_out(GutterSide::Left);
                    self.gutters.close(GutterSide::Left);
                } else {
                    self.reveal_with_slide(GutterSide::Left);
                }
                Task::none()
            }
            Message::ResizeDragStart(side, origin_x) => {
                self.gutters.drag = Some(ResizeDrag {
                    side,
                    origin_x,
                    initial_width: self.gutters.width(side),
                    max_width: self.layout_plan().drag_max(side),
                });
                self.gutters.set_cue(side, true);
                Task::none()
            }
            Message::SidebarDragged(cursor_x) => {
                if let Some(drag) = self.gutters.drag {
                    let direction = if drag.side == GutterSide::Left {
                        1.0
                    } else {
                        -1.0
                    };
                    let width = resize::drag_width(
                        drag.initial_width,
                        drag.origin_x,
                        cursor_x,
                        direction,
                        drag.max_width,
                        budget::min_width(drag.side),
                    );
                    self.gutters.set_width(drag.side, width);
                }
                Task::none()
            }
            Message::SidebarDragEnd => {
                if let Some(drag) = self.gutters.drag.take() {
                    let settled = resize::settle(
                        self.gutters.width(drag.side),
                        drag.max_width,
                        budget::min_width(drag.side),
                    );
                    self.gutters.set_width(drag.side, settled);
                }
                for side in [GutterSide::Left, GutterSide::Right] {
                    self.gutters
                        .set_cue(side, self.gutters.hovered == Some(side));
                }
                Task::none()
            }
            Message::ResizeZoneEntered(side) => {
                self.gutters.hovered = Some(side);
                self.gutters.set_cue(side, true);
                Task::none()
            }
            Message::ResizeZoneExited(side) => {
                if self.gutters.hovered == Some(side) {
                    self.gutters.hovered = None;
                }
                self.gutters.set_cue(
                    side,
                    self.gutters.drag.is_some_and(|drag| drag.side == side),
                );
                Task::none()
            }
            Message::AnimationTick => {
                let now = std::time::Instant::now();
                self.gutters.advance_animations();
                self.search_expand.advance(now);
                self.copy_flash.retain(|_, tween| !tween.sample(now).1);
                Task::none()
            }
            Message::SearchFocused(focused) => {
                self.search_focused = focused;
                let target = if focused { 1.0 } else { 0.0 };
                if self.config.animation.enabled {
                    let (duration, easing) = if focused {
                        (SEARCH_EXPAND_DURATION, Easing::Out)
                    } else {
                        (SEARCH_COLLAPSE_DURATION, Easing::InOut)
                    };
                    self.search_expand.set_target(
                        target,
                        std::time::Instant::now(),
                        duration,
                        easing,
                    );
                } else {
                    self.search_expand.snap_to(target);
                }
                Task::none()
            }
            _ => unreachable!("gutters messages are routed by State::update"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Viewport;

    fn state(width: f32, active: GutterSide) -> State {
        let mut state = State::blank();
        state.viewport = Viewport::new(width);
        state.gutters.sidebar_visible = true;
        state.gutters.inspector_visible = true;
        state.gutters.active = active;
        state
    }

    #[test]
    fn focusing_the_search_starts_expanding_it_and_blurring_turns_it_back() {
        let mut state = State::blank();
        assert_eq!(state.search_expand.value(), 0.0);
        let _ = state.update_gutters(Message::SearchFocused(true));
        assert!(state.search_focused);
        assert!(state.search_expand.is_animating());
        // The expansion eases in; it is not a jump.
        assert_eq!(state.search_expand.value(), 0.0);
        let _ = state.update_gutters(Message::SearchFocused(false));
        assert!(!state.search_focused);
        assert!(!state.search_expand.is_animating(), "it never left rest");
    }

    #[test]
    fn disabling_animation_makes_the_search_expand_instantly() {
        let mut state = State::blank();
        state.config.animation.enabled = false;
        let _ = state.update_gutters(Message::SearchFocused(true));
        assert_eq!(state.search_expand.value(), 1.0);
        assert!(!state.search_expand.is_animating());
        let _ = state.update_gutters(Message::SearchFocused(false));
        assert_eq!(state.search_expand.value(), 0.0);
    }

    #[test]
    fn closing_a_docked_card_holds_it_while_it_slides_out() {
        let mut state = state(1400.0, GutterSide::Left);
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.layout_plan().shows(GutterSide::Left));
        let held = state.gutters.slide_held(GutterSide::Left).expect("held");
        assert_eq!(held.placement, Placement::Docked);
        assert_eq!(held.width, state.gutters.sidebar_width);
        assert!(state.gutters.is_animating());
    }

    #[test]
    fn opening_a_card_slides_it_in_and_toggling_again_reverses_it() {
        let mut state = state(1400.0, GutterSide::Left);
        state.gutters.sidebar_visible = false;
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(state.gutters.is_animating());
        assert!(state.gutters.slide_held(GutterSide::Left).is_none());
        // Toggling mid-slide closes it again, still moving from where it was.
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(state.gutters.slide_held(GutterSide::Left).is_some());
    }

    #[test]
    fn the_compact_strip_never_slides() {
        let mut state = state(600.0, GutterSide::Left);
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.gutters.is_animating());
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.gutters.is_animating());
    }

    #[test]
    fn disabling_animation_makes_toggles_instant() {
        let mut state = state(1400.0, GutterSide::Left);
        state.config.animation.enabled = false;
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.gutters.is_animating());
        assert!(state.gutters.slide_held(GutterSide::Left).is_none());
    }

    #[test]
    fn hiding_files_in_a_wide_window_works_when_the_right_gutter_is_active() {
        let mut state = state(1400.0, GutterSide::Right);
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.gutters.sidebar_visible);
        assert!(state.gutters.inspector_visible);
    }

    #[test]
    fn toggling_a_panel_hidden_by_the_medium_layout_reveals_it() {
        let mut state = state(900.0, GutterSide::Right);
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(state.gutters.sidebar_visible);
        assert_eq!(state.gutters.active, GutterSide::Left);
        assert!(state.layout_plan().shows(GutterSide::Left));
        assert!(!state.layout_plan().shows(GutterSide::Right));
    }

    #[test]
    fn closing_the_shown_panel_does_not_pop_open_the_one_squeezed_out() {
        for shown in [GutterSide::Left, GutterSide::Right] {
            let mut state = state(900.0, shown);
            let _ = state.update_gutters(Message::TogglePanel(shown));
            let plan = state.layout_plan();
            assert!(
                !plan.shows(GutterSide::Left) && !plan.shows(GutterSide::Right),
                "closing {shown:?} should leave nothing showing"
            );
        }
    }

    #[test]
    fn closing_a_panel_keeps_a_second_one_that_is_still_shown() {
        let mut state = state(1400.0, GutterSide::Right);
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Right));
        assert!(state.layout_plan().shows(GutterSide::Left));
    }

    #[test]
    fn files_in_a_compact_window_toggle_as_a_strip_beside_the_right_overlay() {
        let mut state = state(600.0, GutterSide::Right);
        let plan = state.layout_plan();
        assert!(plan.shows(GutterSide::Left) && plan.shows(GutterSide::Right));
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.layout_plan().shows(GutterSide::Left));
        assert!(state.layout_plan().shows(GutterSide::Right));
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(state.layout_plan().shows(GutterSide::Left));
    }

    #[test]
    fn drag_budget_comes_from_the_stored_viewport() {
        let mut state = state(1120.0, GutterSide::Left);
        let _ = state.update_gutters(Message::ResizeDragStart(GutterSide::Right, 500.0));
        // 1120 - 420 preview - 32 chrome - 300 left (both default widths still fit at 1120).
        assert_eq!(state.gutters.drag.map(|d| d.max_width), Some(368.0));
    }

    #[test]
    fn selecting_a_left_tab_opens_the_gutter_on_that_tab() {
        let mut state = state(1400.0, GutterSide::Left);
        state.gutters.sidebar_visible = false;
        let _ = state.update_gutters(Message::SelectLeftTab(crate::gutter::LeftTab::Recent));
        assert!(state.layout_plan().shows(GutterSide::Left));
        assert_eq!(state.gutters.left_tab, crate::gutter::LeftTab::Recent);
    }

    #[test]
    fn selecting_the_already_active_left_tab_closes_the_gutter() {
        let mut state = state(1400.0, GutterSide::Left);
        state.gutters.left_tab = crate::gutter::LeftTab::Files;
        let _ = state.update_gutters(Message::SelectLeftTab(crate::gutter::LeftTab::Files));
        assert!(!state.layout_plan().shows(GutterSide::Left));
    }

    #[test]
    fn switching_left_tab_while_open_switches_without_closing() {
        let mut state = state(1400.0, GutterSide::Left);
        state.gutters.left_tab = crate::gutter::LeftTab::Files;
        let _ = state.update_gutters(Message::SelectLeftTab(crate::gutter::LeftTab::Recent));
        assert!(state.layout_plan().shows(GutterSide::Left));
        assert_eq!(state.gutters.left_tab, crate::gutter::LeftTab::Recent);
    }

    // Issue #70: Welcome shares the same `GutterState`/`layout_plan` as Folder mode, so its left
    // panel toggles the same way.

    #[test]
    fn welcome_can_toggle_its_left_panel_open_and_closed() {
        let mut state = state(1400.0, GutterSide::Left);
        assert!(matches!(state.mode, crate::app::Mode::Welcome));
        state.gutters.sidebar_visible = false;
        assert!(!state.layout_plan().shows(GutterSide::Left));
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(state.layout_plan().shows(GutterSide::Left));
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(!state.layout_plan().shows(GutterSide::Left));
    }

    #[test]
    fn the_left_panel_width_survives_switching_from_welcome_to_folder() {
        let mut state = state(1400.0, GutterSide::Left);
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        // Simulates a finished resize (the actual drag is driven by mouse-move window events,
        // not a `Message`): widening it on Welcome should still be in effect once a folder opens.
        state.gutters.sidebar_width = 360.0;
        let width = state.gutters.sidebar_width;
        state.mode = crate::app::Mode::Folder {
            root: std::path::PathBuf::from("/notes"),
            files: Vec::new(),
            query: String::new(),
            selected: None,
        };
        assert_eq!(state.gutters.sidebar_width, width);
    }
}
