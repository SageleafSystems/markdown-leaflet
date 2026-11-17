// SPDX-License-Identifier: GPL-3.0-or-later
use crate::app::view::toolbar::{tip, with_dropdown_mark};
use crate::app::{Message, chrome_size};
use crate::domain::document::{Document, human_size};
use crate::domain::frontmatter::FrontmatterValue;
use crate::domain::outline::OutlineEntry;
use crate::domain::stats::{DocumentStats, StatsFormat, group_thousands};
use crate::gutter::motion::Tween;
use crate::layout::spacing;
use crate::ui::anchor_area::AnchorArea;
use crate::ui::fonts;
use crate::ui::icons::{self, svg_icon};
use crate::ui::theme;
use crate::ui::theme::Appearance;
use iced::widget::{Space, button, column, container, row, scrollable, text, tooltip};
use iced::{Alignment, Color, Element, Fill, Font, font};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// A section label: small, uppercase, muted.
fn section_label(label: &str, appearance: Appearance) -> Element<'_, Message> {
    text(label)
        .size(chrome_size(11.0))
        .color(theme::colors(appearance).text_muted)
        .into()
}

pub(crate) fn outline_sidebar(
    outline: &[OutlineEntry],
    appearance: Appearance,
) -> Element<'_, Message> {
    let colors = theme::colors(appearance);
    if outline.is_empty() {
        return column![
            section_label("OUTLINE", appearance),
            text("No headings.")
                .size(chrome_size(13.0))
                .color(colors.text_muted),
        ]
        .spacing(spacing::units(1.5))
        .into();
    }

    // The shallowest level present reads as the top level, so a document that starts at H2
    // is not indented as if an H1 were missing.
    let top = outline.iter().map(|entry| entry.level).min().unwrap_or(1);

    let header = row![
        section_label("OUTLINE", appearance),
        Space::new().width(Fill),
        container(
            text(outline.len().to_string())
                .size(chrome_size(11.0))
                .color(colors.text_muted)
        )
        .padding([1.0, spacing::HALF + 2.0])
        .style(move |_| theme::count_badge(appearance)),
    ]
    .align_y(Alignment::Center);

    let list = outline
        .iter()
        .fold(column![].spacing(spacing::list::ROW_GAP), |col, entry| {
            let depth = entry.level.saturating_sub(top);
            let (size, color, weight) = match depth {
                0 => (13.0, colors.text, font::Weight::Bold),
                1 => (13.0, colors.text, font::Weight::Normal),
                _ => (12.0, colors.text_muted, font::Weight::Normal),
            };
            let mut line = row![];
            // One thin guide per ancestor level, so nesting reads at a glance.
            for _ in 0..depth {
                line = line.push(
                    container(
                        container(Space::new().width(1.0).height(Fill))
                            .style(move |_| theme::footer_divider(appearance)),
                    )
                    .padding([0.0, spacing::list::HEADING_GUIDE_PAD]),
                );
            }
            col.push(
                line.push(
                    button(
                        text(entry.title.clone())
                            .size(chrome_size(size))
                            .color(color)
                            .font(Font {
                                weight,
                                ..Font::DEFAULT
                            }),
                    )
                    .style(theme::sidebar_row(appearance, false))
                    .padding(spacing::list::HEADING_ROW_PAD)
                    .width(Fill)
                    .on_press(Message::JumpToHeading(entry.index)),
                ),
            )
        });

    column![header, scrollable(list)]
        .spacing(spacing::GAP)
        .into()
}

const ROW_PAD: [f32; 2] = [spacing::UNIT, spacing::units(1.5)];

/// Right-hand metadata panel toggled by the toolbar's info button. `stats_format` is the "Copy
/// All" button's current type indicator; `copy_flash` is `State::copy_flash`, read to flash a
/// just-copied metric card's value toward the accent color.
pub(crate) fn inspector_panel<'a>(
    doc: &'a Document,
    stats_format: StatsFormat,
    copy_flash: &HashMap<&'static str, Tween>,
    appearance: Appearance,
) -> Element<'a, Message> {
    let colors = theme::colors(appearance);
    let name = doc
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let stats = doc.stats;
    let modified = match (doc.modified_label(), doc.modified_ago()) {
        (Some(date), Some(ago)) => format!("{date} ({ago})"),
        (Some(date), None) => date,
        _ => "unknown".to_string(),
    };

    let file = detail_group(
        vec![
            detail_row("File Name", name, &colors),
            path_row(doc, appearance),
            detail_row("File Size", human_size(doc.size), &colors),
            detail_row_stacked("Modified", modified, &colors),
        ],
        appearance,
    );

    let mut body = column![section_label("FILE", appearance), file];
    if let Some(frontmatter) = doc.frontmatter.as_ref().filter(|fm| !fm.is_empty()) {
        body = body
            .push(frontmatter_header(frontmatter.fields.len(), appearance))
            .push(frontmatter_group(&frontmatter.fields, &colors, appearance));
    }

    scrollable(
        body.push(content_panel(
            stats,
            doc.outline.len(),
            stats_format,
            copy_flash,
            appearance,
        ))
        .spacing(spacing::GAP),
    )
    .into()
}

