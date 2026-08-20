use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Theme};

// Color tokens. A warm brass/amber accent against a cool charcoal
// background — evoking the archive's "vault" — instead of the blue/purple/teal
// most iced apps default to.
pub const BG: Color = Color::from_rgb(0x15 as f32 / 255.0, 0x17 as f32 / 255.0, 0x1C as f32 / 255.0);
pub const SURFACE: Color = Color::from_rgb(0x1B as f32 / 255.0, 0x1E as f32 / 255.0, 0x25 as f32 / 255.0);
pub const SURFACE_ELEVATED: Color = Color::from_rgb(0x22 as f32 / 255.0, 0x26 as f32 / 255.0, 0x2F as f32 / 255.0);
pub const SURFACE_HOVER: Color = Color::from_rgb(0x28 as f32 / 255.0, 0x2D as f32 / 255.0, 0x38 as f32 / 255.0);
pub const BORDER: Color = Color::from_rgb(0x2C as f32 / 255.0, 0x31 as f32 / 255.0, 0x3C as f32 / 255.0);
pub const TEXT_PRIMARY: Color = Color::from_rgb(0xE8 as f32 / 255.0, 0xEA as f32 / 255.0, 0xED as f32 / 255.0);
pub const TEXT_MUTED: Color = Color::from_rgb(0x8B as f32 / 255.0, 0x92 as f32 / 255.0, 0xA3 as f32 / 255.0);
pub const ACCENT: Color = Color::from_rgb(0xE8 as f32 / 255.0, 0xA3 as f32 / 255.0, 0x3D as f32 / 255.0);
pub const ACCENT_DIM: Color = Color::from_rgb(0xC4 as f32 / 255.0, 0x86 as f32 / 255.0, 0x2E as f32 / 255.0);
pub const DANGER: Color = Color::from_rgb(0xD9 as f32 / 255.0, 0x53 as f32 / 255.0, 0x4F as f32 / 255.0);

pub const ACCENT_MUTED_BG: Color = Color {
    r: ACCENT.r,
    g: ACCENT.g,
    b: ACCENT.b,
    a: 0.16,
};

// Spacing scale (px), used consistently instead of ad-hoc numbers.
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;

pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 8.0;
pub const RADIUS_PILL: f32 = 999.0;

/**
 * Builds the custom dark theme (background, text, accent) that every
 * widget's style closure builds on top of.
 *
 * @return iced::Theme the application's palette.
 */
pub fn theme() -> Theme {
    Theme::custom(
        "tag-vfs".to_string(),
        iced::theme::Palette {
            background: BG,
            text: TEXT_PRIMARY,
            primary: ACCENT,
            success: ACCENT,
            warning: ACCENT,
            danger: DANGER,
        },
    )
}

pub fn header(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        ..Default::default()
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            radius: RADIUS_MD.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn preview_frame(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_ELEVATED)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        ..Default::default()
    }
}

/**
 * Style for a sidebar tag entry: a rounded pill, tinted with the accent
 * color and no border when selected, otherwise transparent with a subtle
 * hover state.
 *
 * @param selected whether this entry is the active tag filter.
 * @return a style closure for `button::Style`.
 */
pub fn nav_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            ACCENT_MUTED_BG
        } else {
            match status {
                button::Status::Hovered => SURFACE_ELEVATED,
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: if selected { ACCENT } else { TEXT_PRIMARY },
            border: Border {
                radius: RADIUS_SM.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/**
 * Style for a file-list row rendered as a button: a card with an elevated
 * background, and an accent-tinted background plus left border when the
 * file is the current selection.
 *
 * @param selected whether this row is the selected file.
 * @return a style closure for `button::Style`.
 */
pub fn file_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            ACCENT_MUTED_BG
        } else {
            match status {
                button::Status::Hovered => SURFACE_HOVER,
                _ => SURFACE_ELEVATED,
            }
        };
        let border_color = if selected { ACCENT } else { Color::TRANSPARENT };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: border_color,
                width: 2.0,
                radius: RADIUS_SM.into(),
            },
            ..Default::default()
        }
    }
}

pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => ACCENT,
        button::Status::Pressed => ACCENT_DIM,
        button::Status::Disabled => SURFACE_ELEVATED,
        button::Status::Active => ACCENT_DIM,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: BG,
        border: Border {
            radius: RADIUS_SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (SURFACE_ELEVATED, TEXT_PRIMARY),
        _ => (Color::TRANSPARENT, TEXT_MUTED),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..Default::default()
    }
}

/**
 * Style for a destructive action button (e.g. removing a file): outlined
 * in danger-red, filled solid on hover.
 */
pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (DANGER, BG),
        _ => (Color::TRANSPARENT, DANGER),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: DANGER,
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..Default::default()
    }
}

/**
 * Style for the small "remove" glyph inside a tag chip: transparent until
 * hovered, then tinted danger-red.
 */
pub fn chip_remove(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (Color { a: 0.18, ..DANGER }, DANGER),
        _ => (Color::TRANSPARENT, TEXT_MUTED),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            radius: RADIUS_PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/**
 * Style for a tag chip's pill-shaped background.
 */
pub fn chip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_ELEVATED)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: RADIUS_PILL.into(),
        },
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        _ => BORDER,
    };
    text_input::Style {
        background: Background::Color(SURFACE_ELEVATED),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        icon: TEXT_MUTED,
        placeholder: TEXT_MUTED,
        value: TEXT_PRIMARY,
        selection: ACCENT_MUTED_BG,
    }
}
