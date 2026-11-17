// SPDX-License-Identifier: GPL-3.0-or-later
use crate::actions::MenuAction;
use crate::app::view::search_bar;
use crate::app::{
    DocumentView, ICON_SIZE, Message, Mode, State, TOOLBAR_CONTROL_HEIGHT, chrome_size,
};
use crate::domain::document::Document;
use crate::domain::keymap::{self, Command};
use crate::gutter::{GutterSide, LeftTab, RightTab};
use crate::layout::Placement;
use crate::layout::ToolbarDensity;
use crate::layout::budget;
use crate::layout::spacing;
use crate::ui::anchor_area::AnchorArea;
use crate::ui::ellipsis_text::{ElidedTooltip, EllipsisText, truncate_middle};
use crate::ui::focus_watch::FocusWatch;
use crate::ui::icons::svg_icon;
use crate::ui::slide::Collapse;
use crate::ui::theme::Ends;
use crate::ui::title_search::TitleSearch;
use crate::ui::{icons, theme};
use iced::widget::{Space, button, column, container, row, tooltip};
use iced::{Alignment, Color, Element, Fill, Padding};

/// Tooltip content: compact label on a raised, bordered chip. When `command`
/// has a keymap binding, its chord is appended, e.g. "Hide file list  ⌘B".
/// Builds an owned string, so the result borrows nothing and fits any `'a`.
pub(super) fn tip<'a>(
    label: impl Into<String>,
    command: Option<Command>,
    appearance: crate::ui::theme::Appearance,
) -> Element<'a, Message> {
    let mac = cfg!(target_os = "macos");
    let label = label.into();
    let text = match command.and_then(|command| keymap::label_for(command, mac)) {
        Some(chord) => format!("{label}  {chord}"),
        None => label,
    };
    container(iced::widget::text(text).size(chrome_size(11.0)))
        .padding([spacing::HALF, spacing::UNIT])
        .style(move |_| theme::tooltip_panel(appearance))
        .into()
}

/// Wraps `content` in a tooltip, unless `menu_open` — then the tooltip is simply not built,
/// since a hovering tooltip tends to cover a dropdown or right-click menu that just opened right
/// beside or under it. App-wide: every control with a tooltip goes through this instead of
/// calling `tooltip` directly, with `self.menus.context.is_some()` as `menu_open`.
pub(super) fn maybe_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    tip_content: Element<'a, Message>,
    position: tooltip::Position,
    menu_open: bool,
) -> Element<'a, Message> {
    let content = content.into();
    if menu_open {
        content
    } else {
        tooltip(content, tip_content, position)
            .gap(spacing::TOOLTIP_GAP)
            .into()
    }
}

/// Wraps the right icon cluster in a recessed well. The well grows outward by
/// [`spacing::WELL_INSET`], so callers trim that much from the padding around it.
pub(super) fn icon_well<'a>(
    content: impl Into<Element<'a, Message>>,
    appearance: theme::Appearance,
) -> Element<'a, Message> {
    container(content)
        .padding(spacing::WELL_PAD)
        .style(move |_| theme::icon_well(appearance))
        .into()
}

/// The toolbar icon color, dimmed when `enabled` is false. The SVG tint ignores alpha, so this
/// dims by blending the text color toward the toolbar surface instead.
fn dimmed_text_color(appearance: theme::Appearance, enabled: bool) -> Color {
    let c = theme::colors(appearance);
    if enabled {
        c.text
    } else {
        let t = 0.25;
        Color::from_rgb(
            c.surface.r + (c.text.r - c.surface.r) * t,
            c.surface.g + (c.text.g - c.surface.g) * t,
            c.surface.b + (c.text.b - c.surface.b) * t,
        )
    }
}

/// `button` with a tiny triangle in its bottom-right corner, the mark for a control whose
/// right-click opens a dropdown (Theme, Font, the combined Open button, New). The triangle takes
/// no events, so the button still works.
pub(super) fn with_dropdown_mark<'a>(
    button: impl Into<Element<'a, Message>>,
    appearance: theme::Appearance,
) -> Element<'a, Message> {
    let mark = container(svg_icon(
        icons::CORNER_MARK,
        theme::colors(appearance).text.scale_alpha(0.55),
        DROPDOWN_MARK_SIZE,
    ))
    .width(Fill)
    .height(Fill)
    .align_x(Alignment::End)
    .align_y(Alignment::End)
    .padding(2.0);
    iced::widget::Stack::new().push(button).push(mark).into()
}

