// SPDX-License-Identifier: GPL-3.0-or-later
mod palette;

pub use palette::{PaletteError, PaletteOverrides, ParsedPalette, ParsedTheme, Rgb};

use crate::domain::walk::ScanRules;
use std::path::PathBuf;

/// An unrecognized `appearance` value (an old theme name from before a
/// rename, a typo, a future name this build predates) falls back to
/// `System` and logs the unrecognized value to stdout, rather than
/// failing the whole config file — unlike an unknown table key or an
/// invalid palette color, which are typos worth surfacing loudly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppearanceChoice {
    #[default]
    System,
    Slate,
    Harbor,
    Nightfall,
    Linen,
    Paper,
}

impl<'de> serde::Deserialize<'de> for AppearanceChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "system" => Self::System,
            "slate" => Self::Slate,
            "harbor" => Self::Harbor,
            "nightfall" => Self::Nightfall,
            "linen" => Self::Linen,
            "paper" => Self::Paper,
            _ => {
                println!(
                    "markdown-leaflet: unrecognized theme.appearance {raw:?}, falling back to System"
                );
                Self::System
            }
        })
    }
}

impl AppearanceChoice {
    pub const ALL: [Self; 6] = [
        Self::System,
        Self::Slate,
        Self::Harbor,
        Self::Nightfall,
        Self::Linen,
        Self::Paper,
    ];

    /// The next choice in menu order, wrapping from the last back to the first.
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|&c| c == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }

    /// The name shown in the Theme dropdown.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Slate => "Slate",
            Self::Harbor => "Harbor",
            Self::Nightfall => "Nightfall",
            Self::Linen => "Linen",
            Self::Paper => "Paper",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Slate => "slate",
            Self::Harbor => "harbor",
            Self::Nightfall => "nightfall",
            Self::Linen => "linen",
            Self::Paper => "paper",
        }
    }
}

/// The font stack the preview body is set in. Everything but `System` is a
/// font bundled with the app, so it looks the same on every OS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontChoice {
    /// The platform's default UI font.
    #[default]
    System,
    Sans,
    Serif,
    Mono,
}

impl FontChoice {
    pub const ALL: [Self; 4] = [Self::System, Self::Sans, Self::Serif, Self::Mono];

    /// The next choice in dropdown order, wrapping from the last back to the first.
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|&c| c == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Sans => "sans",
            Self::Serif => "serif",
            Self::Mono => "mono",
        }
    }

    /// The face behind the choice, shown beside its name in the Font dropdown.
    pub fn family(self) -> &'static str {
        match self {
            Self::System => "Platform default",
            Self::Sans => "Source Sans 3",
            Self::Serif => "Source Serif 4",
            Self::Mono => "Source Code Pro",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Sans => "Sans",
            Self::Serif => "Serif",
            Self::Mono => "Mono",
        }
    }
}

/// The `[preview]` table: how the rendered Markdown is set.
#[derive(Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewConfig {
    pub font: FontChoice,
    /// Whether the minimap strip is drawn beside the preview's scrollbar.
    pub minimap: bool,
}

/// The `[files]` table: which kinds of file the nav lists.
#[derive(Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilesConfig {
    /// Whether plain-text (`.txt`) files are listed alongside Markdown (issue #79).
    pub show_text: bool,
}

/// What to do when a link points at a Markdown file outside the open folder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalLinkAction {
    /// Show the dialog every time.
    #[default]
    Ask,
    /// Open the file's folder as the workspace.
    Folder,
    /// Open the file alone, leaving the open folder as it is.
    Document,
}

impl ExternalLinkAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Folder => "folder",
            Self::Document => "document",
        }
    }
}

/// The `[links]` table: how clicked links are handled.
#[derive(Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LinksConfig {
    /// What a link to a Markdown file outside the open folder does: `"ask"` (default),
    /// `"folder"` or `"document"`.
    pub external_markdown: ExternalLinkAction,
}

/// The `[animation]` table. Turning it off makes every transition an instant change, for people
/// who find motion tiring (iced cannot read the OS "reduce motion" setting).
#[derive(Clone, Copy, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnimationConfig {
    pub enabled: bool,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// How the left and right cards show their accent gradient.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GradientStyle {
    /// A flat `surface` fill inside a border that fades toward the accent at both ends.
    #[default]
    Border,
    /// A 1px `border` around a fill that fades toward the accent at the bottom.
    Fill,
}

