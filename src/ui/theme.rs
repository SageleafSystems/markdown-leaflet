// SPDX-License-Identifier: GPL-3.0-or-later
//! A native-feeling macOS look — a unified toolbar, rounded sidebar cards,
//! subtle borders, and a green accent for selection and
//! primary actions — available in both a light and a dark [`Appearance`].
//! Setting `primary`/`success` to the accent green in [`palette`] is what
//! makes iced's *built-in* widget styles (`button::primary`, a focused
//! `text_input`, …) already pick up the accent for free; the style helpers
//! below only exist for the few places that need a look the built-ins
//! don't provide (panels, pills, selectable rows, icon buttons).
use crate::config::{
    AppearanceChoice, Config, GradientConfig, GradientStyle, ParsedPalette, Rgb, ThemeError,
};
use iced::gradient::Linear;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Gradient, Radians};

/// Which palette is active. Detected once at launch from the
/// OS setting (see `app::detect_appearance`); the app does not currently
/// watch for the OS setting changing underneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceKind {
    Slate,
    Harbor,
    Nightfall,
    Linen,
    Paper,
}

#[derive(Clone, Copy, Debug)]
pub struct Appearance {
    pub kind: AppearanceKind,
    pub colors: Colors,
    pub gradient: GradientConfig,
}

impl Appearance {
    pub const SLATE: Self = Self {
        kind: AppearanceKind::Slate,
        colors: SLATE,
        gradient: GradientConfig::DEFAULT,
    };
    pub const HARBOR: Self = Self {
        kind: AppearanceKind::Harbor,
        colors: HARBOR,
        gradient: GradientConfig::DEFAULT,
    };
    pub const NIGHTFALL: Self = Self {
        kind: AppearanceKind::Nightfall,
        colors: NIGHTFALL,
        gradient: GradientConfig::DEFAULT,
    };
    pub const LINEN: Self = Self {
        kind: AppearanceKind::Linen,
        colors: LINEN,
        gradient: GradientConfig::DEFAULT,
    };
    pub const PAPER: Self = Self {
        kind: AppearanceKind::Paper,
        colors: PAPER,
        gradient: GradientConfig::DEFAULT,
    };

