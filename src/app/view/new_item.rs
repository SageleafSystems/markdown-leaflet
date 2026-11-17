// SPDX-License-Identifier: GPL-3.0-or-later
//! The name field for a new or renamed file or folder: in the nav's footer beside the New button,
//! or in a dialog when the nav is not docked. Also the delete confirmation.

use crate::app::{Message, NEW_ITEM_INPUT, State, TOOLBAR_CONTROL_HEIGHT, chrome_size};
use crate::domain::create::{self, ItemKind, NewItem};
use crate::layout::{Placement, spacing};
use crate::ui::icons::svg_icon;
use crate::ui::{icons, theme};
use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input};
use iced::{Alignment, Element, Fill};

impl State {
    /// Whether the draft is named in the nav itself (a new item in the footer, a rename in its
    /// row): true while the Files gutter is docked. Otherwise (a narrow window, Files hidden) it
    /// is named in a dialog.
    pub(crate) fn draft_inline(&self) -> bool {
        self.nav.draft.is_some() && self.layout_plan().left == Placement::Docked
    }

    /// The draft, if it is renaming `path` and the tree is on screen to show the field in place.
    pub(crate) fn rename_draft_for(&self, path: &std::path::Path) -> Option<&NewItem> {
        self.nav
            .draft
            .as_ref()
            .filter(|draft| draft.rename.as_deref() == Some(path) && self.draft_inline())
    }

    /// The name field standing in for the row being renamed, indented to match it. Its hover flag
    /// tells a press on the field from a press elsewhere, which finishes the edit.
    pub(crate) fn rename_row<'a>(&self, draft: &NewItem, indent: f32) -> Element<'a, Message> {
        row![
            Space::new().width(indent),
            mouse_area(self.draft_field(draft))
                .on_enter(Message::NewItemHover(true))
                .on_exit(Message::NewItemHover(false)),
        ]
        .into()
    }

    /// The reason the draft can't be applied yet, if any.
    fn draft_problem(draft: &NewItem) -> Option<String> {
        draft.failure.clone().or_else(|| {
            create::validate_item(draft)
                .err()
                .map(|error| error.to_string())
        })
    }

    /// What the draft is for, as a line above its field.
    fn draft_caption(&self, draft: &NewItem) -> String {
        if let Some(source) = &draft.rename {
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            return format!("Rename “{name}”");
        }
        let place = self
            .root()
            .and_then(|root| draft.parent.strip_prefix(root).ok())
            .map(|relative| relative.display().to_string())
            .filter(|relative| !relative.is_empty())
            .map_or_else(
                || "the open folder".to_owned(),
                |relative| format!("{relative}/"),
            );
        match draft.kind {
            ItemKind::File => format!("New Markdown file in {place}"),
            ItemKind::Folder => format!("New folder in {place}"),
        }
    }

    /// The name box, with the `.md` hint and the problem beneath it.
    fn draft_field<'a>(&self, draft: &NewItem) -> Element<'a, Message> {
        let appearance = self.appearance;
        let colors = theme::colors(appearance);
        let icon = match draft.kind {
            ItemKind::File => icons::FILE,
            ItemKind::Folder => icons::FOLDER,
        };
        let mut line = row![
            svg_icon(icon, colors.text_muted, 14.0),
            text_input("Name", &draft.text)
                .id(NEW_ITEM_INPUT)
                .on_input(Message::NewItemChanged)
                .on_submit(Message::NewItemSubmit)
                .size(chrome_size(13.0))
                .padding(0)
                .style(move |t, s| theme::search_pill_input(appearance)(t, s))
                .width(Fill),
        ]
        .spacing(spacing::GAP)
        .align_y(Alignment::Center);
        if let Some(ext) = draft.added_extension() {
            line = line.push(
                text(format!(".{ext}"))
                    .size(chrome_size(13.0))
                    .color(colors.text_muted),
            );
        }
        // The same corner radius as a file row's highlight.
        let boxed = container(line)
            .width(Fill)
            .height(TOOLBAR_CONTROL_HEIGHT)
            .padding([0.0, spacing::CONTROL_PAD_H])
            .align_y(Alignment::Center)
            .style(move |_| theme::name_field(appearance));
        let mut field = column![boxed].spacing(spacing::list::ROW_GAP);
        if let Some(problem) = Self::draft_problem(draft) {
            field = field.push(text(problem).size(chrome_size(11.0)).color(colors.danger));
        }
        field.into()
    }

