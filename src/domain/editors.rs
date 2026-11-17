// SPDX-License-Identifier: GPL-3.0-or-later
//! External editors offered by the Open With menu.

/// A handful of common editors to offer in the "Open With" menu. Each is
/// tried via its launcher command (`open::with`), so only the ones actually
/// installed and on `PATH`/registered will do anything; picking one that
/// isn't present just surfaces the OS/`open`-crate error in the banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorOption {
    pub name: &'static str,
    pub command: &'static str,
}

impl std::fmt::Display for EditorOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// The only commands the app will run to open a file. They are fixed here, never read from
/// configuration or a document, so a Markdown file or `config.toml` cannot make the app run an
/// arbitrary program.
pub const EDITORS: &[EditorOption] = &[
    EditorOption {
        name: "Visual Studio Code",
        command: "code",
    },
    EditorOption {
        name: "Cursor",
        command: "cursor",
    },
    EditorOption {
        name: "Zed",
        command: "zed",
    },
    EditorOption {
        name: "Sublime Text",
        command: "subl",
    },
    EditorOption {
        name: "gedit",
        command: "gedit",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_names_and_commands_are_unique() {
        for (i, a) in EDITORS.iter().enumerate() {
            for b in &EDITORS[i + 1..] {
                assert_ne!(a.name, b.name);
                assert_ne!(a.command, b.command);
            }
        }
    }

    #[test]
    fn every_editor_command_is_a_bare_program_name() {
        for editor in EDITORS {
            assert!(
                !editor.command.is_empty()
                    && editor
                        .command
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "{} must be a plain command name, not a path or a shell line",
                editor.command
            );
        }
    }
}