/// The CONTENT section: the "CONTENT" title as a header, the metric cards as a body, and the
/// "Copy All" button as a footer underneath them.
fn content_panel(
    stats: DocumentStats,
    headings: usize,
    format: StatsFormat,
    copy_flash: &HashMap<&'static str, Tween>,
    appearance: Appearance,
) -> Element<'static, Message> {
    let header = section_label("CONTENT", appearance);
    let cards = metrics_columns(stats, headings, copy_flash, appearance);
    let footer = row![
        Space::new().width(Fill),
        copy_all_button(format, appearance)
    ];

    column![header, cards, footer]
        .spacing(spacing::GAP_TIGHT)
        .into()
}

/// The six metrics as two independently-flowing columns of cards, so a short value's card is
/// only as tall as it needs to be instead of matching its row's tallest neighbour — the way a
/// masonry layout staggers rather than lining cards up in a strict grid.
fn metrics_columns(
    stats: DocumentStats,
    headings: usize,
    copy_flash: &HashMap<&'static str, Tween>,
    appearance: Appearance,
) -> Element<'static, Message> {
    let count = group_thousands;
    let metrics = [
        ("Words", count(stats.words)),
        ("Characters", count(stats.characters)),
        ("Lines", count(stats.lines)),
        ("Headings", count(headings)),
        ("Links", count(stats.links)),
        ("Images", count(stats.images)),
    ];
    let mut left = column![].spacing(spacing::GAP_TIGHT).width(Fill);
    let mut right = column![].spacing(spacing::GAP_TIGHT).width(Fill);
    for (i, (label, value)) in metrics.into_iter().enumerate() {
        let flash = copy_flash
            .get(label)
            .map_or(0.0, |tween| tween.sample(Instant::now()).0);
        let card = metric_card(label, value, flash, appearance);
        if i % 2 == 0 {
            left = left.push(card);
        } else {
            right = right.push(card);
        }
    }
    row![left, right].spacing(spacing::GAP_TIGHT).into()
}

/// One metric's card: pressing anywhere on it copies its "Label: value" pair and starts `flash`
/// (0.0 at rest, 1.0 right after a copy) easing its value back from the accent color. The copy
/// icon is just a mark, not a separate press target — the whole card is one, so it carries no
/// tooltip of its own. Its heading is set in the bundled monospace face, reading as a
/// measurement rather than prose.
fn metric_card(
    label: &'static str,
    value: String,
    flash: f32,
    appearance: Appearance,
) -> Element<'static, Message> {
    let colors = theme::colors(appearance);
    let value_color = mix(colors.text, colors.accent, flash);
    button(
        column![
            row![
                text(label)
                    .size(chrome_size(11.0))
                    .color(colors.text_muted)
                    .font(fonts::mono()),
                Space::new().width(Fill),
                svg_icon(icons::COPY, colors.text_muted, 12.0),
            ]
            .align_y(Alignment::Center),
            text(value.clone())
                .size(chrome_size(19.0))
                .color(value_color)
                .font(Font {
                    weight: font::Weight::Bold,
                    ..Font::DEFAULT
                }),
        ]
        .spacing(spacing::GAP_TIGHT / 2.0),
    )
    .width(Fill)
    .padding([spacing::UNIT, spacing::UNIT + spacing::HALF])
    .style(theme::card_button(appearance))
    .on_press(Message::CopyMetric(label, value))
    .into()
}

/// `base` moved toward `target` by `t` in `0.0..=1.0`.
fn mix(base: Color, target: Color, t: f32) -> Color {
    Color {
        r: base.r + (target.r - base.r) * t,
        g: base.g + (target.g - base.g) * t,
        b: base.b + (target.b - base.b) * t,
        a: base.a,
    }
}

