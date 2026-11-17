// SPDX-License-Identifier: GPL-3.0-or-later
use crate::app::view::toolbar::tip;
use crate::app::view::{
    PREVIEW_INSET, TextSetting, docked_margin, gutter_card, inspector_panel, outline_sidebar,
    overlay_margin, render_markdown,
};
use crate::app::{
    ContextTarget, DocumentView, FIND_INPUT, Message, SEARCH_INPUT, State, TOOLBAR_CONTROL_HEIGHT,
    chrome_size,
};
use crate::domain::document::Document;
use crate::domain::document::DocumentKind;
use crate::domain::folder::{FolderNode, is_highlighted, matching_files, parent_key};
use crate::domain::minimap::Marks;
use crate::gutter::{GutterSide, LeftTab, RightTab};
use crate::layout::spacing;
use crate::layout::{LayoutPlan, Placement, budget};
use crate::ui::context_area::ContextArea;
use crate::ui::ellipsis_text::{ElidedTooltip, EllipsisText};
use crate::ui::icons::svg_icon;
use crate::ui::minimap::{self, Minimap};
use crate::ui::nav_scroll::{NAV_CURSOR, NAV_SCROLL};
use crate::ui::selectable_text::Drag;
use crate::ui::slide::{Edge, SlideLayer};
use crate::ui::theme::Appearance;
use crate::ui::{icons, theme};
use iced::widget::{
    Column, Space, button, center, column, container, row, scrollable, stack, text, text_input,
    tooltip,
};
use iced::{Alignment, Element, Fill, Length, Padding};
use std::path::{Path, PathBuf};

/// A mode's body plus, when a gutter is open and would otherwise sit in a
/// row below the toolbar, the floating card for it. Returned separately so
/// [`State::view`] can stack those cards over the toolbar and menu row too,
/// instead of just over the content below them. Built by both
/// [`State::folder_panes`] and [`State::welcome_panes`].
pub(crate) struct PanelBody<'a> {
    pub content: Element<'a, Message>,
    pub left_overlay: Option<Element<'a, Message>>,
    pub right_overlay: Option<Element<'a, Message>>,
}

/// How a gutter card is drawn this frame: where, and at what width.
#[derive(Clone, Copy)]
pub(crate) struct CardSlot {
    pub placement: Placement,
    pub width: f32,
}

impl State {
    /// The card to draw for `side`: the plan's, or the one still sliding out after the plan
    /// hid it. `None` when there is nothing to draw.
    pub(crate) fn card_slot(&self, plan: &LayoutPlan, side: GutterSide) -> Option<CardSlot> {
        match plan.placement(side) {
            Placement::Hidden => self.gutters.slide_held(side).map(|held| CardSlot {
                placement: held.placement,
                width: held.width,
            }),
            placement => Some(CardSlot {
                placement,
                width: plan.width(side),
            }),
        }
    }

