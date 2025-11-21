use iced::widget::{button, container, text::Style as TextStyle};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// Color Constants - Deep Slate Theme
pub const APP_BACKGROUND: Color = color_from_rgb8(0x02, 0x06, 0x17); // Slate 950
pub const SURFACE_PRIMARY: Color = color_from_rgb8(0x0F, 0x17, 0x2A); // Slate 900
pub const SURFACE_SECONDARY: Color = color_from_rgb8(0x1E, 0x29, 0x3B); // Slate 800
pub const SURFACE_HOVER: Color = color_from_rgb8(0x33, 0x41, 0x55); // Slate 700

pub const ACCENT_PRIMARY: Color = color_from_rgb8(0x63, 0x66, 0xF1); // Indigo 500
pub const ACCENT_HOVER: Color = color_from_rgb8(0x4F, 0x46, 0xE5); // Indigo 600

pub const SUCCESS_ACCENT: Color = color_from_rgb8(0x10, 0xB9, 0x81); // Emerald 500
pub const WARNING_ACCENT: Color = color_from_rgb8(0xF5, 0x9E, 0x0B); // Amber 500
pub const DANGER_ACCENT: Color = color_from_rgb8(0xEF, 0x44, 0x44); // Red 500

pub const TEXT_PRIMARY: Color = color_from_rgb8(0xF8, 0xFA, 0xFC); // Slate 50
pub const TEXT_MUTED: Color = color_from_rgb8(0x94, 0xA3, 0xB8); // Slate 400

// Status Backgrounds (Subtle/Pastel for banners)
pub const STATUS_BG_INFO: Color = color_from_rgb8(0x1E, 0x29, 0x3B); // Slate 800 (Darker for dark mode)
pub const STATUS_BG_SUCCESS: Color = color_from_rgb8(0x06, 0x4E, 0x3B); // Emerald 900
pub const STATUS_BG_ERROR: Color = color_from_rgb8(0x45, 0x0A, 0x0A); // Red 900

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
        background: Some(Background::Color(SURFACE_PRIMARY)), // Darker background for image container
        border: Border::default().rounded(16),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

pub fn surface_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_PRIMARY)),
        border: Border {
            color: SURFACE_SECONDARY,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.2,
            },
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

pub fn banner_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        border: Border::default().rounded(8),
        text_color: Some(TEXT_PRIMARY),
        ..container::Style::default()
    }
}

pub fn badge_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color.clone())),
        border: Border::default().rounded(6),
        text_color: Some(Color::WHITE), // Always white text for badges
        ..container::Style::default()
    }
}



// Custom Button Styles
pub fn active_success_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let mut style = button::success(theme, status);
    if active {
        style.background = Some(Background::Color(SUCCESS_ACCENT));
        style.text_color = Color::WHITE;
        style.border = Border::default().rounded(8);
    } else {
        // Inactive/Outline look
        style.background = Some(Background::Color(Color::TRANSPARENT));
        style.text_color = SUCCESS_ACCENT;
        style.border = Border {
            color: SUCCESS_ACCENT,
            width: 1.0,
            radius: 8.0.into(),
        };
    }
    
    // Hover state for inactive
    if !active && matches!(status, button::Status::Hovered) {
         style.background = Some(Background::Color(color_from_rgb8(0x06, 0x4E, 0x3B))); // Dark Green
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
        style.background = Some(Background::Color(DANGER_ACCENT));
        style.text_color = Color::WHITE;
        style.border = Border::default().rounded(8);
    } else {
        // Inactive/Outline look
        style.background = Some(Background::Color(Color::TRANSPARENT));
        style.text_color = DANGER_ACCENT;
        style.border = Border {
            color: DANGER_ACCENT,
            width: 1.0,
            radius: 8.0.into(),
        };
    }
    
    // Hover state for inactive
    if !active && matches!(status, button::Status::Hovered) {
         style.background = Some(Background::Color(color_from_rgb8(0x45, 0x0A, 0x0A))); // Dark Red
    }

    style
}

pub fn primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.background = Some(Background::Color(ACCENT_PRIMARY));
    style.border = Border::default().rounded(8);
    
    if matches!(status, button::Status::Hovered) {
        style.background = Some(Background::Color(ACCENT_HOVER));
    }
    
    style
}

pub fn secondary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::secondary(theme, status);
    style.background = Some(Background::Color(SURFACE_SECONDARY));
    style.text_color = TEXT_PRIMARY;
    style.border = Border::default().rounded(8);

    if matches!(status, button::Status::Hovered) {
        style.background = Some(Background::Color(SURFACE_HOVER));
    }

    style
}