/// The side of the dropdown mark's square, in pixels.
const DROPDOWN_MARK_SIZE: f32 = 7.0;

/// The space between the toolbar's content and the visible border of an open gutter's card, on
/// either side: one unit.
const CONTENT_GAP: f32 = spacing::UNIT;

/// A full-window layer holding `content` at `align` with `padding`, for pinning a ghost.
fn ghost_layer(
    content: Element<'_, Message>,
    padding: Padding,
    align: Alignment,
) -> Element<'_, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .padding(padding)
        .align_x(align)
        .into()
}

/// The toolbar's right-hand controls, built together because they share one count of buttons.
struct RightControls<'a> {
    open_with: Element<'a, Message>,
    right_cluster: Element<'a, Message>,
    cluster_shown: bool,
    open_with_shown: bool,
    standalone_open_with: bool,
}

impl State {
    /// The left nav's own toggle, shown inside its header when the nav is open
    /// and inside the topbar when it is closed (so it's always reachable).
    pub(crate) fn sidebar_toggle(&self, active: bool) -> Element<'_, Message> {
        let appearance = self.appearance;
        let button = button(svg_icon(
            icons::SIDEBAR,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| theme::icon_button(appearance, active)(t, s))
        .on_press(Message::TogglePanel(GutterSide::Left));
        let (show_label, hide_label) = match self.mode {
            Mode::Folder { .. } => ("Show file list", "Hide file list"),
            Mode::Welcome => ("Show recent", "Hide recent"),
        };
        maybe_tooltip(
            button,
            tip(
                if active { hide_label } else { show_label },
                Some(Command::ToggleFiles),
                appearance,
            ),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The right gutter's own toggle, shown inside its header when open and
    /// inside the topbar when closed.
    pub(crate) fn right_gutter_toggle(&self, active: bool, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let button = button(svg_icon(
            icons::SIDEBAR_RIGHT,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, active, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        .on_press(Message::TogglePanel(GutterSide::Right));
        maybe_tooltip(
            button,
            tip(
                if active {
                    "Hide right gutter"
                } else {
                    "Show right gutter"
                },
                Some(Command::ToggleRightGutter),
                appearance,
            ),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// Back/Forward document history buttons, part of the left nav's own
    /// control cluster (see [`State::sidebar_toggle`]).
    pub(crate) fn history_buttons(&self) -> Element<'_, Message> {
        row![
            self.history_button(
                icons::CHEVRON_LEFT,
                "Back",
                Command::HistoryBack,
                Message::HistoryBack,
                self.can_go_back()
            ),
            self.history_button(
                icons::CHEVRON_RIGHT,
                "Forward",
                Command::HistoryForward,
                Message::HistoryForward,
                self.can_go_forward()
            ),
        ]
        .spacing(spacing::GAP_TIGHT)
        .align_y(Alignment::Center)
        .into()
    }

    fn history_button(
        &self,
        icon: &'static str,
        label: &'static str,
        command: Command,
        message: Message,
        enabled: bool,
    ) -> Element<'_, Message> {
        let appearance = self.appearance;
        let button = button(svg_icon(
            icon,
            dimmed_text_color(appearance, enabled),
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| theme::icon_button(appearance, false)(t, s))
        .on_press_maybe(enabled.then_some(message));
        maybe_tooltip(
            button,
            tip(label, Some(command), appearance),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The combined Open control, following Back/Forward: a left-click opens a file (the common
    /// case), a right-click opens a dropdown offering both File and Folder — the same pattern as
    /// the Theme and Font buttons. Recent has no toolbar button of its own in either mode: open
    /// the left panel (on Welcome it always shows Recent; in a folder, switch its Files|Recent
    /// tab to Recent).
    pub(crate) fn open_buttons(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        let open_button = button(svg_icon(
            icons::FILE,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| theme::icon_button(appearance, false)(t, s))
        .on_press(Message::OpenFileClicked);
        let menu_open = self.menus.context.is_some();
        let open: Element<'static, Message> = maybe_tooltip(
            AnchorArea::on_right_press(with_dropdown_mark(open_button, appearance), |bounds| {
                Message::OpenFileOrFolderDropdown(bounds)
            }),
            tip("Open file…", Some(Command::OpenFile), appearance),
            tooltip::Position::Bottom,
            menu_open,
        );
        row![open]
            .spacing(spacing::GAP_TIGHT)
            .align_y(Alignment::Center)
            .into()
    }

    /// The Open With button; opens a dropdown of editors like the Recent button.
    pub(crate) fn open_with_button(&self, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let button = button(svg_icon(
            icons::EDIT,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, false, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        // The `AnchorArea` opens the dropdown; this press only gives hover and click feedback.
        .on_press(Message::ContextMenuInert);
        maybe_tooltip(
            AnchorArea::new(button, Message::OpenEditorDropdown),
            tip("Open with…", None, appearance),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// Cycles the active theme to the next choice in menu order (System, Slate, Harbor,
    /// Nightfall, Linen, Paper), saving it like the Appearance menu does.
    pub(crate) fn theme_button(&self, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let next = self.config.theme.appearance.next();
        let button = button(svg_icon(
            icons::THEME,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, false, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        .on_press(Message::MenuAction(MenuAction::Appearance(next)));
        let active = truncate_middle(self.config.theme.appearance.label(), 24);
        maybe_tooltip(
            AnchorArea::on_right_press(with_dropdown_mark(button, appearance), |bounds| {
                Message::OpenThemeDropdown(bounds)
            }),
            tip(format!("Theme: {active}"), None, appearance),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The preview font button: a click cycles to the next font, a right-click opens the list
    /// of fonts like the Recent button's dropdown.
    pub(crate) fn font_button(&self, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let next = self.config.preview.font.next();
        let button = button(svg_icon(
            icons::FONT,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, false, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        .on_press(Message::MenuAction(MenuAction::PreviewFont(next)));
        let active = truncate_middle(self.config.preview.font.family(), 24);
        maybe_tooltip(
            AnchorArea::on_right_press(with_dropdown_mark(button, appearance), |bounds| {
                Message::OpenFontDropdown(bounds)
            }),
            tip(format!("Font: {active}"), None, appearance),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The minimap toggle; lit while the minimap is on.
    pub(crate) fn minimap_button(&self, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let on = self.config.preview.minimap;
        let button = button(svg_icon(
            icons::MINIMAP,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, on, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        .on_press(Message::MenuAction(MenuAction::ToggleMinimap));
        maybe_tooltip(
            button,
            tip(
                if on { "Hide minimap" } else { "Show minimap" },
                Some(Command::ToggleMinimap),
                appearance,
            ),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The view-source toggle; lit while the raw Markdown source is shown instead of the
    /// rendered preview (issue #71).
    pub(crate) fn source_button(&self, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let on = self.document_view == DocumentView::Source;
        let button = button(svg_icon(
            icons::CODE,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, on, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        .on_press_maybe(
            (!self.is_text_document()).then_some(Message::MenuAction(MenuAction::ToggleSourceView)),
        );
        maybe_tooltip(
            button,
            tip(
                if on { "Show preview" } else { "View source" },
                Some(Command::ToggleSourceView),
                appearance,
            ),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The left nav's header zone, shown above its content while it is open: its own toggle,
    /// document history, Open, and the icon-only Files|Recent capsule, all packed together —
    /// unlike the right header, nothing pushes the tabs away to the opposite end, since that
    /// read as the tabs being disconnected from the rest of the header's controls.
    pub(crate) fn left_nav_header(&self) -> Element<'_, Message> {
        let tabs = self.gutter_tabs(vec![
            (
                icons::FOLDER,
                "Files",
                false,
                self.gutters.left_tab == LeftTab::Files,
                Message::SelectLeftTab(LeftTab::Files),
            ),
            (
                icons::RECENT,
                "Recent",
                false,
                self.gutters.left_tab == LeftTab::Recent,
                Message::SelectLeftTab(LeftTab::Recent),
            ),
        ]);
        // Top-aligned, like the toolbar's own primary row (see `State::toolbar`): with matching
        // padding above both, that's what keeps the toggle sitting at the exact same pixel in
        // either place, regardless of a taller sibling (the toolbar's search field, or now the
        // tabs capsule) stretching one row's natural height past the other's.
        row![
            self.sidebar_toggle(true),
            self.history_buttons(),
            self.open_buttons(),
            self.new_item_button(),
            tabs,
        ]
        .spacing(spacing::GAP)
        .align_y(Alignment::Start)
        .into()
    }

    /// Welcome's left panel header: its own toggle, Open, and a plain "Recent" label in place of
    /// the Files|Recent capsule — there's no Files tab before a folder is open, and no history to
    /// carry Back/Forward buttons for.
    pub(crate) fn welcome_panel_header(&self) -> Element<'_, Message> {
        row![
            self.sidebar_toggle(true),
            self.open_buttons(),
            iced::widget::text("Recent")
                .size(chrome_size(12.0))
                .color(theme::colors(self.appearance).text_muted),
        ]
        .spacing(spacing::GAP)
        .align_y(Alignment::Start)
        .into()
    }

    /// The nav header's New button: a left-click creates a new Markdown file (the common case,
    /// same pattern as the combined Open button), a right-click opens the same File/Folder menu
    /// `nav_footer`'s button used to, now positioned under this button instead of flipped upward
    /// to clear the nav's bottom edge. Open-panel only: there's no closed-toolbar copy the way
    /// the toggle, history, and Open get one, since creating something needs the nav to be
    /// showing a destination for it.
    fn new_item_button(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        let Some(root) = self.root().map(std::path::Path::to_path_buf) else {
            return Space::new().into();
        };
        let button = button(svg_icon(
            icons::ADD,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| theme::icon_button(appearance, false)(t, s))
        .on_press(Message::NewMarkdownFile(root));
        maybe_tooltip(
            AnchorArea::on_right_press(with_dropdown_mark(button, appearance), |bounds| {
                Message::OpenNewDropdown(bounds)
            }),
            tip("New .md or folder", None, appearance),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// The right gutter's header zone, shown above its content while it is
    /// open: the Outline/Details tabs, the copy-source-path button (it
    /// belongs with the document details this gutter shows), and the
    /// gutter's own toggle.
    pub(crate) fn right_gutter_header(&self) -> Element<'_, Message> {
        // Centered, not top-aligned: the well makes the icon cluster `2 * WELL_INSET` taller than
        // the tabs, and centering puts the tabs exactly where they were (see `header_padding`).
        // Document actions live inside the card; the app-wide theme and font buttons stay in the
        // toolbar (see `State::toolbar`).
        let has_document = self.current_document().is_some();
        let ends = |first: bool| if first { Ends::Left } else { Ends::None };
        let mut actions = row![].spacing(spacing::GAP_TIGHT).align_y(Alignment::Start);
        if has_document {
            actions = actions.push(self.open_with_button(Ends::Left));
        }
        actions = actions
            .push(self.copy_path_button(ends(!has_document)))
            .push(self.right_gutter_toggle(true, Ends::Right));
        let tabs = self.gutter_tabs(vec![
            (
                icons::OUTLINE,
                "Outline",
                true,
                self.gutters.right_tab == RightTab::Outline,
                Message::SelectRightTab(RightTab::Outline),
            ),
            (
                icons::INFO,
                "Details",
                true,
                self.gutters.right_tab == RightTab::Details,
                Message::SelectRightTab(RightTab::Details),
            ),
        ]);
        row![
            tabs,
            Space::new().width(Fill),
            icon_well(actions, self.appearance),
        ]
        .spacing(spacing::GAP_TIGHT)
        .align_y(Alignment::Center)
        .into()
    }

    /// Copies the current document's source path; travels with the right
    /// gutter's toggle (see [`State::right_gutter_header`]) since both sit
    /// in the toolbar region a docked or overlaid right gutter floats over.
    pub(crate) fn copy_path_button(&self, ends: Ends) -> Element<'_, Message> {
        let appearance = self.appearance;
        let has_document = self.current_document().is_some();
        let button = button(svg_icon(
            icons::SHARE,
            theme::colors(appearance).text,
            ICON_SIZE,
        ))
        .padding(spacing::CONTROL_PAD_V)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .style(move |t, s| {
            theme::icon_button_ends(appearance, false, ends, TOOLBAR_CONTROL_HEIGHT / 2.0)(t, s)
        })
        .on_press_maybe(has_document.then_some(Message::CopySource));
        maybe_tooltip(
            button,
            tip("Copy source path", None, appearance),
            tooltip::Position::Bottom,
            self.menus.context.is_some(),
        )
    }

    /// `content` dimmed toward the toolbar's own fill by `1 - visibility`, so it fades without
    /// any widget opacity (iced has none). The cover takes no events: buttons beneath still work.
    fn faded<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        visibility: f32,
    ) -> Element<'a, Message> {
        let toolbar = theme::colors(self.appearance).surface;
        let cover =
            container(Space::new().width(Fill).height(Fill)).style(move |_| container::Style {
                background: Some(iced::Background::Color(Color {
                    a: 1.0 - visibility,
                    ..toolbar
                })),
                ..container::Style::default()
            });
        iced::widget::Stack::new().push(content).push(cover).into()
    }

    /// The toolbar's left-edge cluster when the left card is not docked to hold it: the sidebar
    /// toggle, plus document history on a folder (Welcome has none), plus Open. Shared by
    /// [`State::toolbar_leading`], [`State::toolbar_ghost`], and the Full-density toolbar's
    /// mid-slide branch so the three near-identical rows stay in one place.
    fn left_cluster(&self) -> Element<'_, Message> {
        let mut cluster = row![self.sidebar_toggle(false)];
        if matches!(self.mode, Mode::Folder { .. }) {
            cluster = cluster.push(self.history_buttons());
        }
        cluster = cluster.push(self.open_buttons());
        cluster
            .spacing(spacing::GAP)
            .align_y(Alignment::Start)
            .into()
    }

    /// The layer drawn over the toolbar while the left card slides: the cluster the toolbar
    /// shows when the left panel is closed (toggle, history, Open), fading out as the card
    /// arrives over it and back in as it leaves, so it is never swept sideways. Always one layer
    /// (an empty one when idle) so the stack above keeps its shape and the cards' widget state is
    /// not rebuilt. It sits where the closed toolbar puts the cluster (see [`State::toolbar`]).
    /// The right cluster needs no ghost: it stays in the toolbar row and collapses.
    pub(crate) fn toolbar_ghost(&self) -> Element<'_, Message> {
        let Some(visibility) = self.gutters.toolbar_fade(GutterSide::Left) else {
            return Space::new().into();
        };
        let cluster = self.left_cluster();
        ghost_layer(
            self.faded(cluster, visibility),
            Padding {
                top: spacing::UNIT + budget::OUTER_MARGIN,
                left: spacing::CONTROL_PAD_H + super::gutters::TOGGLE_EDGE_INSET,
                ..Padding::ZERO
            },
            Alignment::Start,
        )
    }

    /// The open document's title for the toolbar — its frontmatter `title` if it has one,
    /// otherwise its file name (`Document::display_title`) — one line, ending in an ellipsis
    /// when it does not fit (the full name shows on hover), or nothing when no document is open.
    fn toolbar_title(&self) -> Element<'_, Message> {
        let Some(name) = self.current_document().map(Document::display_title) else {
            return Space::new().into();
        };
        let text = EllipsisText::new(name.clone())
            .size(chrome_size(13.0))
            .color(theme::colors(self.appearance).text)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .shrink();
        ElidedTooltip::new(text, tip(name, None, self.appearance)).into()
    }

    /// The controls at the toolbar's left edge when the left card is not docked to hold them: the
    /// sidebar toggle, plus history and Open on a folder, or just toggle and Open on Welcome.
    fn toolbar_leading(&self, panel_active: bool, left_sliding: bool) -> Element<'_, Message> {
        if !panel_active && !left_sliding {
            self.left_cluster()
        } else {
            Space::new().into()
        }
    }

    /// The folder search field, or nothing on the welcome screen.
    fn toolbar_search(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        match &self.mode {
            Mode::Folder { query, .. } => {
                // Watched so the field can expand while it has focus (Full density; see below).
                // With text in it, the first Escape clears it and keeps focus; the second, with
                // nothing left to clear, passes through and blurs the field.
                let watch = FocusWatch::new(
                    search_bar(query, appearance),
                    self.search_focused,
                    Message::SearchFocused,
                );
                if query.is_empty() {
                    watch.into()
                } else {
                    watch.on_escape(|| Message::SearchCleared).into()
                }
            }
            Mode::Welcome => Space::new().into(),
        }
    }

    /// The right-hand controls: the cluster of theme, font, minimap and document actions, and
    /// the Open with button that sits apart from it on narrower toolbars.
    fn toolbar_right_controls(
        &self,
        density: ToolbarDensity,
        has_document: bool,
        right_showing: bool,
        right_fade: Option<f32>,
    ) -> RightControls<'_> {
        let plan_toolbar = density;
        let in_folder = matches!(self.mode, Mode::Folder { .. });
        let docs_shown = !right_showing || right_fade.is_some();
        // Stacked puts Open with in its own well on the second row, apart from the cluster.
        let joined = plan_toolbar != ToolbarDensity::Stacked;
        let open_with_shown = has_document && docs_shown;
        let in_cluster_open_with = open_with_shown && joined && in_folder;
        let standalone_open_with = open_with_shown && !in_cluster_open_with;
        let globals = if in_folder { 4 } else { 0 };
        let doc_count =
            usize::from(in_cluster_open_with) + if in_folder && docs_shown { 2 } else { 0 };
        let total = globals + doc_count;
        // Ends by position in the one capsule: first rounds left, last rounds right.
        let ends_at = |i: usize| match (i == 0, i + 1 == total) {
            (true, true) => Ends::Both,
            (true, false) => Ends::Left,
            (false, true) => Ends::Right,
            (false, false) => Ends::None,
        };
        let open_with: Element<'_, Message> = if open_with_shown {
            self.open_with_button(Ends::Both)
        } else {
            Space::new().into()
        };

        // Copy-path travels with the right gutter's own toggle (both sit in the toolbar region a
        // docked or overlaid right gutter floats over).
        let cluster_shown = in_folder && total > 0;
        let right_cluster: Element<'_, Message> = if cluster_shown {
            let mut globals_row = row![]
                .spacing(spacing::GAP_TIGHT)
                .align_y(Alignment::Center);
            globals_row = globals_row
                .push(self.theme_button(ends_at(0)))
                .push(self.font_button(ends_at(1)))
                .push(self.minimap_button(ends_at(2)))
                .push(self.source_button(ends_at(3)));
            let mut docs = row![]
                .spacing(spacing::GAP_TIGHT)
                .align_y(Alignment::Center);
            let mut i = globals;
            if in_cluster_open_with {
                // Rebuilt here (not reusing `open_with`) so it sits in the faded group.
                docs = docs.push(self.open_with_button(ends_at(i)));
                i += 1;
            }
            if docs_shown {
                docs = docs
                    .push(self.copy_path_button(ends_at(i)))
                    .push(self.right_gutter_toggle(false, ends_at(i + 1)));
                let docs: Element<'_, Message> =
                    row![Space::new().width(spacing::GAP_TIGHT), docs].into();
                let docs = match right_fade {
                    Some(visibility) => {
                        Collapse::new(self.faded(docs, visibility), visibility).into()
                    }
                    None => docs,
                };
                row![globals_row, docs].into()
            } else {
                globals_row.into()
            }
        } else {
            Space::new().into()
        };
        RightControls {
            open_with,
            right_cluster,
            cluster_shown,
            open_with_shown,
            standalone_open_with,
        }
    }

    /// The unified toolbar: gutter toggles and a cluster
    /// of actions (search, Open With, share) mirroring a
    /// native macOS window's title-bar toolbar.
    pub(crate) fn toolbar(&self) -> Element<'_, Message> {
        let plan = self.layout_plan();
        let appearance = self.appearance;
        let has_document = self.current_document().is_some();
        // Only a docked card carries the nav header; the compact strip has none, so the toolbar
        // keeps the toggle, history, and Open buttons itself and reserves no card width.
        let files_active = plan.left == Placement::Docked;
        let right_showing = plan.shows(GutterSide::Right);
        // While the left card slides, the toolbar's own copy of its buttons is drawn as a fading
        // ghost (`State::toolbar_ghost`), not as part of this row.
        let left_sliding = self.gutters.toolbar_fade(GutterSide::Left).is_some();
        // The right cluster stays in the row while its card slides, collapsing and fading, so the
        // controls beside it glide instead of jumping by its width.
        let right_fade = self.gutters.toolbar_fade(GutterSide::Right);

        // Back/Forward are document history, not nav-specific, so they stay reachable in the
        // toolbar even while the nav (and its header, where they also live — see
        // `State::left_nav_header`) is closed; only the toggle itself travels either way.
        let sidebar_toggle = self.toolbar_leading(files_active, left_sliding);

        let search_widget = self.toolbar_search();

        // Theme and font are app-wide, so they stay in the toolbar even while the right card is
        // open. The document actions (Open with, copy path, the card's toggle) collapse into the
        // card's header (`State::right_gutter_header`) once it opens.
        let RightControls {
            open_with,
            right_cluster,
            cluster_shown,
            open_with_shown,
            standalone_open_with,
        } = self.toolbar_right_controls(plan.toolbar, has_document, right_showing, right_fade);

        // Each right-hand cluster sits in a well that extends `WELL_INSET` past its buttons. The
        // trims below put the buttons back where they were without the well; `row_inset` is how far
        // a row grows on each of its sides.
        fn well(
            present: bool,
            content: Element<'_, Message>,
            appearance: theme::Appearance,
        ) -> Element<'_, Message> {
            if present {
                icon_well(content, appearance)
            } else {
                content
            }
        }
        let inset = spacing::WELL_INSET;
        let grow = |present: bool| if present { inset } else { 0.0 };
        // A row without a well gets this much right padding back, so its right edge still lines
        // up with the rows that have one (the toolbar's own right padding is trimmed for them).
        let flush = |row_inset: f32, any: f32| Padding {
            right: any - row_inset,
            ..Padding::ZERO
        };

        let (content, top_trim, bottom_trim, right_trim): (Element<'_, Message>, f32, f32, f32) =
            match plan.toolbar {
                ToolbarDensity::Stacked => {
                    // Top-aligned, not centered: see the comment on `State::left_nav_header` for why
                    // — a row mixing the toggle with a taller sibling must not center it.
                    let (r1, r2) = (cluster_shown, open_with_shown);
                    let (g1, g2) = (grow(r1), grow(r2));
                    let any = g1.max(g2);
                    let first_row = row![
                        sidebar_toggle,
                        Space::new().width(Fill),
                        well(r1, right_cluster, appearance)
                    ]
                    .align_y(Alignment::Center)
                    .spacing(spacing::GAP);
                    let second_row = row![search_widget, well(r2, open_with, appearance)]
                        .align_y(Alignment::Center)
                        .spacing(spacing::GAP - g2);
                    (
                        column![
                            container(first_row).padding(flush(g1, any)),
                            container(second_row).padding(flush(g2, any))
                        ]
                        .spacing(spacing::GAP - g1 - g2)
                        .into(),
                        g1,
                        g2,
                        any,
                    )
                }
                ToolbarDensity::TwoRow => {
                    let present = standalone_open_with || cluster_shown;
                    let g = grow(present);
                    let actions = row![]
                        .extend(standalone_open_with.then_some(open_with))
                        .extend(cluster_shown.then_some(right_cluster))
                        .spacing(0)
                        .align_y(Alignment::Start);
                    let first_row = row![
                        sidebar_toggle,
                        Space::new().width(Fill),
                        well(present, actions.into(), appearance)
                    ]
                    .align_y(Alignment::Center)
                    .spacing(spacing::GAP);
                    (
                        column![first_row, container(search_widget).padding(flush(0.0, g))]
                            .spacing(spacing::GAP - g)
                            .into(),
                        g,
                        0.0,
                        g,
                    )
                }
                ToolbarDensity::Full => {
                    let present = standalone_open_with || cluster_shown;
                    let g = grow(present);
                    let actions = row![]
                        .extend(standalone_open_with.then_some(open_with))
                        .extend(cluster_shown.then_some(right_cluster))
                        .spacing(0)
                        .align_y(Alignment::Start);
                    // The title and the search share one slot: the title at its start, the search at
                    // its end, and focusing the search grows it over the room the title gives up.
                    let slot: Element<'_, Message> = if matches!(self.mode, Mode::Folder { .. }) {
                        TitleSearch::new(
                            self.toolbar_title(),
                            search_widget,
                            self.search_expand.value(),
                            spacing::SEARCH_WIDTH,
                            spacing::TITLE_MAX_WIDTH,
                            spacing::GAP,
                        )
                        .into()
                    } else {
                        Space::new().width(Fill).into()
                    };
                    // The gaps between the three are explicit, not row spacing, so that nothing is
                    // left between the open left card and the title once the cluster is gone.
                    let gap = spacing::GAP - g;
                    let left_slot: Element<'_, Message> =
                        match self.gutters.toolbar_fade(GutterSide::Left) {
                            // Mid-slide the cluster stays in the row, hidden, collapsing with its
                            // gap (the visible copy is `toolbar_ghost`), so the title glides
                            // instead of jumping by the cluster's width.
                            Some(visibility) => {
                                let cluster = row![self.left_cluster(), Space::new().width(gap),];
                                Collapse::new(self.faded(cluster, 0.0), visibility).into()
                            }
                            // Open at rest: the card holds these buttons, and takes no gap.
                            _ if files_active => Space::new().into(),
                            // Closed: unlike the docked card's bordered edge, there's no well
                            // behind this cluster, so it needs a wider gap to read as a separate
                            // group from the title rather than crowding it.
                            _ => row![sidebar_toggle, Space::new().width(spacing::GAP_SECTION - g)]
                                .into(),
                        };
                    (
                        row![
                            left_slot,
                            slot,
                            Space::new().width(gap),
                            well(present, actions.into(), appearance),
                        ]
                        .align_y(Alignment::Center)
                        .spacing(0)
                        .into(),
                        g,
                        g,
                        g,
                    )
                }
            };

        // Reserve, on each side that has an open gutter, the width its floating card takes (the
        // same `width + gutter_overhead()` the card itself and the content row's reserved space
        // use — see `gutters::docked_margin`/`overlay_margin`) so the toolbar's own content never
        // renders under a card it can't be seen or clicked through. While the card slides, the
        // reservation follows its position (`slide_progress`, `1.0` at rest) so the search field
        // glides with the card's edge instead of jumping; a card the plan has already hidden is
        // still reserved for until it is gone (`card_slot`).
        let edge = spacing::CONTROL_PAD_H + super::gutters::TOGGLE_EDGE_INSET;
        // With no left card, the content keeps `edge` from the window. Beside a left card, its
        // visible border is `INNER_MARGIN` inside the card's footprint, and the content starts
        // `CONTENT_GAP` past it, so the padding is the footprint less that margin plus the gap.
        // In between, it moves with the card.
        let left_padding = self
            .card_slot(&plan, GutterSide::Left)
            .filter(|slot| slot.placement == Placement::Docked)
            .map_or(edge, |slot| {
                budget::toolbar_padding(
                    edge,
                    slot.width,
                    CONTENT_GAP,
                    self.gutters.slide_progress(GutterSide::Left),
                )
            });
        // With no right card, the content keeps `edge` from the window. Beside a right card, its
        // visible edge is `INNER_MARGIN` inside the card's footprint, and the content stops
        // `CONTENT_GAP` short of it, so the padding is the footprint less that margin plus
        // the gap. In between, it moves with the card.
        let right_padding = {
            let closed = edge - right_trim;
            self.card_slot(&plan, GutterSide::Right)
                .filter(|slot| matches!(slot.placement, Placement::Docked | Placement::Overlay))
                .map_or(closed, |slot| {
                    budget::toolbar_padding(
                        closed,
                        slot.width,
                        CONTENT_GAP,
                        self.gutters.slide_progress(GutterSide::Right),
                    )
                })
        };
        // The extra top padding matches a docked gutter card's own `OUTER_MARGIN` (see
        // `gutters::docked_margin`): the card now sits that same gap below the window's top edge,
        // so growing the toolbar by it here is what keeps its content lined up with a header's,
        // rather than the header drifting lower once the card stopped being flush with the top.
        container(content)
            .padding(Padding {
                top: spacing::UNIT + budget::OUTER_MARGIN - top_trim,
                right: right_padding,
                bottom: spacing::UNIT - bottom_trim,
                // Same inset as a gutter header's left padding, so the toggle sits at one pixel either way.
                left: left_padding,
            })
            .width(Fill)
            .style(move |_| theme::toolbar_panel(appearance))
            .into()
    }
}
