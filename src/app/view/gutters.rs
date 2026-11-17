// SPDX-License-Identifier: GPL-3.0-or-later
use super::toolbar::{maybe_tooltip, tip};
use crate::app::{Message, State, TOOLBAR_CONTROL_HEIGHT, chrome_size};
use crate::gutter::GutterSide;
use crate::layout::budget;
use crate::layout::spacing;
use crate::ui::icons::svg_icon;
use crate::ui::press_area::PressArea;
use crate::ui::theme;
use crate::ui::theme::Appearance;
use iced::widget::{Space, Stack, button, column, container, mouse_area, row, text, tooltip};
use iced::{Alignment, Element, Fill, Padding};

impl State {
    /// Overlays `well` (a docked card and the margins around it, `width` wide inside) with its
    /// resize strip, placed from [`budget::edge_spans`] so the target and the cue line lie on the
    /// card's own inner edge rather than in the margin beside it.
    pub(crate) fn with_resize_strip<'a>(
        &self,
        well: impl Into<Element<'a, Message>>,
        side: GutterSide,
        width: f32,
    ) -> Element<'a, Message> {
        Stack::new()
            .push(well)
            .push(self.resize_strip(side, width))
            .into()
    }

    /// The full-height resize target laid over a gutter card's inner edge, on the side facing the
    /// preview. It costs no layout width and sits on the card's padding, clear of its content.
    /// Hovering or dragging it fades in one full-height line along the card's edge, over its
    /// border; global events continue the drag.
    fn resize_strip<'a>(&self, side: GutterSide, width: f32) -> Element<'a, Message> {
        let appearance = self.appearance;
        let opacity = self.gutters.cue_opacity(side);
        let spans = budget::edge_spans(side, width);
        // The line hugs the card's edge, which is the edge of the target.
        let edge = match side {
            GutterSide::Left => Alignment::End,
            GutterSide::Right => Alignment::Start,
        };
        let line = container(Space::new().width(budget::LINE_WIDTH).height(Fill))
            .style(move |_| theme::resize_handle(appearance, opacity));
        let visual = container(line).width(Fill).height(Fill).align_x(edge);
        let area = mouse_area(container(visual).width(spans.target.width()).height(Fill))
            .interaction(iced::mouse::Interaction::ResizingHorizontally)
            .on_enter(Message::ResizeZoneEntered(side))
            .on_exit(Message::ResizeZoneExited(side));
        container(PressArea::new(area, move |x| {
            Message::ResizeDragStart(side, x)
        }))
        .width(Fill)
        .height(Fill)
        .padding(Padding {
            left: spans.target.start,
            right: spans.footprint.end - spans.target.end,
            ..Padding::ZERO
        })
        .into()
    }

    /// A segmented capsule of tab chips, shared by the right gutter's Outline/Details and the
    /// left gutter's Files/Recent. `chips` is `(icon, label, show_label, active, message)` per
    /// tab, in order; the first and last get the capsule's rounded ends. `label` always names the
    /// tab for its tooltip; `show_label` additionally prints it beside the icon (the right
    /// gutter's tabs do, the left gutter's icon-only ones don't — there isn't room beside its
    /// other header controls for both tabs to carry text and stay close to them). The left
    /// gutter's icon-only tabs also use the toggle buttons' subtler active highlight (a tinted
    /// background, not a filled one) rather than the right gutter's bolder filled segment —
    /// `show_label` picks both at once, since they're the same two tabs either way.
    pub(crate) fn gutter_tabs<'a>(
        &self,
        chips: Vec<(&'static str, &'static str, bool, bool, Message)>,
    ) -> Element<'a, Message> {
        let appearance = self.appearance;
        let menu_open = self.menus.context.is_some();
        let last = chips.len().saturating_sub(1);
        let mut tabs = row![].spacing(spacing::GAP_TIGHT);
        for (index, (icon, label, show_label, active, message)) in chips.into_iter().enumerate() {
            let ends = match index {
                0 if last == 0 => theme::Ends::None,
                0 => theme::Ends::Left,
                i if i == last => theme::Ends::Right,
                _ => theme::Ends::None,
            };
            let colors = theme::colors(appearance);
            let color = if !active {
                colors.text
            } else if show_label {
                colors.on_accent
            } else {
                colors.accent
            };
            let content: Element<'_, Message> = if show_label {
                row![
                    svg_icon(icon, color, 14.0),
                    text(label).size(chrome_size(12.0))
                ]
                .spacing(spacing::GAP)
                .align_y(Alignment::Center)
                .into()
            } else {
                svg_icon(icon, color, 14.0)
            };
            let padding = if show_label {
                Padding::from([spacing::CONTROL_PAD_V, spacing::CONTROL_PAD_H])
            } else {
                Padding::from(spacing::CONTROL_PAD_V)
            };
            let end_radius = TOOLBAR_CONTROL_HEIGHT / 2.0;
            let chip = button(content)
                .padding(padding)
                .height(TOOLBAR_CONTROL_HEIGHT);
            let chip = if show_label {
                chip.style(theme::segmented_tab(appearance, active, ends, end_radius))
            } else {
                chip.style(theme::icon_button_ends(
                    appearance, active, ends, end_radius,
                ))
            }
            .on_press(message);
            let chip: Element<'_, Message> = if show_label {
                chip.into()
            } else {
                maybe_tooltip(
                    chip,
                    tip(label, None, appearance),
                    tooltip::Position::Bottom,
                    menu_open,
                )
            };
            tabs = tabs.push(chip);
        }
        // A segmented control: every chip in one capsule well, the active one filled.
        container(tabs)
            .padding(spacing::WELL_PAD)
            .style(move |_| theme::icon_well(appearance))
            .into()
    }
}