/// The "Copy All" button: its label is the format it copies (Plain Text, JSON, YAML, CSV), so
/// the type indicator is the button, not a badge beside it. A left-click copies every metric in
/// that format; a right-click opens a dropdown to change it, the current one checked — the same
/// right-click-for-more pattern as the toolbar's Theme and Font buttons. Rounded on its left
/// edge, the side with nothing else next to it; its right edge keeps the standard icon-button
/// radius, since the right-click dropdown mark sits in that corner. The only tooltip left in
/// this panel, so it keeps one, placed above the cursor instead of beside it.
fn copy_all_button(format: StatsFormat, appearance: Appearance) -> Element<'static, Message> {
    let colors = theme::colors(appearance);
    let button = button(
        row![
            svg_icon(icons::COPY, colors.text, 13.0),
            text(format.label()).size(chrome_size(11.0)),
        ]
        .spacing(spacing::HALF)
        .align_y(Alignment::Center),
    )
    .padding([spacing::HALF, spacing::UNIT + spacing::HALF])
    .style(theme::icon_button_ends(
        appearance,
        false,
        theme::Ends::Left,
        999.0,
    ))
    .on_press(Message::CopyAllStats(format));
    tooltip(
        AnchorArea::on_right_press(
            with_dropdown_mark(button, appearance),
            Message::OpenMetricsFormatDropdown,
        ),
        tip("Copy all · right-click to change format", None, appearance),
        tooltip::Position::Top,
    )
    .gap(spacing::TOOLTIP_GAP)
    .into()
}

/// The FRONTMATTER section label with a field-count badge, matching the Outline tab's header.
fn frontmatter_header(field_count: usize, appearance: Appearance) -> Element<'static, Message> {
    row![
        section_label("FRONTMATTER", appearance),
        Space::new().width(Fill),
        container(
            text(field_count.to_string())
                .size(chrome_size(11.0))
                .color(theme::colors(appearance).text_muted)
        )
        .padding([1.0, spacing::HALF + 2.0])
        .style(move |_| theme::count_badge(appearance)),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Longer than this, a value gets its own line under the label (`detail_row_stacked`) instead
/// of sharing a row with it, the way Modified and Full Path already do.
const STACKED_VALUE_THRESHOLD: usize = 32;

fn scalar_row(label: String, value: String, colors: &theme::Colors) -> Element<'static, Message> {
    if value.chars().count() > STACKED_VALUE_THRESHOLD {
        detail_row_stacked(label, value, colors)
    } else {
        detail_row(label, value, colors)
    }
}

/// Renders a frontmatter block's fields into one `detail_group`, same as the FILE section. A
/// scalar or a flat list of scalars (joined with `, `) is one row; a map, or a list holding
/// anything but scalars, becomes a label row (with a field/item count in place of a value, like
/// a tiny nested FILE group) followed by its own fields or items, each indented one guide line
/// further than its parent — the same nesting language the Outline tab uses for headings.
fn frontmatter_group(
    fields: &[(String, FrontmatterValue)],
    colors: &theme::Colors,
    appearance: Appearance,
) -> Element<'static, Message> {
    let mut rows = Vec::new();
    for (key, value) in fields {
        push_frontmatter_row(key.clone(), value, 0, colors, appearance, &mut rows);
    }
    detail_group(rows, appearance)
}

fn push_frontmatter_row(
    label: String,
    value: &FrontmatterValue,
    depth: usize,
    colors: &theme::Colors,
    appearance: Appearance,
    rows: &mut Vec<Element<'static, Message>>,
) {
    match value {
        FrontmatterValue::Scalar(scalar) => {
            rows.push(indent(
                depth,
                appearance,
                scalar_row(label, scalar.clone(), colors),
            ));
        }
        FrontmatterValue::List(items)
            if items
                .iter()
                .all(|item| matches!(item, FrontmatterValue::Scalar(_))) =>
        {
            let joined = items
                .iter()
                .map(|item| match item {
                    FrontmatterValue::Scalar(scalar) => scalar.as_str(),
                    FrontmatterValue::List(_) | FrontmatterValue::Map(_) => "",
                })
                .collect::<Vec<_>>()
                .join(", ");
            rows.push(indent(depth, appearance, scalar_row(label, joined, colors)));
        }
        FrontmatterValue::List(items) => {
            rows.push(indent(
                depth,
                appearance,
                detail_row(label, format!("[{}]", items.len()), colors),
            ));
            for (index, item) in items.iter().enumerate() {
                push_frontmatter_row(
                    format!("[{index}]"),
                    item,
                    depth + 1,
                    colors,
                    appearance,
                    rows,
                );
            }
        }
        FrontmatterValue::Map(fields) => {
            rows.push(indent(
                depth,
                appearance,
                detail_row(label, format!("{{{}}}", fields.len()), colors),
            ));
            for (key, value) in fields {
                push_frontmatter_row(key.clone(), value, depth + 1, colors, appearance, rows);
            }
        }
    }
}