    /// Puts `card` at its slid position in a full-size layer. The layer is the same widget at
    /// rest as mid-slide (a different shape would rebuild the card and reset the nav list's
    /// scroll offset), and the card is always laid out at its natural width, so it is moved,
    /// never squeezed, as it nears an edge.
    fn slid_card<'a>(&self, card: Element<'a, Message>, side: GutterSide) -> Element<'a, Message> {
        let edge = match side {
            GutterSide::Left => Edge::Start,
            GutterSide::Right => Edge::End,
        };
        SlideLayer::new(card, edge, self.gutters.slide_progress(side)).into()
    }

    /// A nav label that ends in an ellipsis when it does not fit, and shows in full on hover.
    fn elided_label(
        &self,
        label: String,
        size: f32,
        color: Option<iced::Color>,
    ) -> Element<'static, Message> {
        let mut text = EllipsisText::new(label.clone()).size(chrome_size(size));
        if let Some(color) = color {
            text = text.color(color);
        }
        ElidedTooltip::new(text, tip(label, None, self.appearance)).into()
    }

    pub(crate) fn folder_file_row(
        &self,
        root: &Path,
        path: &Path,
        selected: bool,
        show_location: bool,
        depth: usize,
    ) -> Element<'static, Message> {
        let colors = theme::colors(self.appearance);
        // Being renamed: the field takes the row's place, at the row's own indent.
        if let Some(draft) = self.rename_draft_for(path) {
            let slot = if show_location {
                0.0
            } else {
                spacing::list::CHEVRON_SLOT + spacing::GAP
            };
            return self.rename_row(draft, depth as f32 * spacing::list::TREE_INDENT + slot);
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut label = column![self.elided_label(name, 13.0, None)]
            .spacing(spacing::list::ROW_GAP)
            .width(Fill);
        if show_location {
            let parent = parent_key(root, path);
            if !parent.as_os_str().is_empty() {
                label = label.push(self.elided_label(
                    parent.display().to_string(),
                    11.0,
                    Some(colors.text_muted),
                ));
            }
        }
        let file_icon = container(svg_icon(
            icons::FILE,
            if selected {
                colors.on_accent
            } else {
                colors.file_icon
            },
            14.0,
        ))
        .width(spacing::list::ICON_SLOT)
        .align_x(Alignment::Center);
        // In the tree, the chevron's slot stays empty so file icons line up with folder icons. It
        // sits outside the highlight, with the indent, so a file's highlight hugs its own icon
        // and text at every depth, root included. Flat search results have no tree and no slot.
        let slot = if show_location {
            0.0
        } else {
            spacing::list::CHEVRON_SLOT + spacing::GAP
        };
        let content = row![file_icon, label]
            .spacing(spacing::GAP)
            .align_y(Alignment::Center);
        let button = press_to_open(
            content,
            spacing::list::FILE_ROW_PAD,
            Fill,
            theme::sidebar_row(self.appearance, selected),
            path,
        );
        // The indent sits outside the button so the highlight starts at the row's own content.
        let row = row![
            Space::new().width(depth as f32 * spacing::list::TREE_INDENT + slot),
            button
        ];
        let target = ContextTarget::NavFile(path.to_path_buf());
        mark_cursor(
            ContextArea::new(row, move |at| Message::OpenContextMenu(target.clone(), at)).into(),
            selected,
        )
    }

    pub(crate) fn folder_tree_rows(
        &self,
        root: &Path,
        node: &FolderNode,
        path: &Path,
        depth: usize,
        selected: Option<&Document>,
    ) -> Column<'static, Message> {
        let colors = theme::colors(self.appearance);
        let mut rows = column![].spacing(spacing::list::TREE_ROW_GAP);
        for (name, child) in &node.folders {
            let child_path = path.join(name);
            let expanded = self.nav.expanded.contains(&child_path);
            let chevron = if expanded {
                icons::CHEVRON_DOWN
            } else {
                icons::CHEVRON_RIGHT
            };
            let folder_button = button(
                row![
                    svg_icon(chevron, colors.text_muted, spacing::list::CHEVRON_SLOT),
                    svg_icon(
                        icons::FOLDER,
                        if self.nav.cursor.as_deref() == Some(child_path.as_path()) {
                            colors.on_accent
                        } else {
                            colors.folder_icon
                        },
                        16.0
                    ),
                    self.elided_label(name.clone(), 13.0, None),
                ]
                .spacing(spacing::GAP)
                .align_y(Alignment::Center),
            )
            .style(theme::sidebar_row(
                self.appearance,
                self.nav.cursor.as_deref() == Some(child_path.as_path()),
            ))
            .padding(spacing::list::FILE_ROW_PAD)
            .width(Fill)
            .on_press(Message::ToggleFolder(child_path.clone()));
            let target = ContextTarget::NavFolder(child_path.clone());
            rows = rows.push(match self.rename_draft_for(&child_path) {
                Some(draft) => self.rename_row(draft, depth as f32 * spacing::list::TREE_INDENT),
                None => mark_cursor(
                    ContextArea::new(
                        row![
                            Space::new().width(depth as f32 * spacing::list::TREE_INDENT),
                            folder_button
                        ],
                        move |at| Message::OpenContextMenu(target.clone(), at),
                    )
                    .into(),
                    self.nav.cursor.as_deref() == Some(child_path.as_path()),
                ),
            });
            if expanded {
                rows =
                    rows.push(self.folder_tree_rows(root, child, &child_path, depth + 1, selected));
            }
        }
        for file in &node.files {
            let is_selected = is_highlighted(
                self.nav.cursor.as_deref(),
                selected.map(|doc| doc.path.as_path()),
                file,
            );
            rows = rows.push(self.folder_file_row(root, file, is_selected, false, depth));
        }
        rows
    }

    pub(crate) fn file_list<'a>(
        &'a self,
        root: &'a Path,
        files: &'a [PathBuf],
        query: &'a str,
        selected: Option<&'a Document>,
    ) -> Element<'a, Message> {
        let matches = matching_files(root, files, query);
        let mut list: Column<'_, Message> = column![].spacing(spacing::list::TREE_ROW_GAP);
        if query.trim().is_empty() {
            let tree = FolderNode::from_files(root, files);
            list = list.push(self.folder_tree_rows(root, &tree, root, 0, selected));
        } else {
            for path in &matches {
                let is_selected = is_highlighted(
                    self.nav.cursor.as_deref(),
                    selected.map(|doc| doc.path.as_path()),
                    path,
                );
                list = list.push(self.folder_file_row(root, path, is_selected, true, 0));
            }
        }
        if files.is_empty() {
            column![
                text("No Markdown files").size(chrome_size(14.0)),
                text("This folder has no Markdown files to preview.").size(chrome_size(12.0))
            ]
            .spacing(spacing::GAP)
            .into()
        } else if matches.is_empty() {
            column![
                text("No matches").size(chrome_size(14.0)),
                text("Try another file or folder name.").size(chrome_size(12.0))
            ]
            .spacing(spacing::GAP)
            .into()
        } else {
            // Rows claim their own right-clicks; whatever is left, the list's padding and the
            // empty space below the last row, opens the menu for the root.
            ContextArea::new(
                scrollable(list.width(Fill))
                    .id(NAV_SCROLL)
                    .width(Fill)
                    .height(Fill),
                |at| Message::OpenContextMenu(ContextTarget::NavBlank, at),
            )
            .into()
        }
    }

    /// The name field while a new item is being named, pinned to the top of the nav so it's never
    /// scrolled out of view by a long tree. The New button that used to sit here lives in the nav
    /// header now (`State::new_item_button`).
    fn nav_new_item_field(&self) -> Element<'static, Message> {
        self.nav
            .draft
            .as_ref()
            .filter(|draft| draft.rename.is_none() && self.draft_inline())
            .map_or_else(|| Space::new().into(), |draft| self.draft_footer(draft))
    }

    /// The Find bar above the preview: the query, the match counter and the step and close buttons.
    fn find_bar(&self) -> Element<'_, Message> {
        let position = if self.find.matches.is_empty() {
            0
        } else {
            self.find.active + 1
        };
        container(
            row![
                // No `.on_submit`: Enter and Shift+Enter are
                // keymap bindings (`Scope::Find`), so the
                // global key handler tells them apart.
                text_input("Find in document", &self.find.query)
                    .id(FIND_INPUT)
                    .on_input(Message::FindChanged)
                    .width(Fill),
                text(format!("{position}/{}", self.find.matches.len())).size(chrome_size(12.0)),
                button(svg_icon(
                    icons::CHEVRON_UP,
                    theme::colors(self.appearance).text,
                    14.0
                ))
                .on_press(Message::FindPrevious),
                button(svg_icon(
                    icons::CHEVRON_DOWN,
                    theme::colors(self.appearance).text,
                    14.0
                ))
                .on_press(Message::FindNext),
                button(svg_icon(
                    icons::CLOSE,
                    theme::colors(self.appearance).text,
                    14.0
                ))
                .on_press(Message::CloseFind),
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        )
        .padding(Padding {
            top: 8.0,
            right: PREVIEW_INSET,
            bottom: 8.0,
            left: PREVIEW_INSET,
        })
        .into()
    }

    /// The preview's scrolled content, with the minimap strip laid over it when it is turned on.
    fn with_minimap<'a>(&'a self, scroller: Element<'a, Message>) -> Element<'a, Message> {
        let geometry = self.minimap.geometry;
        if self.config.preview.minimap {
            // The strip is always in the layout while the minimap is on, blank when
            // there is nothing to show, so a document switch never resizes the preview.
            let scrolls = geometry.content_height > geometry.viewport_height
                && !self.minimap.blocks.is_empty();
            let marks = if scrolls {
                let searching: &[usize] = if self.find.visible {
                    &self.find.matches
                } else {
                    &[]
                };
                Marks::new(
                    &self.minimap.blocks,
                    geometry.offset,
                    searching,
                    self.find.active,
                )
            } else {
                Marks::default()
            };
            let (bars, slider) = if scrolls {
                (geometry.bars(&self.minimap.blocks), geometry.slider())
            } else {
                (Vec::new(), (0.0, 0.0))
            };
            // The scrollbar stays on the preview's right edge; the strip is laid over
            // the scrollable just inside it, and the text keeps clear of it.
            let strip = row![
                Space::new().width(Fill),
                Minimap::new(
                    bars,
                    slider,
                    theme::colors(self.appearance),
                    Message::MinimapSeek,
                )
                .with_marks(marks),
                Space::new().width(minimap::SCROLLBAR_WIDTH),
            ];
            // Only the scrollable is stacked, so the find bar above it stays clear.
            stack![scroller, strip].width(Fill).height(Fill).into()
        } else {
            scroller
        }
    }

    pub(crate) fn folder_panes<'a>(
        &'a self,
        root: &'a Path,
        files: &'a [PathBuf],
        query: &'a str,
        selected: Option<&'a Document>,
    ) -> PanelBody<'a> {
        let plan = self.layout_plan();
        let left: Element<'a, Message> = if self.gutters.left_tab == LeftTab::Recent {
            self.recent_panel()
        } else {
            column![
                self.nav_new_item_field(),
                self.file_list(root, files, query, selected),
            ]
            .spacing(spacing::GAP)
            .height(Fill)
            .into()
        };
        let right: Element<'_, Message> = if self.gutters.right_tab == RightTab::Outline {
            selected.map_or_else(
                || text("Select a file to see its outline.").into(),
                |doc| outline_sidebar(&doc.outline, self.appearance),
            )
        } else {
            selected.map_or_else(
                || text("Select a file to see its details.").into(),
                |doc| {
                    inspector_panel(
                        doc,
                        self.menus.stats_format,
                        &self.copy_flash,
                        self.appearance,
                    )
                },
            )
        };
        let detail: Element<'_, Message> = match selected {
            Some(doc) => {
                let mut preview = column![].height(Fill);
                if self.find.visible {
                    preview = preview.push(self.find_bar());
                }
                let drag = Drag {
                    selecting: self.selecting,
                    autoscrolling: self.selecting && self.autoscroll != 0.0,
                };
                // No padding at all: the scrollbar sits on the pane's edge, and the scrolled text
                // runs under the toolbar and to the window's bottom, so it is cut off cleanly
                // instead of leaving a gap. The text inset lives in the scrolled content (see
                // `render_markdown`).
                preview = preview.push(match self.document_view {
                    DocumentView::Preview => {
                        let scroller = render_markdown(
                            &doc.items,
                            self.appearance,
                            &self.find,
                            self.selection,
                            drag,
                            TextSetting {
                                measure: plan.text_measure(),
                                font: self.config.preview.font,
                                reserve_right: if self.config.preview.minimap {
                                    minimap::WIDTH
                                } else {
                                    0.0
                                },
                                zoom: self.zoom,
                            },
                            &self.images,
                        );
                        self.with_minimap(scroller)
                    }
                    // The minimap's blocks are measured from the rendered preview, so it stays
                    // hidden over raw source (see the issue #71 design note).
                    DocumentView::Source => crate::ui::source_view::render_source(
                        &doc.raw,
                        self.appearance,
                        &self.find,
                        self.selection,
                        drag,
                        crate::ui::source_view::SourceSetting {
                            measure: plan.text_measure(),
                            plain: doc.kind == DocumentKind::Text,
                            zoom: self.zoom,
                        },
                    ),
                });
                container(preview).width(Fill).height(Fill).into()
            }
            None => center(
                text(if files.is_empty() {
                    "Choose another folder with Markdown files."
                } else {
                    "Select a file from the sidebar."
                })
                .color(theme::colors(self.appearance).text_muted),
            )
            .into(),
        };
        // Only Files (left) ever takes `Strip`, and only the right gutter
        // ever takes `Overlay` (see `layout::plan`), so each side needs its
        // own card logic rather than a shared "the one overlay side" branch.
        // A side the plan has already hidden is still drawn, sliding out, until it is gone.
        let left_slot = self.card_slot(&plan, GutterSide::Left);
        let right_slot = self.card_slot(&plan, GutterSide::Right);
        let left_overlay = left_slot
            .filter(|slot| slot.placement == Placement::Docked)
            .map(|slot| {
                self.slid_card(
                    self.with_resize_strip(
                        gutter_card(
                            left,
                            Some(self.left_nav_header()),
                            self.appearance,
                            slot.width,
                            docked_margin(GutterSide::Left),
                            GutterSide::Left,
                            spacing::list::CARD_PAD,
                        ),
                        GutterSide::Left,
                        slot.width,
                    ),
                    GutterSide::Left,
                )
            });
        let right_overlay = right_slot.and_then(|slot| match slot.placement {
            Placement::Docked => Some(self.with_resize_strip(
                gutter_card(
                    right,
                    Some(self.right_gutter_header()),
                    self.appearance,
                    slot.width,
                    docked_margin(GutterSide::Right),
                    GutterSide::Right,
                    spacing::CARD_PAD,
                ),
                GutterSide::Right,
                slot.width,
            )),
            Placement::Overlay => Some(gutter_card(
                right,
                Some(self.right_gutter_header()),
                self.appearance,
                slot.width,
                overlay_margin(),
                GutterSide::Right,
                spacing::CARD_PAD,
            )),
            Placement::Hidden | Placement::Strip => None,
        });
        let right_overlay = right_slot
            .zip(right_overlay)
            .map(|(_, card)| self.slid_card(card, GutterSide::Right));

        let left_docked = left_slot.is_some_and(|slot| slot.placement == Placement::Docked);
        let right_docked = right_slot.is_some_and(|slot| slot.placement == Placement::Docked);
        let mut panels = row![];
        if let Some(slot) = left_slot.filter(|_| left_docked) {
            let progress = self.gutters.slide_progress(GutterSide::Left);
            panels = panels
                .push(Space::new().width((slot.width + budget::gutter_overhead()) * progress));
        }
        panels = panels.push(container(detail).width(Fill));
        if let Some(slot) = right_slot.filter(|_| right_docked) {
            let progress = self.gutters.slide_progress(GutterSide::Right);
            panels = panels
                .push(Space::new().width((slot.width + budget::gutter_overhead()) * progress));
        }
        let content: Element<'a, Message> = if plan.left == Placement::Strip {
            column![self.mini_nav(root, files, query, selected), panels].into()
        } else {
            panels.into()
        };
        PanelBody {
            content,
            left_overlay,
            right_overlay,
        }
    }

    /// The welcome screen's body plus its left panel's floating card, the same
    /// `layout_plan`/`gutter_card`/`with_resize_strip`/`slid_card` path `folder_panes` uses.
    /// Welcome has no Files tab and no right gutter: the panel's content is always
    /// `recent_panel`, and it never docks/overlays the right side.
    pub(crate) fn welcome_panes(&self) -> PanelBody<'_> {
        let plan = self.layout_plan();
        let left = self.recent_panel();
        let left_slot = self.card_slot(&plan, GutterSide::Left);
        let left_overlay = left_slot
            .filter(|slot| slot.placement == Placement::Docked)
            .map(|slot| {
                self.slid_card(
                    self.with_resize_strip(
                        gutter_card(
                            left,
                            Some(self.welcome_panel_header()),
                            self.appearance,
                            slot.width,
                            docked_margin(GutterSide::Left),
                            GutterSide::Left,
                            spacing::list::CARD_PAD,
                        ),
                        GutterSide::Left,
                        slot.width,
                    ),
                    GutterSide::Left,
                )
            });
        let left_docked = left_slot.is_some_and(|slot| slot.placement == Placement::Docked);
        let mut panels = row![];
        if let Some(slot) = left_slot.filter(|_| left_docked) {
            let progress = self.gutters.slide_progress(GutterSide::Left);
            panels = panels
                .push(Space::new().width((slot.width + budget::gutter_overhead()) * progress));
        }
        panels = panels.push(container(self.welcome_view()).width(Fill).height(Fill));
        PanelBody {
            content: panels.into(),
            left_overlay,
            right_overlay: None,
        }
    }

    /// Files as one horizontally scrolling row of chips, shown above the
    /// preview in compact windows. It follows the toolbar search filter.
    pub(crate) fn mini_nav<'a>(
        &'a self,
        root: &'a Path,
        files: &'a [PathBuf],
        query: &'a str,
        selected: Option<&'a Document>,
    ) -> Element<'a, Message> {
        let appearance = self.appearance;
        let matches = matching_files(root, files, query);
        let content: Element<'a, Message> = if matches.is_empty() {
            text(if files.is_empty() {
                "No Markdown files"
            } else {
                "No matches"
            })
            .size(chrome_size(12.0))
            .color(theme::colors(appearance).text_muted)
            .into()
        } else {
            let mut chips = row![]
                .spacing(spacing::GAP_TIGHT)
                .align_y(Alignment::Center);
            for path in matches {
                let active = is_highlighted(
                    self.nav.cursor.as_deref(),
                    selected.map(|doc| doc.path.as_path()),
                    path,
                );
                let label = path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .display()
                    .to_string();
                chips = chips.push(press_to_open(
                    text(label).size(chrome_size(12.0)),
                    spacing::list::CHIP_PAD,
                    Length::Shrink,
                    theme::sidebar_row(appearance, active),
                    path,
                ));
            }
            scrollable(chips)
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .into()
        };
        column![
            container(content)
                .width(Fill)
                .padding(spacing::list::STRIP_PAD)
                .style(move |_| theme::mini_nav_panel(appearance)),
            container(Space::new().height(1).width(Fill))
                .style(move |_| theme::footer_divider(appearance)),
        ]
        .into()
    }
}