/// The gap kept around a docked gutter's card: [`budget::OUTER_MARGIN`] on
/// every side that faces the window itself (its own edge, the topbar above,
/// the footer below), so its border is never flush with any of them, and a
/// narrower [`budget::INNER_MARGIN`] on the edge that carries the resize
/// handle. The two horizontal margins sum to exactly
/// [`budget::gutter_overhead`] — the number every other width calculation
/// (`LayoutPlan::text_measure`, the toolbar's reserved padding, the content
/// row's reserved space) already assumes, so nothing drifts out of step
/// with the card's actual footprint. The toolbar grows its own top padding
/// by the same `OUTER_MARGIN` (see `State::toolbar`) so a header's content,
/// now sitting that much further from the window's top, still lines up
/// with the toolbar's own row instead of drifting out of alignment with it.
pub(crate) fn docked_margin(side: GutterSide) -> Padding {
    let (left, right) = match side {
        GutterSide::Left => (budget::OUTER_MARGIN, budget::INNER_MARGIN),
        GutterSide::Right => (budget::INNER_MARGIN, budget::OUTER_MARGIN),
    };
    Padding {
        top: budget::OUTER_MARGIN,
        right,
        bottom: budget::OUTER_MARGIN,
        left,
    }
}

/// The gap kept around an overlaid gutter's card (compact windows): it
/// floats fully inset, clear of every edge, since there is no resize handle.
pub(crate) fn overlay_margin() -> Padding {
    Padding::new(budget::OUTER_MARGIN)
}

/// Pixels shaved off a gutter header's left padding to cancel the toggle's shift on open.
const HEADER_EDGE_TRIM: f32 = 8.0;

/// Extra space left of the toggle, kept in both the header and the toolbar so it doesn't shift.
pub(crate) const TOGGLE_EDGE_INSET: f32 = 4.0;

/// A gutter header's padding. The edge facing the window is trimmed by [`HEADER_EDGE_TRIM`] and
/// widened by [`TOGGLE_EDGE_INSET`] so the gutter's toggle lands where the toolbar's does when the
/// gutter is closed (the card's margin and border otherwise push it inward when it opens; #36).
fn header_padding(side: GutterSide) -> Padding {
    let outer = spacing::CONTROL_PAD_H - HEADER_EDGE_TRIM + TOGGLE_EDGE_INSET;
    let (left, right) = match side {
        GutterSide::Left => (outer, spacing::CONTROL_PAD_H),
        GutterSide::Right => (spacing::CONTROL_PAD_H, outer),
    };
    // The right header's icon cluster sits in a well that extends `WELL_INSET` past its buttons;
    // trimming that much here keeps the buttons where they were.
    let (inset_right, inset_v) = match side {
        GutterSide::Left => (0.0, 0.0),
        GutterSide::Right => (spacing::WELL_INSET, spacing::WELL_INSET),
    };
    Padding {
        top: spacing::UNIT - inset_v,
        right: right - inset_right,
        bottom: spacing::UNIT - inset_v,
        left,
    }
}

/// A gutter's floating card (rounded, bordered, `surface`-filled, at the
/// current drag-adjusted width), with an optional header zone — the
/// gutter's own show/hide toggle and, for the right gutter, its Outline/
/// Details tabs — sitting above the content with no divider between them.
/// `margin` is the gap kept around the card
/// inside its `width`-plus-margin slot: callers flush the edges that should
/// bump into the window or the topbar and keep a small gap only on the edge
/// that carries the resize handle (see [`State::with_resize_strip`]). `content_pad` insets the
/// content from the card's edge: [`spacing::CARD_PAD`] for text panels, [`spacing::list::CARD_PAD`]
/// for lists whose rows carry their own padding.
pub(crate) fn gutter_card<'a>(
    content: impl Into<Element<'a, Message>>,
    header: Option<Element<'a, Message>>,
    appearance: Appearance,
    width: f32,
    margin: Padding,
    side: GutterSide,
    content_pad: f32,
) -> Element<'a, Message> {
    let inner: Element<'a, Message> = match header {
        Some(header) => column![
            // Same padding as the toolbar's own content (`State::toolbar`), not the card's usual
            // `CARD_PAD`: with the card flush against the toolbar (`docked_margin`'s zero top), this
            // is what lands the header's buttons at the exact same height as the toolbar's, so the
            // two read as one continuous row instead of visibly jumping when a gutter toggles.
            container(header).width(Fill).padding(header_padding(side)),
            container(content)
                .width(Fill)
                .height(Fill)
                .padding(content_pad),
        ]
        .into(),
        None => container(content)
            .width(Fill)
            .height(Fill)
            .padding(content_pad)
            .into(),
    };
    // Both cards carry the accent gradient, which is a ring: an outer container that paints it,
    // padded by the ring's thickness, around the flat-filled card.
    container(
        container(
            container(inner)
                .width(Fill)
                .height(Fill)
                .style(move |_| theme::accent_card_inner(appearance)),
        )
        .width(width)
        .height(Fill)
        .padding(theme::accent_card_ring(appearance))
        .style(move |_| theme::accent_card(appearance)),
    )
    .padding(margin)
    .height(Fill)
    .style(move |_| theme::sidebar_well(appearance))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_margins_are_the_ones_the_resize_strip_is_placed_from() {
        for side in [GutterSide::Left, GutterSide::Right] {
            let spans = budget::edge_spans(side, 300.0);
            let margin = docked_margin(side);
            assert_eq!(margin.left, spans.card.start - spans.footprint.start);
            assert_eq!(margin.right, spans.footprint.end - spans.card.end);
        }
    }
}
