// SPDX-License-Identifier: GPL-3.0-or-later
//! Palette overrides as written in the config file, parsed to plain RGB so
//! this module needs no rendering types.

/// A color value that is not `#RRGGBB`.
#[derive(Debug, thiserror::Error)]
#[error("expected #RRGGBB, got {0}")]
pub struct ColorError(String);

/// An override that failed to parse, named by its role.
#[derive(Debug, thiserror::Error)]
#[error("{role}: {source}")]
pub struct PaletteError {
    pub role: &'static str,
    pub source: ColorError,
}

/// An 8-bit sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

macro_rules! palette_roles {
    ($($name:ident),+ $(,)?) => {
        /// Raw `#RRGGBB` strings for each optional color role.
        #[derive(Default, Clone, serde::Deserialize)]
        #[serde(default, deny_unknown_fields)]
        pub struct PaletteOverrides {
            $($name: Option<String>,)+
        }

        /// Validated overrides for one palette.
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
        pub struct ParsedPalette {
            $(pub $name: Option<Rgb>,)+
        }

        impl PaletteOverrides {
            /// Parses every role; errors are prefixed with the role name.
            pub fn parse(&self) -> Result<ParsedPalette, PaletteError> {
                Ok(ParsedPalette {
                    $($name: self
                        .$name
                        .as_deref()
                        .map(parse_color)
                        .transpose()
                        .map_err(|source| PaletteError {
                            role: stringify!($name),
                            source,
                        })?,)+
                })
            }
        }
    };
}

palette_roles!(
    background,
    surface,
    surface_alt,
    border,
    text,
    text_muted,
    accent,
    on_accent,
    danger,
    warning,
    resize,
    file_icon,
    folder_icon,
    utility_icon
);

/// Validated overrides for every built-in palette.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ParsedTheme {
    pub slate: ParsedPalette,
    pub harbor: ParsedPalette,
    pub nightfall: ParsedPalette,
    pub linen: ParsedPalette,
    pub paper: ParsedPalette,
}

fn parse_color(value: &str) -> Result<Rgb, ColorError> {
    let hex = value
        .strip_prefix('#')
        .ok_or_else(|| ColorError(value.to_string()))?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ColorError(value.to_string()));
    }
    // The length and digits were checked above, so each pair parses.
    let component =
        |range| u8::from_str_radix(&hex[range], 16).map_err(|_| ColorError(value.to_string()));
    Ok(Rgb(component(0..2)?, component(2..4)?, component(4..6)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_reports_the_role() {
        assert_eq!(parse_color("#123456").unwrap(), Rgb(0x12, 0x34, 0x56));
        let bad = PaletteOverrides {
            accent: Some("red".into()),
            ..PaletteOverrides::default()
        };
        assert_eq!(
            bad.parse().unwrap_err().to_string(),
            "accent: expected #RRGGBB, got red"
        );
        assert!(parse_color("#12345").is_err());
        assert!(parse_color("123456").is_err());
    }

    mod properties {
        use super::super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn color_parsing_never_panics_on_arbitrary_text(text in "\\PC{0,20}") {
                let _ = parse_color(&text);
            }

            #[test]
            fn every_six_digit_hex_color_round_trips(r in any::<u8>(), g in any::<u8>(), b in any::<u8>()) {
                let parsed = parse_color(&format!("#{r:02x}{g:02X}{b:02x}"));
                prop_assert_eq!(parsed.ok(), Some(Rgb(r, g, b)));
            }
        }
    }
}
