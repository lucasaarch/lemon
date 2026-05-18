use std::{cell::RefCell, rc::Rc};

use crate::element::style::Color;

/// Application theme grouped by token families.
///
/// Use [`Theme::default_light`] or [`Theme::default_dark`] as sensible base palettes.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Color tokens used for backgrounds, text, accents, and borders.
    pub colors: ColorTokens,
    /// Typography scale used by widgets and app-level styles.
    pub typography: TypographyTokens,
    /// Spacing scale used for padding, gaps, and margins.
    pub spacing: SpacingTokens,
    /// Corner radius scale for rounded containers and controls.
    pub radius: RadiusTokens,
}

impl Theme {
    /// Returns the default light theme.
    pub fn default_light() -> Self {
        Self {
            colors: ColorTokens {
                background: Color::rgb8(248, 250, 252),
                surface: Color::rgb8(255, 255, 255),
                foreground: Color::rgb8(15, 23, 42),
                foreground_secondary: Color::rgb8(71, 85, 105),
                accent: Color::rgb8(59, 130, 246),
                accent_hover: Color::rgb8(37, 99, 235),
                error: Color::rgb8(220, 38, 38),
                border: Color::rgb8(203, 213, 225),
            },
            typography: TypographyTokens {
                font_size_sm: 12.0,
                font_size_md: 14.0,
                font_size_lg: 18.0,
                font_size_xl: 24.0,
                line_height: 1.5,
            },
            spacing: SpacingTokens {
                xs: 4.0,
                sm: 8.0,
                md: 12.0,
                lg: 16.0,
                xl: 24.0,
            },
            radius: RadiusTokens {
                sm: 4.0,
                md: 8.0,
                lg: 12.0,
            },
        }
    }

    /// Returns the default dark theme.
    pub fn default_dark() -> Self {
        Self {
            colors: ColorTokens {
                background: Color::rgb8(17, 24, 39),
                surface: Color::rgb8(35, 35, 52),
                foreground: Color::rgb8(235, 235, 240),
                foreground_secondary: Color::rgb8(156, 163, 175),
                accent: Color::rgb8(59, 130, 246),
                accent_hover: Color::rgb8(37, 99, 235),
                error: Color::rgb8(248, 113, 113),
                border: Color::rgb8(80, 80, 110),
            },
            typography: TypographyTokens {
                font_size_sm: 12.0,
                font_size_md: 14.0,
                font_size_lg: 18.0,
                font_size_xl: 24.0,
                line_height: 1.5,
            },
            spacing: SpacingTokens {
                xs: 4.0,
                sm: 8.0,
                md: 12.0,
                lg: 16.0,
                xl: 24.0,
            },
            radius: RadiusTokens {
                sm: 4.0,
                md: 8.0,
                lg: 12.0,
            },
        }
    }
}

/// Color token group used by controls and surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorTokens {
    pub background: Color,
    pub surface: Color,
    pub foreground: Color,
    pub foreground_secondary: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub error: Color,
    pub border: Color,
}

/// Typography token group used by text styles.
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTokens {
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub line_height: f32,
}

/// Spacing token group used by layout spacing.
#[derive(Clone, Debug, PartialEq)]
pub struct SpacingTokens {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

/// Radius token group used by rounded corners.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusTokens {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

thread_local! {
    static ACTIVE_THEME: RefCell<Rc<Theme>> = RefCell::new(Rc::new(Theme::default_light()));
}

/// Sets the active theme for the current thread.
pub fn set_active_theme(theme: Theme) {
    ACTIVE_THEME.with(|active| {
        *active.borrow_mut() = Rc::new(theme);
    });
}

/// Returns a clone of the active theme for the current thread.
pub fn current_theme() -> Theme {
    ACTIVE_THEME.with(|active| active.borrow().as_ref().clone())
}

#[cfg(test)]
mod tests {
    use super::{current_theme, set_active_theme, Theme};

    #[test]
    fn active_theme_roundtrip_works() {
        let dark = Theme::default_dark();
        set_active_theme(dark.clone());
        assert_eq!(current_theme(), dark);
    }
}