    /// Resolves the configured appearance, applying user palette overrides.
    /// Every palette is validated, even those not currently visible.
    pub fn from_config(config: &Config, system: AppearanceKind) -> Result<Self, ThemeError> {
        let kind = match config.theme.appearance {
            AppearanceChoice::System => system,
            AppearanceChoice::Slate => AppearanceKind::Slate,
            AppearanceChoice::Harbor => AppearanceKind::Harbor,
            AppearanceChoice::Nightfall => AppearanceKind::Nightfall,
            AppearanceChoice::Linen => AppearanceKind::Linen,
            AppearanceChoice::Paper => AppearanceKind::Paper,
        };
        let parsed = config.theme.parse()?;
        let (mut active, overrides) = match kind {
            AppearanceKind::Slate => (Self::SLATE, parsed.slate),
            AppearanceKind::Harbor => (Self::HARBOR, parsed.harbor),
            AppearanceKind::Nightfall => (Self::NIGHTFALL, parsed.nightfall),
            AppearanceKind::Linen => (Self::LINEN, parsed.linen),
            AppearanceKind::Paper => (Self::PAPER, parsed.paper),
        };
        active.colors.apply(&overrides);
        active.gradient = config.theme.gradient.sanitized();
        Ok(active)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Colors {
    pub background: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    /// Text color used *on top of* an accent-colored fill. The accent
    /// green sits at ~0.65 relative luminance in both palettes, so a dark
    /// foreground reads far better there than white in either mode.
    pub on_accent: Color,
    pub danger: Color,
    pub warning: Color,
    pub resize: Color,
    pub file_icon: Color,
    pub folder_icon: Color,
    pub utility_icon: Color,
}

impl Colors {
    fn apply(&mut self, overrides: &ParsedPalette) {
        macro_rules! apply {
            ($($name:ident),+ $(,)?) => {
                $(if let Some(Rgb(r, g, b)) = overrides.$name {
                    self.$name = Color::from_rgb8(r, g, b);
                })+
            };
        }
        apply!(
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
    }
}

/// Dark, charcoal-navy, with Finder green for selection — the closest of
/// the five to the app's original look (issue #78).
const SLATE: Colors = Colors {
    background: Color::from_rgb8(0x1a, 0x1b, 0x24),
    surface: Color::from_rgb8(0x1f, 0x20, 0x29),
    surface_alt: Color::from_rgb8(0x26, 0x27, 0x36),
    border: Color::from_rgb8(0x36, 0x37, 0x48),
    text: Color::from_rgb8(0xe9, 0xe9, 0xee),
    text_muted: Color::from_rgb8(0x9a, 0x9a, 0xa8),
    accent: Color::from_rgb8(0x30, 0xd1, 0x58),
    on_accent: Color::from_rgb8(0x12, 0x16, 0x10),
    danger: Color::from_rgb8(0xff, 0x6b, 0x6b),
    warning: Color::from_rgb8(0xef, 0xba, 0x55),
    resize: Color::from_rgb8(0xff, 0x9f, 0x43),
    file_icon: Color::from_rgb8(0x81, 0xb8, 0xeb),
    folder_icon: Color::from_rgb8(0xed, 0xbb, 0x67),
    utility_icon: Color::from_rgb8(0x9f, 0xc4, 0xae),
};

/// Dark teal with a cyan accent and warm amber folder/git icons (issue #78).
const HARBOR: Colors = Colors {
    background: Color::from_rgb8(0x01, 0x29, 0x34),
    surface: Color::from_rgb8(0x0c, 0x35, 0x42),
    surface_alt: Color::from_rgb8(0x12, 0x41, 0x50),
    border: Color::from_rgb8(0x1f, 0x54, 0x65),
    text: Color::from_rgb8(0xf7, 0xec, 0xe0),
    text_muted: Color::from_rgb8(0xb8, 0xab, 0x95),
    accent: Color::from_rgb8(0x2d, 0xd4, 0xbf),
    on_accent: Color::from_rgb8(0x04, 0x30, 0x3a),
    danger: Color::from_rgb8(0xff, 0x6b, 0x6b),
    warning: Color::from_rgb8(0xe0, 0xa5, 0x26),
    resize: Color::from_rgb8(0xff, 0x9f, 0x43),
    file_icon: Color::from_rgb8(0x7e, 0xb6, 0xff),
    folder_icon: Color::from_rgb8(0xe3, 0xb3, 0x41),
    utility_icon: Color::from_rgb8(0x4f, 0xd1, 0xc5),
};

/// Dark slate-blue with a light cyan accent (issue #78).
const NIGHTFALL: Colors = Colors {
    background: Color::from_rgb8(0x1a, 0x1f, 0x2e),
    surface: Color::from_rgb8(0x24, 0x2a, 0x3a),
    surface_alt: Color::from_rgb8(0x2c, 0x33, 0x44),
    border: Color::from_rgb8(0x3c, 0x43, 0x57),
    text: Color::from_rgb8(0xec, 0xef, 0xf4),
    text_muted: Color::from_rgb8(0xa8, 0xb0, 0xc0),
    accent: Color::from_rgb8(0x5f, 0xb8, 0xd9),
    on_accent: Color::from_rgb8(0x1a, 0x1f, 0x2e),
    danger: Color::from_rgb8(0xbf, 0x61, 0x6a),
    warning: Color::from_rgb8(0xeb, 0xcb, 0x8b),
    resize: Color::from_rgb8(0xd0, 0x87, 0x70),
    file_icon: Color::from_rgb8(0x81, 0xa1, 0xc1),
    folder_icon: Color::from_rgb8(0xeb, 0xcb, 0x8b),
    utility_icon: Color::from_rgb8(0xa3, 0xbe, 0x8c),
};

/// Light cream/linen with a deep teal accent (issue #78).
const LINEN: Colors = Colors {
    background: Color::from_rgb8(0xfb, 0xf4, 0xe4),
    surface: Color::from_rgb8(0xf7, 0xef, 0xdd),
    surface_alt: Color::from_rgb8(0xf0, 0xe6, 0xd0),
    border: Color::from_rgb8(0xdd, 0xd0, 0xb0),
    text: Color::from_rgb8(0x33, 0x2b, 0x1e),
    text_muted: Color::from_rgb8(0x7a, 0x6f, 0x58),
    accent: Color::from_rgb8(0x0f, 0x6f, 0x66),
    on_accent: Color::from_rgb8(0xfb, 0xf4, 0xe4),
    danger: Color::from_rgb8(0xb3, 0x26, 0x1e),
    warning: Color::from_rgb8(0x8a, 0x5a, 0x00),
    resize: Color::from_rgb8(0xb5, 0x65, 0x1d),
    file_icon: Color::from_rgb8(0x2b, 0x6c, 0xb0),
    folder_icon: Color::from_rgb8(0xa6, 0x79, 0x0b),
    utility_icon: Color::from_rgb8(0x2f, 0x7a, 0x5c),
};

/// Light, near-white, with a blue accent (issue #78).
const PAPER: Colors = Colors {
    background: Color::from_rgb8(0xfd, 0xfd, 0xfe),
    surface: Color::from_rgb8(0xff, 0xff, 0xff),
    surface_alt: Color::from_rgb8(0xec, 0xef, 0xf5),
    border: Color::from_rgb8(0xd7, 0xdc, 0xe6),
    text: Color::from_rgb8(0x3a, 0x3e, 0x48),
    text_muted: Color::from_rgb8(0x6b, 0x72, 0x80),
    accent: Color::from_rgb8(0x2f, 0x6f, 0xed),
    on_accent: Color::from_rgb8(0xff, 0xff, 0xff),
    danger: Color::from_rgb8(0xd9, 0x2d, 0x20),
    warning: Color::from_rgb8(0xb5, 0x47, 0x08),
    resize: Color::from_rgb8(0xc2, 0x41, 0x0c),
    file_icon: Color::from_rgb8(0x25, 0x63, 0xeb),
    folder_icon: Color::from_rgb8(0xb4, 0x53, 0x09),
    utility_icon: Color::from_rgb8(0x0e, 0x74, 0x90),
};

pub const fn colors(appearance: Appearance) -> Colors {
    appearance.colors
}

/// Whether the theme's window background is dark, so artwork can pick its light-on-dark variant.
pub fn is_dark(appearance: Appearance) -> bool {
    let bg = colors(appearance).background;
    0.2126 * bg.r + 0.7152 * bg.g + 0.0722 * bg.b < 0.5
}

pub fn palette(appearance: Appearance) -> iced::theme::Palette {
    let c = colors(appearance);
    iced::theme::Palette {
        background: c.background,
        text: c.text,
        primary: c.accent,
        success: c.accent,
        warning: c.warning,
        danger: c.danger,
    }
}

pub fn theme(appearance: Appearance) -> iced::Theme {
    let name = match appearance.kind {
        AppearanceKind::Slate => "Slate",
        AppearanceKind::Harbor => "Harbor",
        AppearanceKind::Nightfall => "Nightfall",
        AppearanceKind::Linen => "Linen",
        AppearanceKind::Paper => "Paper",
    };
    iced::Theme::custom(name.to_string(), palette(appearance))
}

/// The gap kept around a floating gutter card is left transparent on
/// purpose: the card now sits over the toolbar, so its margin should show
/// the toolbar's own surface and border coming through underneath — the
/// "floating on top of a continuous topbar" illusion — rather than paint
/// over them with a flat well color. Below the toolbar's height the same
/// transparency just reveals the window's background, as a recessed well
/// did before. The resize handle still disappears into this gap when idle,
/// since it only paints a color while its hover/drag opacity is above zero.
pub fn sidebar_well(_appearance: Appearance) -> container::Style {
    container::Style::default()
}

/// A rounded sidebar card inside a [`sidebar_well`], shared by the file
/// list, outline, and document details.
pub fn sidebar_card(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface)),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: CARD_RADIUS.into(),
        },
        ..container::Style::default()
    }
}

