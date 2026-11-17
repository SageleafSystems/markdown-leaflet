// SPDX-License-Identifier: GPL-3.0-or-later
use crate::app::{Dialog, Message, State, chrome_size};
use crate::domain::keymap;
use crate::layout::spacing;
use crate::ui::theme;
use iced::Element;
use iced::widget::{button, column, container, scrollable, svg, text};

impl State {
    pub(crate) fn dialog_view(&self, dialog: Dialog) -> Element<'_, Message> {
        match dialog {
            Dialog::NewItem => return self.new_item_dialog(),
            Dialog::ConfirmDelete => return self.confirm_delete_dialog(),
            Dialog::ExternalLink => return self.external_link_dialog(),
            Dialog::About | Dialog::Shortcuts => {}
        }
        let appearance = self.appearance;
        let mac = cfg!(target_os = "macos");
        let mut content = column![].spacing(spacing::units(1.5));
        if matches!(dialog, Dialog::About) {
            content = content.push(
                svg(svg::Handle::from_memory(
                    include_bytes!("../../../assets/brand/markdown-leaflet-icon.svg").as_slice(),
                ))
                .width(72)
                .height(72),
            );
        }
        let (title, width): (&str, f32) = match dialog {
            Dialog::About => ("About Markdown Leaflet", 390.0),
            Dialog::Shortcuts => ("Keyboard Shortcuts", 460.0),
            Dialog::NewItem | Dialog::ConfirmDelete | Dialog::ExternalLink => {
                unreachable!("drawn above")
            }
        };
        content = content.push(text(title).size(chrome_size(19.0)));
        match dialog {
            Dialog::About => {
                content = content
                    .push(
                        text(format!(
                            "Version {} ({})",
                            env!("CARGO_PKG_VERSION"),
                            env!("MP_BUILD_REV")
                        ))
                        .size(chrome_size(13.0)),
                    )
                    .push(
                        text("A desktop viewer for Markdown files and folders.")
                            .size(chrome_size(13.0)),
                    );
            }
            Dialog::NewItem | Dialog::ConfirmDelete | Dialog::ExternalLink => {
                unreachable!("drawn above")
            }
            Dialog::Shortcuts => {
                let mut body = column![].spacing(spacing::units(1.0));
                for (group, lines) in keymap::help_sections(mac) {
                    body = body.push(text(group).size(chrome_size(12.0)));
                    for line in lines {
                        body = body.push(text(line).size(chrome_size(13.0)));
                    }
                }
                content = content.push(
                    container(scrollable(body).height(iced::Length::Fixed(360.0)))
                        .width(iced::Length::Fill)
                        .padding(spacing::units(1.5))
                        .style(move |_| theme::dialog_well(appearance)),
                );
            }
        }
        content = content.push(
            button(text("Close").size(chrome_size(13.0)))
                .padding([spacing::units(0.75), spacing::units(2.0)])
                .style(theme::dialog_button_primary(appearance))
                .on_press(Message::DismissDialog),
        );
        container(content)
            .width(iced::Length::Fixed(width))
            .padding(spacing::units(3.0))
            .style(move |_| theme::dialog_panel(appearance))
            .into()
    }
}