/// The `[theme.gradient]` table: the gradient on the left and right cards.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GradientConfig {
    pub style: GradientStyle,
    /// Degrees the gradient is tilted from straight down; 45 leans it toward the bottom right.
    pub angle: f32,
    /// How strongly the gradient leans toward the accent, 0 (none) to 1 (fully accent).
    pub opacity: f32,
    /// Strength of the accent glow around the card, 0 (off) to 1.
    pub glow: f32,
}

impl GradientConfig {
    pub const DEFAULT: Self = Self {
        style: GradientStyle::Border,
        angle: 45.0,
        opacity: 0.35,
        glow: 0.5,
    };

    /// Replaces a non-finite angle, opacity or glow with the default and clamps the last two to
    /// 0..=1.
    pub fn sanitized(self) -> Self {
        let unit = |value: f32, default: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                default
            }
        };
        Self {
            angle: if self.angle.is_finite() {
                self.angle
            } else {
                Self::DEFAULT.angle
            },
            opacity: unit(self.opacity, Self::DEFAULT.opacity),
            glow: unit(self.glow, Self::DEFAULT.glow),
            ..self
        }
    }
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    pub appearance: AppearanceChoice,
    pub gradient: GradientConfig,
    pub slate: PaletteOverrides,
    pub harbor: PaletteOverrides,
    pub nightfall: PaletteOverrides,
    pub linen: PaletteOverrides,
    pub paper: PaletteOverrides,
}

/// A palette override that failed to parse, named by its palette.
#[derive(Debug, thiserror::Error)]
#[error("theme.{palette}.{source}")]
pub struct ThemeError {
    pub palette: &'static str,
    pub source: PaletteError,
}

/// Why the config file could not be read, validated, or saved.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("No OS config directory is available")]
    NoConfigDir,
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{}: {source}", path.display())]
    Parse {
        path: PathBuf,
        source: Box<UpdateError>,
    },
    #[error(transparent)]
    Theme(#[from] ThemeError),
}

/// Why a config document could not be parsed or edited.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Edit(#[from] toml_edit::TomlError),
    #[error("theme must be a TOML table")]
    ThemeNotTable,
    #[error("preview must be a TOML table")]
    PreviewNotTable,
    #[error("files must be a TOML table")]
    FilesNotTable,
    #[error("links must be a TOML table")]
    LinksNotTable,
    #[error(transparent)]
    Theme(#[from] ThemeError),
}

impl ThemeConfig {
    /// Validates every palette, even those not currently visible.
    pub fn parse(&self) -> Result<ParsedTheme, ThemeError> {
        let parse = |palette: &'static str, overrides: &PaletteOverrides| {
            overrides
                .parse()
                .map_err(|source| ThemeError { palette, source })
        };
        Ok(ParsedTheme {
            slate: parse("slate", &self.slate)?,
            harbor: parse("harbor", &self.harbor)?,
            nightfall: parse("nightfall", &self.nightfall)?,
            linen: parse("linen", &self.linen)?,
            paper: parse("paper", &self.paper)?,
        })
    }
}

/// User config at `<config dir>/markdown-leaflet/config.toml`.
#[derive(serde::Deserialize, Default, Clone)]
pub struct Config {
    /// Directory names to NOT ignore, overriding the default ignore list.
    #[serde(default)]
    pub(crate) include: Vec<String>,
    #[serde(default)]
    pub(crate) theme: ThemeConfig,
    #[serde(default)]
    pub(crate) preview: PreviewConfig,
    #[serde(default)]
    pub(crate) animation: AnimationConfig,
    #[serde(default)]
    pub(crate) links: LinksConfig,
    #[serde(default)]
    pub(crate) files: FilesConfig,
}