/// Where an accent card's fill fade begins, as a fraction of its length. Above it the card is
/// the flat `surface`, so the file list's rows sit on the same fill as every other panel.
const ACCENT_FADE_START: f32 = 0.6;

/// How far the end of the `fill` style's fade leans toward the accent per unit of opacity.
/// A whole fill needs far less than a 1px ring to be noticed, and a small share keeps every
/// row's contrast unchanged. The ring itself leans by the opacity directly.
const ACCENT_FILL_MIX_PER_OPACITY: f32 = 0.2;

/// Where the ring has fully returned to flat `border` after the start, and where it begins to
/// lean toward the accent again, as fractions along the gradient line.
const ACCENT_RING_FLAT: (f32, f32) = (0.35, 0.65);

/// The card's corner radius.
const CARD_RADIUS: f32 = 10.0;

/// The card's border width, and so the ring's thickness.
const CARD_BORDER: f32 = 1.0;

/// Peak alpha of the accent glow at full strength, and how far it spreads in pixels.
const ACCENT_GLOW_ALPHA: f32 = 0.22;
const ACCENT_GLOW_BLUR: f32 = 5.0;

/// The gradient's angle in iced's terms, where 180° runs straight down; the tilt leans it toward
/// the bottom right.
fn gradient_angle(settings: GradientConfig) -> Radians {
    Radians(std::f32::consts::PI - settings.angle.to_radians())
}

/// An accent card's outer container. In the default [`GradientStyle::Border`] it is a ring of
/// gradient (`border`, leaning toward the accent at both ends) around the card, with a faint
/// accent glow; the card's own fill goes in [`accent_card_inner`]. In [`GradientStyle::Fill`] it is the
/// plain bordered card with a fill that fades toward the accent at one end, and the inner
/// container is transparent.
pub fn accent_card(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    let settings = appearance.gradient;
    match settings.style {
        GradientStyle::Fill => {
            let tint = mix(
                c.surface,
                c.accent,
                settings.opacity * ACCENT_FILL_MIX_PER_OPACITY,
            );
            let gradient = Linear::new(gradient_angle(settings))
                .add_stop(0.0, c.surface)
                .add_stop(ACCENT_FADE_START, c.surface)
                .add_stop(1.0, tint);
            container::Style {
                background: Some(Background::Gradient(Gradient::Linear(gradient))),
                ..sidebar_card(appearance)
            }
        }
        GradientStyle::Border => {
            let tint = mix(c.border, c.accent, settings.opacity);
            let gradient = Linear::new(gradient_angle(settings))
                .add_stop(0.0, tint)
                .add_stop(ACCENT_RING_FLAT.0, c.border)
                .add_stop(ACCENT_RING_FLAT.1, c.border)
                .add_stop(1.0, tint);
            container::Style {
                background: Some(Background::Gradient(Gradient::Linear(gradient))),
                border: Border {
                    radius: CARD_RADIUS.into(),
                    ..Border::default()
                },
                shadow: card_glow(appearance),
                ..container::Style::default()
            }
        }
    }
}