pub(crate) fn search_bar(query: &str, appearance: Appearance) -> Element<'_, Message> {
    let colors = theme::colors(appearance);
    let mut field = row![
        svg_icon(icons::SEARCH, colors.text_muted, 14.0),
        // No padding of its own, and the pill fixes its own height below: a text field's
        // default padding, left alone, makes it taller than its toolbar siblings — enough to
        // poke past the toolbar's own border — and vertical centering here is what keeps it
        // looking centered despite that removed padding, rather than hugging the pill's top.
        text_input("Search files…", query)
            .id(SEARCH_INPUT)
            .on_input(Message::SearchChanged)
            .size(chrome_size(13.0))
            .padding(0)
            .style(move |t, s| theme::search_pill_input(appearance)(t, s))
            .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    // The clear button is the far right of the pill, and only there when there is text to clear.
    if !query.is_empty() {
        field = field.push(
            tooltip(
                button(svg_icon(icons::CLOSE, colors.text_muted, 12.0))
                    .padding(spacing::HALF)
                    .style(move |t, s| theme::search_clear_button(appearance)(t, s))
                    .on_press(Message::SearchCleared),
                tip("Clear search", None, appearance),
                tooltip::Position::Bottom,
            )
            .gap(spacing::TOOLTIP_GAP),
        );
    }
    container(field)
        .width(Fill)
        .height(TOOLBAR_CONTROL_HEIGHT)
        .padding(Padding {
            // A little less on the right when the clear button is there, so it sits close to the
            // pill's curve.
            right: if query.is_empty() {
                spacing::CONTROL_PAD_H
            } else {
                spacing::CONTROL_PAD_H - spacing::HALF
            },
            left: spacing::CONTROL_PAD_H,
            ..Padding::ZERO
        })
        .align_y(Alignment::Center)
        .style(move |_| theme::search_pill(appearance))
        .into()
}

/// A sidebar row that opens `path` on mouse-down instead of on release, so the file renders
/// without waiting for the click to finish. The inner `mouse_area` publishes and captures the
/// press, so the button keeps its hover styling but never reaches its own release action.
fn press_to_open<'a>(
    content: impl Into<Element<'a, Message>>,
    padding: [f32; 2],
    width: Length,
    style: impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'a,
    path: &Path,
) -> Element<'a, Message> {
    let message = Message::FileSelected(path.to_path_buf());
    button(
        iced::widget::mouse_area(container(content).padding(padding).width(width))
            .on_press(message.clone()),
    )
    .padding(0)
    .width(width)
    .style(style)
    .on_press(message)
    .into()
}

/// Tags the highlighted row so the nav can scroll it into view.
fn mark_cursor(row: Element<'static, Message>, highlighted: bool) -> Element<'static, Message> {
    if highlighted {
        container(row).id(NAV_CURSOR).into()
    } else {
        row
    }
}
