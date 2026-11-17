// SPDX-License-Identifier: GPL-3.0-or-later
use crate::app::{ContextTarget, FindState, Message};
use crate::config::FontChoice;
use crate::domain::selection::Selection;
use crate::layout::{budget, spacing, typography};
use crate::ui::context_area::ContextArea;
use crate::ui::find_viewer::FindViewer;
use crate::ui::fonts;
use crate::ui::images::ImageStore;
use crate::ui::locate_heading::DOC_SCROLL;
use crate::ui::selectable_text::Drag;
use crate::ui::selection_area::SelectionArea;
use crate::ui::theme;
use crate::ui::theme::Appearance;
use iced::widget::{Column, column, container, markdown, scrollable};
use iced::{Element, Fill, Padding};

/// Space between the window or a gutter card and the preview's text, in pixels.
pub(crate) const PREVIEW_INSET: f32 = budget::TEXT_INSET;

/// How the preview's text is set: the column width it breaks at and the body font.
#[derive(Clone, Copy)]
pub(crate) struct TextSetting {
    pub measure: f32,
    pub font: FontChoice,
    /// Extra space kept at the text's right edge for something drawn over the scrollable,
    /// such as the minimap.
    pub reserve_right: f32,
    /// Multiplies every text size in the preview (issue #76): 1.0 is the base 16px.
    pub zoom: f32,
}

pub(crate) fn render_markdown<'a>(
    items: &'a [markdown::Item],
    appearance: Appearance,
    find: &'a FindState,
    selection: Option<Selection>,
    drag: Drag,
    text: TextSetting,
    images: &'a ImageStore,
) -> Element<'a, Message> {
    let TextSetting {
        measure,
        font,
        reserve_right,
        zoom,
    } = text;
    let body_font = fonts::body(font);
    let settings = markdown::Settings::with_text_size(
        16.0 * zoom,
        markdown::Style {
            font: body_font,
            ..markdown::Style::from_palette(theme::palette(appearance))
        },
    );
    let body_ratio = typography::body_ratio(settings.text_size.0, measure);
    let mut content: Column<'_, Message> = column![].spacing(settings.spacing);
    let mut heading_index = 0;
    for (item_index, item) in items.iter().enumerate() {
        let active = find.matches.get(find.active).and_then(|block| {
            (*block == item_index).then(|| {
                find.matches[..find.active]
                    .iter()
                    .filter(|&&i| i == item_index)
                    .count()
            })
        });
        let viewer = FindViewer::new(
            if find.visible { &find.query } else { "" },
            active,
            appearance,
            item_index,
            selection,
            drag,
        )
        .with_body_ratio(body_ratio)
        .with_font(body_font)
        .with_images(images);
        let rendered: Element<'_, Message> = markdown::view_with([item], settings, &viewer);
        if matches!(item, markdown::Item::Heading(..)) {
            let top_padding = if item_index > 0 {
                settings.text_size.0 / 2.0
            } else {
                0.0
            };
            content = content.push(
                container(container(rendered).id(format!("document-find-{item_index}")))
                    .id(format!("document-heading-{heading_index}"))
                    .padding(Padding {
                        top: top_padding,
                        ..Padding::ZERO
                    }),
            );
            heading_index += 1;
        } else {
            content = content.push(container(rendered).id(format!("document-find-{item_index}")));
        }
    }
    let preview = scrollable(container(content).padding(Padding {
        top: spacing::units(2.0),
        right: budget::SCROLLBAR_CLEARANCE + reserve_right,
        bottom: spacing::units(2.0),
        left: PREVIEW_INSET,
    }))
    .id(DOC_SCROLL)
    .on_scroll(Message::PreviewScrolled)
    .width(Fill)
    .height(Fill);
    let preview = SelectionArea::new(
        preview,
        drag.selecting,
        Message::SelectionCleared,
        Message::SelectionAutoScroll,
    );
    ContextArea::new(preview, |at| {
        Message::OpenContextMenu(ContextTarget::Preview, at)
    })
    .into()
}
