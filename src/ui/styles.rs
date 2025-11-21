use iced::widget::{button, container, text::Style as TextStyle};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// Color Constants
pub const STATUS_BG_INFO: Color = color_from_rgb8(0xEE, 0xE8, 0xFF);
pub const STATUS_BG_SUCCESS: Color = color_from_rgb8(0xE2, 0xF7, 0xD3);
pub const STATUS_BG_ERROR: Color = color_from_rgb8(0xFF, 0xDF, 0xDC);
pub const APP_BACKGROUND: Color = color_from_rgb8(0x0D, 0x13, 0x24);
pub const SURFACE_PRIMARY: Color = color_from_rgb8(0x1C, 0x23, 0x38);
pub const SURFACE_SECONDARY: Color = color_from_rgb8(0x24, 0x2E, 0x45);
pub const ACCENT_PRIMARY: Color = color_from_rgb8(0x64, 0xA9, 0xFF);
pub const SUCCESS_ACCENT: Color = color_from_rgb8(0x34, 0xD3, 0x89);
pub const WARNING_ACCENT: Color = color_from_rgb8(0xFF, 0xC2, 0x6A);
pub const DANGER_ACCENT: Color = color_from_rgb8(0xFF, 0x7D, 0x7D);
pub const TEXT_PRIMARY: Color = color_from_rgb8(0xF4, 0xF5, 0xF7);
pub const TEXT_MUTED: Color = color_from_rgb8(0xA0, 0xA8, 0xC0);

pub const fn color_from_rgb8(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub fn text_color(color: Color) -> impl Fn(&Theme) -> TextStyle {
    move |_| TextStyle {
        color: Some(color),
        ..TextStyle::default()
    }
}

pub fn background_panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(APP_BACKGROUND)),
        ..container::Style::default()
    }
}

pub fn image_frame_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_SECONDARY)),
        border: Border::default().rounded(28),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.35,
            },
            offset: Vector::new(0.0, 16.0),
            blur_radius: 32.0,
        },
        ..container::Style::default()
    }
}

pub fn surface_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_PRIMARY)),
        border: Border::default().rounded(20),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.25,
            },
            offset: Vector::new(0.0, 14.0),
            blur_radius: 28.0,
        },
        ..container::Style::default()
    }
}

pub fn banner_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        border: Border::default().rounded(12),
        ..container::Style::default()
    }
}

pub fn badge_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        border: Border::default().rounded(30),
        ..container::Style::default()
    }
}

pub fn darken(color: Color, amount: f32) -> Color {
    let factor = 1.0 - amount;
    Color {
        r: (color.r * factor).clamp(0.0, 1.0),
        g: (color.g * factor).clamp(0.0, 1.0),
        b: (color.b * factor).clamp(0.0, 1.0),
        a: color.a,
    }
}

pub fn active_success_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let mut style = button::success(theme, status);
    if active {
        style.background = style
            .background
            .map(|bg| match bg {
                Background::Color(c) => Background::Color(darken(c, 0.5)),
                other => other,
            });
    }
    style
}

pub fn active_danger_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let mut style = button::danger(theme, status);
    if active {
        style.background = style
            .background
            .map(|bg| match bg {
                Background::Color(c) => Background::Color(darken(c, 0.5)),
                other => other,
            });
    }
    style
}