impl Config {
    pub fn path() -> Result<PathBuf, ConfigError> {
        // Tests that save settings must never read or overwrite the developer's real config.
        if cfg!(test) {
            // A fixed name, so repeated runs reuse one folder instead of leaving a new one each time.
            return Ok(std::env::temp_dir()
                .join("mdp-test-config")
                .join("config.toml"));
        }
        dirs::config_dir()
            .map(|dir| dir.join("markdown-leaflet/config.toml"))
            .ok_or(ConfigError::NoConfigDir)
    }

    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::path()?;
        match std::fs::read_to_string(&path) {
            Ok(raw) => toml::from_str(&raw).map_err(|source| ConfigError::Parse {
                path,
                source: Box::new(source.into()),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io { path, source }),
        }
    }

    pub fn save_appearance(choice: AppearanceChoice) -> Result<(), ConfigError> {
        Self::save_with(|raw| update_appearance(raw, choice))
    }

    pub fn save_font(choice: FontChoice) -> Result<(), ConfigError> {
        Self::save_with(|raw| update_font(raw, choice))
    }

    pub fn save_minimap(shown: bool) -> Result<(), ConfigError> {
        Self::save_with(|raw| update_minimap(raw, shown))
    }

    pub fn save_show_text(shown: bool) -> Result<(), ConfigError> {
        Self::save_with(|raw| update_show_text(raw, shown))
    }

    pub fn save_external_link_action(choice: ExternalLinkAction) -> Result<(), ConfigError> {
        Self::save_with(|raw| update_external_link_action(raw, choice))
    }

    fn save_with(
        edit: impl FnOnce(&str) -> Result<String, UpdateError>,
    ) -> Result<(), ConfigError> {
        let path = Self::path()?;
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => return Err(ConfigError::Io { path, source }),
        };
        let updated = match edit(&raw) {
            Ok(updated) => updated,
            Err(source) => {
                return Err(ConfigError::Parse {
                    path,
                    source: Box::new(source),
                });
            }
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&path, updated).map_err(|source| ConfigError::Io { path, source })
    }

    pub fn scan_rules(&self) -> ScanRules {
        ScanRules::new(self.include.clone()).with_text(self.files.show_text)
    }
}