/// The padding between [`accent_card`]'s outer edge and its inner container: the ring's thickness
/// in [`GradientStyle::Border`], none in [`GradientStyle::Fill`].
pub fn accent_card_ring(appearance: Appearance) -> f32 {
    match appearance.gradient.style {
        GradientStyle::Border => CARD_BORDER,
        GradientStyle::Fill => 0.0,
    }
}

/// The inner container of an accent card: the flat `surface` fill inside the ring. Its radius
/// is the outer radius less the ring so the two curves stay concentric.
pub fn accent_card_inner(appearance: Appearance) -> container::Style {
    match appearance.gradient.style {
        GradientStyle::Border => container::Style {
            background: Some(Background::Color(colors(appearance).surface)),
            border: Border {
                radius: (CARD_RADIUS - CARD_BORDER).into(),
                ..Border::default()
            },
            ..container::Style::default()
        },
        GradientStyle::Fill => container::Style::default(),
    }
}

/// A faint accent halo a few pixels wide around the card, scaled by the configured strength.
fn card_glow(appearance: Appearance) -> iced::Shadow {
    iced::Shadow {
        color: Color {
            a: ACCENT_GLOW_ALPHA * appearance.gradient.glow,
            ..colors(appearance).accent
        },
        offset: iced::Vector::ZERO,
        blur_radius: ACCENT_GLOW_BLUR,
    }
}

/// The highlight painted behind a drag-selected run of rendered text, as a
/// translucent accent so the text's own color still reads on top of it —
/// the same idea as the active find match, at lower opacity so it doesn't
/// compete with it when both land on the same text.
pub fn selection_background(appearance: Appearance) -> Color {
    Color {
        a: 0.35,
        ..colors(appearance).accent
    }
}

/// A translucent orange cue fades while a gutter handle is hovered or dragged.
/// Peak alpha of the resize cue: present enough to find, quiet enough not to compete with content.
const RESIZE_CUE_ALPHA: f32 = 0.3;

pub fn resize_color(appearance: Appearance, opacity: f32) -> Color {
    let color = colors(appearance).resize;
    Color {
        a: RESIZE_CUE_ALPHA * opacity,
        ..color
    }
}

pub fn resize_handle(appearance: Appearance, opacity: f32) -> container::Style {
    container::Style {
        background: (opacity > 0.0).then(|| Background::Color(resize_color(appearance, opacity))),
        ..container::Style::default()
    }
}

/// A quiet footer surface, between the window background and the card surface.
pub fn footer_panel(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(Color {
            r: c.background.r * 0.7 + c.surface.r * 0.3,
            g: c.background.g * 0.7 + c.surface.g * 0.3,
            b: c.background.b * 0.7 + c.surface.b * 0.3,
            a: 1.0,
        })),
        ..container::Style::default()
    }
}

pub fn footer_divider(appearance: Appearance) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            a: 0.55,
            ..colors(appearance).border
        })),
        ..container::Style::default()
    }
}

/// The top toolbar, divided from the content below it.
pub fn toolbar_panel(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface)),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Blends `base` toward black by `amount` (0 = unchanged, 1 = black).
/// `base` moved `amount` (0.0–1.0) of the way toward `toward`.
fn mix(base: Color, toward: Color, amount: f32) -> Color {
    let lerp = |a: f32, b: f32| a + (b - a) * amount;
    Color::from_rgb(
        lerp(base.r, toward.r),
        lerp(base.g, toward.g),
        lerp(base.b, toward.b),
    )
}

fn darken(base: Color, amount: f32) -> Color {
    let k = 1.0 - amount;
    Color::from_rgb(base.r * k, base.g * k, base.b * k)
}

fn lighten(base: Color, amount: f32) -> Color {
    mix(base, Color::WHITE, amount)
}

/// A recessed well behind a cluster of toolbar icon buttons: darker than the surface it sits on,
/// with a thin, darker border.
pub fn icon_well(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(Color {
            a: 0.5,
            ..darken(c.surface, 0.12)
        })),
        border: Border {
            color: Color {
                a: 0.5,
                ..darken(c.surface, 0.3)
            },
            width: crate::layout::spacing::WELL_BORDER,
            radius: 999.0.into(),
        },
        ..container::Style::default()
    }
}

/// `detail_group`'s look as a clickable button: a metric card, whose whole area copies its
/// value when pressed, with a faint hover/press fill so the press target reads as clickable.
pub fn card_button(
    appearance: Appearance,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => lighten(c.surface_alt, 0.05),
            button::Status::Pressed => darken(c.surface_alt, 0.05),
            _ => c.surface_alt,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: c.text,
            border: Border {
                color: Color { a: 0.5, ..c.border },
                width: 1.0,
                radius: CARD_RADIUS.into(),
            },
            ..button::Style::default()
        }
    }
}