/// Wraps a row in one thin guide line per nesting level, the same `footer_divider`-styled line
/// `outline_sidebar` nests headings under.
fn indent(
    depth: usize,
    appearance: Appearance,
    content: Element<'static, Message>,
) -> Element<'static, Message> {
    if depth == 0 {
        return content;
    }
    let mut wrapped = row![];
    for _ in 0..depth {
        wrapped = wrapped.push(
            container(
                container(Space::new().width(1.0).height(Fill))
                    .style(move |_| theme::footer_divider(appearance)),
            )
            .padding([0.0, spacing::list::HEADING_GUIDE_PAD]),
        );
    }
    wrapped.push(content).into()
}

/// The full path under its label, with a button that copies it.
fn path_row(doc: &Document, appearance: Appearance) -> Element<'_, Message> {
    let colors = theme::colors(appearance);
    let copy = tooltip(
        button(svg_icon(icons::COPY, colors.text_muted, 14.0))
            .padding(spacing::HALF)
            .style(theme::icon_button(appearance, false))
            .on_press(Message::CopyPath(doc.path.clone())),
        tip("Copy full path", None, appearance),
        tooltip::Position::Left,
    )
    .gap(spacing::TOOLTIP_GAP);
    container(
        column![
            row![
                text("Full Path").size(chrome_size(13.0)),
                Space::new().width(Fill),
                copy
            ]
            .align_y(Alignment::Center),
            text(abbreviate_home(&doc.path))
                .size(chrome_size(12.0))
                .color(colors.text_muted)
                .width(Fill),
        ]
        .spacing(spacing::GAP_TIGHT),
    )
    .padding([spacing::HALF, spacing::units(1.5)])
    .into()
}

/// Rows stacked in one rounded card, each pair separated by a hairline.
fn detail_group(rows: Vec<Element<'_, Message>>, appearance: Appearance) -> Element<'_, Message> {
    let mut body = column![];
    let last = rows.len().saturating_sub(1);
    for (i, row) in rows.into_iter().enumerate() {
        body = body.push(row);
        if i < last {
            body = body.push(
                container(Space::new().width(Fill).height(1.0))
                    .padding([0.0, spacing::units(1.5)])
                    .style(move |_| theme::footer_divider(appearance)),
            );
        }
    }
    container(body)
        .width(Fill)
        .style(move |_| theme::detail_group(appearance))
        .into()
}

/// Label on the left, value on the right.
fn detail_row(
    label: impl Into<String>,
    value: String,
    colors: &theme::Colors,
) -> Element<'static, Message> {
    container(
        row![
            text(label.into()).size(chrome_size(13.0)),
            Space::new().width(Fill),
            text(value).size(chrome_size(13.0)).color(colors.text_muted),
        ]
        .spacing(spacing::GAP)
        .align_y(Alignment::Center),
    )
    .padding(ROW_PAD)
    .into()
}

/// Label over a right-aligned value, for values that need the row's full width.
fn detail_row_stacked(
    label: impl Into<String>,
    value: String,
    colors: &theme::Colors,
) -> Element<'static, Message> {
    container(
        column![
            text(label.into()).size(chrome_size(13.0)),
            text(value)
                .size(chrome_size(13.0))
                .color(colors.text_muted)
                .width(Fill)
                .align_x(Alignment::End),
        ]
        .spacing(spacing::GAP_TIGHT),
    )
    .padding(ROW_PAD)
    .into()
}

/// The path with the home directory shown as `~`.
fn abbreviate_home(path: &Path) -> String {
    dirs::home_dir()
        .and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf))
        .map_or_else(
            || path.display().to_string(),
            |rest| format!("~/{}", rest.display()),
        )
}
