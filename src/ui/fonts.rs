// SPDX-License-Identifier: GPL-3.0-or-later
//! The fonts bundled for the preview body, and the choice-to-`Font` mapping.
//!
//! Source Sans 3, Source Serif 4 and Source Code Pro are Adobe's SIL Open Font License families,
//! instanced to static Regular, Bold, Italic and Bold Italic and subset to Latin (see
//! `assets/fonts/OFL-*.txt`). `System` bundles nothing and uses the platform default.

use crate::config::FontChoice;
use iced::Font;

const SANS: &str = "Source Sans 3";
const SERIF: &str = "Source Serif";
const MONO: &str = "Source Code Pro";

/// Every bundled face, for `iced::Application::font` to register at startup.
pub const BUNDLED: [&[u8]; 12] = [
    include_bytes!("../../assets/fonts/SourceSans-Regular.ttf"),
    include_bytes!("../../assets/fonts/SourceSans-Bold.ttf"),
    include_bytes!("../../assets/fonts/SourceSans-Italic.ttf"),
    include_bytes!("../../assets/fonts/SourceSans-BoldItalic.ttf"),
    include_bytes!("../../assets/fonts/SourceSerif-Regular.ttf"),
    include_bytes!("../../assets/fonts/SourceSerif-Bold.ttf"),
    include_bytes!("../../assets/fonts/SourceSerif-Italic.ttf"),
    include_bytes!("../../assets/fonts/SourceSerif-BoldItalic.ttf"),
    include_bytes!("../../assets/fonts/SourceCode-Regular.ttf"),
    include_bytes!("../../assets/fonts/SourceCode-Bold.ttf"),
    include_bytes!("../../assets/fonts/SourceCode-Italic.ttf"),
    include_bytes!("../../assets/fonts/SourceCode-BoldItalic.ttf"),
];

/// The bundled monospace face, used where a label should read as a measurement rather than
/// prose (the details panel's metric headings), independent of the preview's own font choice.
pub fn mono() -> Font {
    Font::with_name(MONO)
}

/// The font the preview's body text is set in.
pub fn body(choice: FontChoice) -> Font {
    match choice {
        FontChoice::System => Font::DEFAULT,
        FontChoice::Sans => Font::with_name(SANS),
        FontChoice::Serif => Font::with_name(SERIF),
        FontChoice::Mono => Font::with_name(MONO),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::font::Family;

    #[test]
    fn system_keeps_the_platform_default_and_the_rest_name_a_bundled_family() {
        assert_eq!(body(FontChoice::System), Font::DEFAULT);
        for choice in [FontChoice::Sans, FontChoice::Serif, FontChoice::Mono] {
            assert!(matches!(body(choice).family, Family::Name(_)));
        }
    }

    #[test]
    fn bundled_faces_declare_the_family_names_the_choices_use() {
        // Every file's family name must be one `body` asks for, or the choice silently falls back.
        for bytes in BUNDLED {
            // The name table stores Windows names as UTF-16BE.
            let found = [SANS, SERIF, MONO].iter().any(|name| {
                let utf16: Vec<u8> = name.encode_utf16().flat_map(u16::to_be_bytes).collect();
                bytes.windows(utf16.len()).any(|w| w == utf16)
            });
            assert!(found, "a bundled face names none of the families");
        }
    }
}