/// A rounded group of label/value rows in the inspector: a recessed fill one step off the card's
/// surface, with a faint border so it reads in both appearances.
pub fn detail_group(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface_alt)),
        border: Border {
            color: Color { a: 0.5, ..c.border },
            width: 1.0,
            radius: CARD_RADIUS.into(),
        },
        ..container::Style::default()
    }
}

/// The small pill that holds a count, such as the number of headings in the outline.
pub fn count_badge(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface_alt)),
        border: Border {
            color: Color { a: 0.5, ..c.border },
            width: 1.0,
            radius: 999.0.into(),
        },
        ..container::Style::default()
    }
}

/// The compact Files strip under the toolbar: a recessed fill with no border
/// of its own, since a divider is drawn beneath it.
pub fn mini_nav_panel(appearance: Appearance) -> container::Style {
    container::Style {
        background: Some(Background::Color(colors(appearance).surface_alt)),
        ..container::Style::default()
    }
}

pub fn dialog_panel(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface)),
        text_color: Some(c.text),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: iced::Shadow {
            color: Color {
                a: 0.35,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 24.0,
        },
        ..container::Style::default()
    }
}

/// A nested, slightly recessed panel inside a dialog for scrollable or
/// grouped content (the Shortcuts list, a file's path) — the lighter inset
/// box dialogs like it use to separate detail from the lede.
pub fn dialog_well(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface_alt)),
        border: Border {
            color: Color { a: 0.5, ..c.border },
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

/// A dialog's pill-shaped default action: solid accent fill, the same
/// treatment a native dialog gives its default button.
pub fn dialog_button_primary(
    appearance: Appearance,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => lighten(c.accent, 0.08),
            button::Status::Pressed => darken(c.accent, 0.08),
            button::Status::Active | button::Status::Disabled => c.accent,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: c.on_accent,
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// A dialog's pill-shaped secondary action (Cancel, an alternate choice):
/// a neutral fill a step off the dialog's own surface.
pub fn dialog_button_secondary(
    appearance: Appearance,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => lighten(c.surface_alt, 0.06),
            button::Status::Pressed => darken(c.surface_alt, 0.06),
            button::Status::Active | button::Status::Disabled => c.surface_alt,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: c.text,
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// A dialog's pill-shaped destructive action (Move to Trash): the
/// secondary fill with danger-colored text, rather than a loud red button,
/// so it still reads as a choice and not an alarm.
pub fn dialog_button_danger(
    appearance: Appearance,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => lighten(c.surface_alt, 0.06),
            button::Status::Pressed => darken(c.surface_alt, 0.06),
            button::Status::Active | button::Status::Disabled => c.surface_alt,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: c.danger,
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// Compact in-window menu titles, separate from the width of their popups.
#[cfg(not(target_os = "macos"))]
pub fn menu_bar_button(
    appearance: Appearance,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(Background::Color(c.surface_alt))
            .or_else(|| active.then_some(Background::Color(c.surface_alt))),
        text_color: c.text,
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Small floating label shown when hovering a toolbar control.
pub fn tooltip_panel(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        text_color: Some(c.text),
        background: Some(Background::Color(c.surface_alt)),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow {
            color: Color {
                a: 0.22,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..container::Style::default()
    }
}

pub fn menu_popup(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface)),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            color: Color {
                a: 0.22,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

pub fn menu_item(appearance: Appearance) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| {
        let highlighted = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: highlighted.then_some(Background::Color(c.surface_alt)),
            text_color: c.text,
            border: Border {
                radius: 5.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// A translucent, rounded banner for the error message.
pub fn error_banner(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        text_color: Some(c.danger),
        background: Some(Background::Color(Color {
            a: 0.12,
            ..c.danger
        })),
        border: Border {
            color: c.danger,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

/// A toast (issue #43): a solid, floating card, darker than the surrounding
/// surface and opaque regardless of appearance, so it reads above whatever
/// content it floats over rather than tinting it like the error banner does.
pub fn toast_panel(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(darken(c.surface, 0.1))),
        text_color: Some(c.text),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color {
                a: 0.35,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

/// A fully-rounded "pill" wrapper for the search box — the `text_input`
/// inside is styled transparent (via [`search_pill_input`]) so this
/// container's background/border reads as a single pill shape.
pub fn search_pill(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface_alt)),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 999.0.into(),
        },
        ..container::Style::default()
    }
}

/// The clear (×) button at the right end of the search pill: bare at rest, a round `border`-colored
/// fill on hover and press, since the pill itself is already `surface_alt`.
pub fn search_clear_button(
    appearance: Appearance,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => Some(Background::Color(c.border)),
            _ => None,
        },
        text_color: c.text_muted,
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// The `text_input` nested inside [`search_pill`] — no background/border
/// of its own, since the wrapping container already draws the pill.
pub fn search_pill_input(
    appearance: Appearance,
) -> impl Fn(&iced::Theme, iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    let c = colors(appearance);
    move |_theme, _status| iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: c.text_muted,
        placeholder: c.text_muted,
        value: c.text,
        selection: c.accent,
    }
}

/// Corner radius of a file-list row's highlight; the new-item field matches it.
pub const ROW_RADIUS: f32 = 6.0;

/// The nav's name field for a new or renamed item: a recessed box with a file row's corners.
pub fn name_field(appearance: Appearance) -> container::Style {
    let c = colors(appearance);
    container::Style {
        background: Some(Background::Color(c.surface_alt)),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: ROW_RADIUS.into(),
        },
        ..container::Style::default()
    }
}

/// A selectable row in a flat list (file list, outline). Selected rows get
/// a solid accent-green pill; everything else is transparent with a faint
/// hover highlight.
pub fn sidebar_row(
    appearance: Appearance,
    selected: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    move |_theme, status| {
        // Pressed keeps a fill so the highlight never drops out between hover and selection.
        let background = if selected {
            c.accent
        } else if matches!(status, button::Status::Pressed) {
            c.border
        } else if matches!(status, button::Status::Hovered) {
            c.surface_alt
        } else {
            Color::TRANSPARENT
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: if selected { c.on_accent } else { c.text },
            border: Border {
                radius: ROW_RADIUS.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// A [`sidebar_row`]-style tab in a segmented control: the edge facing the capsule well is
/// rounded to `end_radius` (half the row's height makes it follow the well's own curve, so the
/// fill reads as continuous with the pill rather than a rounded rectangle dropped inside it);
/// the edge facing the next tab keeps the tighter [`ROW_RADIUS`], so the two tabs still read as
/// separate segments.
pub fn segmented_tab(
    appearance: Appearance,
    selected: bool,
    ends: Ends,
    end_radius: f32,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    let (left, right) = match ends {
        Ends::None => (ROW_RADIUS, ROW_RADIUS),
        Ends::Left => (end_radius, ROW_RADIUS),
        Ends::Right => (ROW_RADIUS, end_radius),
        Ends::Both => (end_radius, end_radius),
    };
    move |_theme, status| {
        // Pressed keeps a fill so the highlight never drops out between hover and selection.
        let background = if selected {
            c.accent
        } else if matches!(status, button::Status::Pressed) {
            c.border
        } else if matches!(status, button::Status::Hovered) {
            c.surface_alt
        } else {
            Color::TRANSPARENT
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: if selected { c.on_accent } else { c.text },
            border: Border {
                radius: iced::border::Radius {
                    top_left: left,
                    bottom_left: left,
                    top_right: right,
                    bottom_right: right,
                },
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// Which outer corners of an icon button are fully rounded, so the end buttons of a well follow
/// its capsule shape. Inner corners keep the usual [`ICON_BUTTON_RADIUS`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ends {
    None,
    Left,
    Right,
    Both,
}

const ICON_BUTTON_RADIUS: f32 = 6.0;

/// A borderless, square icon button for the toolbar (sidebar toggle,
/// inspector toggle, share). Picks up a faint fill on hover/press so it
/// reads as interactive without looking like a bordered form control.
pub fn icon_button(
    appearance: Appearance,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    icon_button_ends(appearance, active, Ends::None, 0.0)
}

/// [`icon_button`] whose `ends` corners are rounded by `end_radius` (half the button's height
/// makes them follow a capsule well).
pub fn icon_button_ends(
    appearance: Appearance,
    active: bool,
    ends: Ends,
    end_radius: f32,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let c = colors(appearance);
    let (left, right) = match ends {
        Ends::None => (ICON_BUTTON_RADIUS, ICON_BUTTON_RADIUS),
        Ends::Left => (end_radius, ICON_BUTTON_RADIUS),
        Ends::Right => (ICON_BUTTON_RADIUS, end_radius),
        Ends::Both => (end_radius, end_radius),
    };
    move |_theme, status| {
        let background = if active {
            Some(Background::Color(Color {
                a: 0.16,
                ..c.accent
            }))
        } else {
            match status {
                button::Status::Hovered => Some(Background::Color(c.surface_alt)),
                button::Status::Pressed => Some(Background::Color(c.border)),
                _ => None,
            }
        };

        button::Style {
            background,
            text_color: if active { c.accent } else { c.text },
            border: Border {
                radius: iced::border::Radius {
                    top_left: left,
                    bottom_left: left,
                    top_right: right,
                    bottom_right: right,
                },
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

#[cfg(test)]
mod is_dark_tests {
    use super::*;

    #[test]
    fn only_the_light_themes_are_light() {
        assert!(!is_dark(Appearance::LINEN));
        assert!(!is_dark(Appearance::PAPER));
        assert!(is_dark(Appearance::SLATE));
        assert!(is_dark(Appearance::HARBOR));
        assert!(is_dark(Appearance::NIGHTFALL));
    }
}

#[cfg(test)]
mod accent_card_tests {
    use super::*;

    const KINDS: [AppearanceKind; 5] = [
        AppearanceKind::Slate,
        AppearanceKind::Harbor,
        AppearanceKind::Nightfall,
        AppearanceKind::Linen,
        AppearanceKind::Paper,
    ];

    fn appearance(kind: AppearanceKind, gradient: GradientConfig) -> Appearance {
        let mut a = Appearance::from_config(&Config::default(), kind).unwrap();
        a.gradient = gradient;
        a
    }

    fn fill() -> GradientConfig {
        GradientConfig {
            style: GradientStyle::Fill,
            ..GradientConfig::DEFAULT
        }
    }

    fn linear(style: container::Style) -> Linear {
        let Some(Background::Gradient(Gradient::Linear(g))) = style.background else {
            panic!("expected a linear gradient background");
        };
        g
    }

    #[test]
    fn border_is_the_default_style_at_45_degrees() {
        assert_eq!(GradientConfig::default().style, GradientStyle::Border);
        assert_eq!(GradientConfig::default().angle, 45.0);
    }

    #[test]
    fn border_style_ring_fades_to_accent_at_both_ends() {
        for kind in KINDS {
            let a = appearance(kind, GradientConfig::DEFAULT);
            let c = colors(a);
            let g = linear(accent_card(a));
            let stops: Vec<_> = g.stops.iter().flatten().collect();
            assert_eq!(stops.len(), 4, "{kind:?}");
            let (first, last) = (stops[0], stops[3]);
            assert_eq!((first.offset, last.offset), (0.0, 1.0));
            assert_eq!(first.color, last.color, "{kind:?}: ends match");
            assert_ne!(first.color, c.border, "{kind:?}: ends are tinted");
            assert_eq!(stops[1].color, c.border);
            assert_eq!(stops[2].color, c.border);
            // The tint moves the border toward the accent, never past it.
            for (t, b, ac) in [
                (first.color.r, c.border.r, c.accent.r),
                (first.color.g, c.border.g, c.accent.g),
                (first.color.b, c.border.b, c.accent.b),
            ] {
                assert!(t >= b.min(ac) - 1e-4 && t <= b.max(ac) + 1e-4, "{kind:?}");
            }
        }
    }

    #[test]
    fn border_style_inner_card_is_flat_surface_with_a_concentric_radius() {
        let a = appearance(AppearanceKind::Slate, GradientConfig::DEFAULT);
        let inner = accent_card_inner(a);
        assert_eq!(inner.background, Some(Background::Color(colors(a).surface)));
        assert_eq!(accent_card_ring(a), CARD_BORDER);
        assert_eq!(
            inner.border.radius,
            (accent_card(a).border.radius.top_left - accent_card_ring(a)).into()
        );
        assert_eq!(inner.border.width, 0.0);
    }

    #[test]
    fn glow_is_a_faint_accent_halo_scaled_by_strength() {
        let a = appearance(AppearanceKind::Slate, GradientConfig::DEFAULT);
        let glow = accent_card(a).shadow;
        assert_eq!(glow.offset, iced::Vector::ZERO);
        assert!(
            glow.blur_radius > 0.0 && glow.blur_radius <= 8.0,
            "a few pixels"
        );
        assert_eq!(glow.color.r, colors(a).accent.r);
        assert!(glow.color.a > 0.0 && glow.color.a <= 0.3, "subtle");

        let off = appearance(
            AppearanceKind::Slate,
            GradientConfig {
                glow: 0.0,
                ..GradientConfig::DEFAULT
            },
        );
        assert_eq!(accent_card(off).shadow.color.a, 0.0);
    }

    #[test]
    fn fill_style_fades_from_flat_surface_to_accent_tint() {
        for kind in KINDS {
            let a = appearance(kind, fill());
            let c = colors(a);
            let g = linear(accent_card(a));
            let stops: Vec<_> = g.stops.iter().flatten().collect();
            assert_eq!(stops.first().unwrap().color, c.surface, "{kind:?}");
            assert_eq!(stops.first().unwrap().offset, 0.0);
            assert_eq!(stops.last().unwrap().offset, 1.0);
            let bottom = stops.last().unwrap().color;
            assert_ne!(bottom, c.surface, "{kind:?}: end should be tinted");
            // Subtle: every channel stays within the mix fraction of the surface.
            for (b, s) in [
                (bottom.r, c.surface.r),
                (bottom.g, c.surface.g),
                (bottom.b, c.surface.b),
            ] {
                assert!(
                    (b - s).abs()
                        <= GradientConfig::DEFAULT.opacity * ACCENT_FILL_MIX_PER_OPACITY + 1e-4,
                    "{kind:?}: too strong"
                );
            }
        }
    }

    #[test]
    fn fill_style_keeps_the_shared_card_border_and_no_ring() {
        let a = appearance(AppearanceKind::Slate, fill());
        assert_eq!(accent_card(a).border, sidebar_card(a).border);
        assert_eq!(accent_card(a).shadow, iced::Shadow::default());
        assert_eq!(accent_card_ring(a), 0.0);
        assert_eq!(accent_card_inner(a).background, None);
    }

    #[test]
    fn angle_tilts_from_straight_down() {
        let straight = GradientConfig {
            angle: 0.0,
            ..GradientConfig::DEFAULT
        };
        assert!((gradient_angle(straight).0 - std::f32::consts::PI).abs() < 1e-6);
        let tilted = gradient_angle(GradientConfig::DEFAULT).0;
        assert!((tilted - std::f32::consts::PI * 0.75).abs() < 1e-6);
        let a = appearance(AppearanceKind::Slate, GradientConfig::DEFAULT);
        assert_eq!(linear(accent_card(a)).angle.0, tilted);
    }

    #[test]
    fn opacity_scales_how_far_the_ring_leans_toward_the_accent() {
        let ends = |opacity: f32| {
            let a = appearance(
                AppearanceKind::Slate,
                GradientConfig {
                    opacity,
                    ..GradientConfig::DEFAULT
                },
            );
            linear(accent_card(a)).stops[0].unwrap().color
        };
        let c = colors(appearance(AppearanceKind::Slate, GradientConfig::DEFAULT));
        assert_eq!(ends(0.0), c.border);
        assert_eq!(
            ends(1.0),
            Color::from_rgb(c.accent.r, c.accent.g, c.accent.b)
        );
        let (low, high) = (ends(0.2), ends(0.8));
        assert!((low.g - c.border.g).abs() < (high.g - c.border.g).abs());
    }

    #[test]
    fn the_default_is_subtle() {
        const {
            assert!(GradientConfig::DEFAULT.opacity <= 0.4);
            assert!(GradientConfig::DEFAULT.glow <= 0.5);
        }
    }

    #[test]
    fn sanitized_clamps_the_unit_values_and_replaces_a_non_finite_angle() {
        let wild = GradientConfig {
            style: GradientStyle::Fill,
            angle: f32::NAN,
            opacity: -3.0,
            glow: 9.0,
        };
        let s = wild.sanitized();
        assert_eq!(s.angle, GradientConfig::DEFAULT.angle);
        assert_eq!(s.opacity, 0.0);
        assert_eq!(s.glow, 1.0);
        assert_eq!(s.style, GradientStyle::Fill);
        let nan = GradientConfig {
            opacity: f32::NAN,
            ..GradientConfig::DEFAULT
        };
        assert_eq!(nan.sanitized().opacity, GradientConfig::DEFAULT.opacity);
    }

    #[test]
    fn gradient_table_parses_from_config() {
        let config: Config = toml::from_str(
            "[theme.gradient]\nstyle = \"fill\"\nangle = 20.0\nopacity = 0.6\nglow = 0.5\n",
        )
        .unwrap();
        let a = Appearance::from_config(&config, AppearanceKind::Slate).unwrap();
        assert_eq!(
            a.gradient,
            GradientConfig {
                style: GradientStyle::Fill,
                angle: 20.0,
                opacity: 0.6,
                glow: 0.5
            }
        );
        let default: Config = toml::from_str("").unwrap();
        let a = Appearance::from_config(&default, AppearanceKind::Slate).unwrap();
        assert_eq!(a.gradient, GradientConfig::DEFAULT);
    }

    #[test]
    fn mix_interpolates_channels() {
        let m = mix(Color::BLACK, Color::WHITE, 0.25);
        assert!((m.r - 0.25).abs() < 1e-6 && (m.g - 0.25).abs() < 1e-6);
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    fn load(raw: &str) -> Config {
        toml::from_str(raw).unwrap()
    }

    #[test]
    fn partial_palette_overrides_active_color_only() {
        let config = load("[theme]\nappearance = 'paper'\n[theme.paper]\nfile_icon = '#123456'\n");
        let active = Appearance::from_config(&config, AppearanceKind::Slate).unwrap();
        assert_eq!(active.kind, AppearanceKind::Paper);
        assert_eq!(active.colors.file_icon, Color::from_rgb8(0x12, 0x34, 0x56));
        assert_eq!(active.colors.background, PAPER.background);
    }

    #[test]
    fn nightfall_and_harbor_choices_load_their_palettes_and_overrides() {
        let config =
            load("[theme]\nappearance = 'nightfall'\n[theme.nightfall]\naccent = '#123456'\n");
        let nightfall = Appearance::from_config(&config, AppearanceKind::Paper).unwrap();
        assert_eq!(nightfall.kind, AppearanceKind::Nightfall);
        assert_eq!(nightfall.colors.accent, Color::from_rgb8(0x12, 0x34, 0x56));

        let config = load("[theme]\nappearance = 'harbor'\n");
        let harbor = Appearance::from_config(&config, AppearanceKind::Slate).unwrap();
        assert_eq!(harbor.kind, AppearanceKind::Harbor);
        assert_eq!(harbor.colors.background, HARBOR.background);
    }

    #[test]
    fn invalid_inactive_palette_is_reported() {
        let config = load("[theme]\nappearance = 'paper'\n[theme.slate]\naccent = 'red'\n");
        assert!(
            Appearance::from_config(&config, AppearanceKind::Paper)
                .unwrap_err()
                .to_string()
                .contains("theme.slate.accent")
        );
    }
}