    /// The footer block: a caption and the field. Its hover flag tells a press on the field from
    /// a press elsewhere, which finishes the edit.
    pub(crate) fn draft_footer<'a>(&self, draft: &NewItem) -> Element<'a, Message> {
        let colors = theme::colors(self.appearance);
        mouse_area(
            column![
                text(self.draft_caption(draft))
                    .size(chrome_size(11.0))
                    .color(colors.text_muted),
                self.draft_field(draft),
            ]
            .spacing(spacing::list::ROW_GAP),
        )
        .on_enter(Message::NewItemHover(true))
        .on_exit(Message::NewItemHover(false))
        .into()
    }

    pub(crate) fn new_item_dialog(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        let Some(draft) = &self.nav.draft else {
            return Space::new().into();
        };
        let (title, action) = match (&draft.rename, draft.kind) {
            (Some(_), ItemKind::File) => ("Rename File", "Rename"),
            (Some(_), ItemKind::Folder) => ("Rename Folder", "Rename"),
            (None, ItemKind::File) => ("New Markdown File", "Create"),
            (None, ItemKind::Folder) => ("New Folder", "Create"),
        };
        let confirm = button(text(action).size(chrome_size(13.0)))
            .padding([spacing::units(0.75), spacing::units(2.0)])
            .style(theme::dialog_button_primary(appearance))
            .on_press_maybe(
                Self::draft_problem(draft)
                    .is_none()
                    .then_some(Message::NewItemSubmit),
            );
        let cancel = button(text("Cancel").size(chrome_size(13.0)))
            .padding([spacing::units(0.75), spacing::units(2.0)])
            .style(theme::dialog_button_secondary(appearance))
            .on_press(Message::NewItemCancel);
        let content = column![
            text(title).size(chrome_size(19.0)),
            text(self.draft_caption(draft))
                .size(chrome_size(12.0))
                .color(theme::colors(appearance).text_muted),
            self.draft_field(draft),
            row![confirm, cancel].spacing(spacing::GAP),
        ]
        .spacing(spacing::units(1.5));
        container(content)
            .width(iced::Length::Fixed(400.0))
            .padding(spacing::units(3.0))
            .style(move |_| theme::dialog_panel(appearance))
            .into()
    }

    pub(crate) fn confirm_delete_dialog(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        let path = self.nav.pending_delete.as_deref();
        let name = path
            .and_then(|path| path.file_name())
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let contents = path
            .filter(|path| path.is_dir())
            .zip(match &self.mode {
                crate::app::Mode::Folder { files, .. } => Some(files),
                crate::app::Mode::Welcome => None,
            })
            .map(|(dir, files)| files.iter().filter(|file| file.starts_with(dir)).count());
        let message = match contents {
            Some(1) => {
                format!("The folder “{name}” and its 1 Markdown file will be moved to the Trash.")
            }
            Some(count) => {
                format!(
                    "The folder “{name}” and its {count} Markdown files will be moved to the Trash."
                )
            }
            None => format!("“{name}” will be moved to the Trash."),
        };
        let content = column![
            text("Move to Trash?").size(chrome_size(19.0)),
            text(format!("{message} You can restore it from there.")).size(chrome_size(13.0)),
            row![
                button(text("Move to Trash").size(chrome_size(13.0)))
                    .padding([spacing::units(0.75), spacing::units(2.0)])
                    .style(theme::dialog_button_danger(appearance))
                    .on_press(Message::ConfirmDelete),
                button(text("Cancel").size(chrome_size(13.0)))
                    .padding([spacing::units(0.75), spacing::units(2.0)])
                    .style(theme::dialog_button_secondary(appearance))
                    .on_press(Message::CancelDelete),
            ]
            .spacing(spacing::GAP),
        ]
        .spacing(spacing::units(1.5));
        container(content)
            .width(iced::Length::Fixed(400.0))
            .padding(spacing::units(3.0))
            .style(move |_| theme::dialog_panel(appearance))
            .into()
    }
}