fn update_appearance(raw: &str, choice: AppearanceChoice) -> Result<String, UpdateError> {
    let config: Config = toml::from_str(raw)?;
    config.theme.parse()?;
    let mut doc = raw.parse::<toml_edit::DocumentMut>()?;
    if !doc.contains_key("theme") {
        doc["theme"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let table = doc["theme"]
        .as_table_mut()
        .ok_or(UpdateError::ThemeNotTable)?;
    let decor = table
        .get("appearance")
        .and_then(toml_edit::Item::as_value)
        .map(|value| value.decor().clone());
    let mut appearance = toml_edit::value(choice.as_str());
    if let (Some(decor), Some(value)) = (decor, appearance.as_value_mut()) {
        *value.decor_mut() = decor;
    }
    table.insert("appearance", appearance);
    Ok(doc.to_string())
}

fn update_font(raw: &str, choice: FontChoice) -> Result<String, UpdateError> {
    let config: Config = toml::from_str(raw)?;
    config.theme.parse()?;
    let mut doc = raw.parse::<toml_edit::DocumentMut>()?;
    if !doc.contains_key("preview") {
        doc["preview"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let table = doc["preview"]
        .as_table_mut()
        .ok_or(UpdateError::PreviewNotTable)?;
    let decor = table
        .get("font")
        .and_then(toml_edit::Item::as_value)
        .map(|value| value.decor().clone());
    let mut font = toml_edit::value(choice.as_str());
    if let (Some(decor), Some(value)) = (decor, font.as_value_mut()) {
        *value.decor_mut() = decor;
    }
    table.insert("font", font);
    Ok(doc.to_string())
}

fn update_external_link_action(
    raw: &str,
    choice: ExternalLinkAction,
) -> Result<String, UpdateError> {
    let config: Config = toml::from_str(raw)?;
    config.theme.parse()?;
    let mut doc = raw.parse::<toml_edit::DocumentMut>()?;
    if !doc.contains_key("links") {
        doc["links"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let table = doc["links"]
        .as_table_mut()
        .ok_or(UpdateError::LinksNotTable)?;
    let decor = table
        .get("external_markdown")
        .and_then(toml_edit::Item::as_value)
        .map(|value| value.decor().clone());
    let mut action = toml_edit::value(choice.as_str());
    if let (Some(decor), Some(value)) = (decor, action.as_value_mut()) {
        *value.decor_mut() = decor;
    }
    table.insert("external_markdown", action);
    Ok(doc.to_string())
}

fn update_show_text(raw: &str, shown: bool) -> Result<String, UpdateError> {
    let config: Config = toml::from_str(raw)?;
    config.theme.parse()?;
    let mut doc = raw.parse::<toml_edit::DocumentMut>()?;
    if !doc.contains_key("files") {
        doc["files"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let table = doc["files"]
        .as_table_mut()
        .ok_or(UpdateError::FilesNotTable)?;
    table.insert("show_text", toml_edit::value(shown));
    Ok(doc.to_string())
}

fn update_minimap(raw: &str, shown: bool) -> Result<String, UpdateError> {
    let config: Config = toml::from_str(raw)?;
    config.theme.parse()?;
    let mut doc = raw.parse::<toml_edit::DocumentMut>()?;
    if !doc.contains_key("preview") {
        doc["preview"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let table = doc["preview"]
        .as_table_mut()
        .ok_or(UpdateError::PreviewNotTable)?;
    table.insert("minimap", toml_edit::value(shown));
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_font_preserves_comments_and_other_tables() {
        let raw = "# note\n[theme]\nappearance = 'slate'\n\n[preview]\nfont = 'sans' # body\n";
        let updated = update_font(raw, FontChoice::Serif).unwrap();
        assert!(updated.contains("# note"));
        assert!(updated.contains("# body"));
        let config: Config = toml::from_str(&updated).unwrap();
        assert_eq!(config.preview.font, FontChoice::Serif);
        assert_eq!(config.theme.appearance, AppearanceChoice::Slate);
    }

    #[test]
    fn unrecognized_appearance_falls_back_to_system_without_failing_the_file() {
        let raw = "[preview]\nfont = 'serif'\n\n[theme]\nappearance = 'dark'\n";
        let config: Config = toml::from_str(raw).unwrap();
        assert_eq!(config.theme.appearance, AppearanceChoice::System);
        assert_eq!(config.preview.font, FontChoice::Serif);
    }

    #[test]
    fn minimap_defaults_off_and_the_toggle_keeps_other_settings() {
        assert!(!toml::from_str::<Config>("").unwrap().preview.minimap);
        let updated = update_minimap("[preview]\n# keep\nfont = \"serif\"\n", false).unwrap();
        assert!(updated.contains("# keep") && updated.contains("font = \"serif\""));
        let config: Config = toml::from_str(&updated).unwrap();
        assert!(!config.preview.minimap);
    }

    #[test]
    fn changing_font_creates_missing_preview_table() {
        let updated = update_font("", FontChoice::Mono).unwrap();
        let config: Config = toml::from_str(&updated).unwrap();
        assert_eq!(config.preview.font, FontChoice::Mono);
    }

    #[test]
    fn font_defaults_to_system_and_rejects_unknown_names() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.preview.font, FontChoice::System);
        assert!(toml::from_str::<Config>("[preview]\nfont = 'comic'").is_err());
    }

    #[test]
    fn animation_defaults_on_and_can_be_turned_off() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.animation.enabled);
        let config: Config = toml::from_str("[animation]\nenabled = false").unwrap();
        assert!(!config.animation.enabled);
        assert!(toml::from_str::<Config>("[animation]\nspeed = 2").is_err());
    }

    #[test]
    fn changing_appearance_preserves_comments_and_scan_rules() {
        let raw = "# user's note\ninclude = ['target']\n\n[theme]\nappearance = 'system' # follow OS\n\n[theme.slate]\naccent = '#123456'\n";
        let updated = update_appearance(raw, AppearanceChoice::Paper).unwrap();
        assert!(updated.contains("# user's note"));
        assert!(updated.contains("include = ['target']"));
        assert!(updated.contains("# follow OS"));
        assert!(updated.contains("accent = '#123456'"));
        let config: Config = toml::from_str(&updated).unwrap();
        assert_eq!(config.theme.appearance, AppearanceChoice::Paper);
    }

    #[test]
    fn next_appearance_cycles_through_every_choice_and_wraps() {
        let mut seen = vec![AppearanceChoice::System];
        let mut choice = AppearanceChoice::System.next();
        while choice != AppearanceChoice::System {
            seen.push(choice);
            choice = choice.next();
        }
        assert_eq!(seen, AppearanceChoice::ALL);
    }

    #[test]
    fn next_font_cycles_through_every_choice_and_wraps() {
        let mut seen = vec![FontChoice::System];
        let mut choice = FontChoice::System.next();
        while choice != FontChoice::System {
            seen.push(choice);
            choice = choice.next();
        }
        assert_eq!(seen, FontChoice::ALL);
    }

    #[test]
    fn changing_appearance_creates_missing_theme_table_and_key() {
        for raw in ["", "[theme]\n", "[theme.slate]\naccent = '#123456'\n"] {
            let updated = update_appearance(raw, AppearanceChoice::Nightfall).unwrap();
            let config: Config = toml::from_str(&updated).unwrap();
            assert_eq!(config.theme.appearance, AppearanceChoice::Nightfall);
        }
    }

    #[test]
    fn changing_appearance_rejects_invalid_file() {
        assert!(
            update_appearance("[theme.slate]\naccent = 'red'\n", AppearanceChoice::Paper).is_err()
        );
        assert!(update_appearance("[theme\n", AppearanceChoice::Paper).is_err());
    }

    #[test]
    fn external_link_action_defaults_to_ask_and_round_trips() {
        assert_eq!(
            Config::default().links.external_markdown,
            ExternalLinkAction::Ask
        );
        let raw =
            "# keep\n[theme]\nappearance = 'slate'\n\n[links]\nexternal_markdown = 'ask' # why\n";
        let updated = update_external_link_action(raw, ExternalLinkAction::Document).unwrap();
        assert!(updated.contains("# keep") && updated.contains("# why"));
        let config: Config = toml::from_str(&updated).unwrap();
        assert_eq!(config.links.external_markdown, ExternalLinkAction::Document);
        assert_eq!(config.theme.appearance, AppearanceChoice::Slate);
        let back = update_external_link_action(&updated, ExternalLinkAction::Ask).unwrap();
        let config: Config = toml::from_str(&back).unwrap();
        assert_eq!(config.links.external_markdown, ExternalLinkAction::Ask);
    }

    #[test]
    fn external_link_action_is_added_to_an_empty_file_and_rejects_other_values() {
        let updated = update_external_link_action("", ExternalLinkAction::Folder).unwrap();
        let config: Config = toml::from_str(&updated).unwrap();
        assert_eq!(config.links.external_markdown, ExternalLinkAction::Folder);
        assert!(toml::from_str::<Config>("[links]\nexternal_markdown = 'always'\n").is_err());
        assert!(update_external_link_action("links = 3\n", ExternalLinkAction::Ask).is_err());
    }

    mod properties {
        use super::super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(200))]

            #[test]
            fn config_parsing_never_panics_on_arbitrary_text(raw in "\\PC{0,200}") {
                let _ = toml::from_str::<Config>(&raw);
            }

            #[test]
            fn config_edits_never_panic_and_keep_the_file_valid(
                raw in "(\\[[a-z]{1,8}\\]\n([a-z_]{1,8} = ('[a-z]{0,6}'|[0-9]{1,3}|true)\n){0,3}){0,3}"
            ) {
                let edits = [
                    update_external_link_action(&raw, ExternalLinkAction::Folder),
                    update_font(&raw, FontChoice::Mono),
                    update_minimap(&raw, true),
                ];
                for updated in edits.into_iter().flatten() {
                    prop_assert!(toml::from_str::<Config>(&updated).is_ok(), "{updated}");
                }
            }
        }
    }

    #[test]
    fn show_text_defaults_off_and_saving_keeps_comments() {
        let config = toml::from_str::<Config>("").unwrap();
        assert!(!config.files.show_text);
        assert!(!config.scan_rules().lists(std::path::Path::new("a.txt")));
        let updated = update_show_text("# keep\n[files]\n", true).unwrap();
        assert!(updated.contains("# keep"));
        let config: Config = toml::from_str(&updated).unwrap();
        assert!(config.files.show_text);
        assert!(config.scan_rules().lists(std::path::Path::new("a.txt")));
        assert!(
            update_show_text("", true)
                .unwrap()
                .contains("show_text = true")
        );
    }
}
