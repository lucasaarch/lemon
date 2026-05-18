use std::{cell::RefCell, rc::Rc};

use crate::element::style::Color;

/// Application theme grouped by token families.
///
/// Use [`Theme::default_light`] or [`Theme::default_dark`] as sensible base palettes.
///
/// [`Default`] and [`run`](crate::platform::run) use [`Theme::default_dark`] so new apps get light
/// foreground text on the default near-black window clear color.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Color tokens used for backgrounds, text, accents, and borders.
    pub colors: ColorTokens,
    /// Widget chrome color tokens for paint-layer decorations (scrollbar, caret, focus ring).
    pub chrome: WidgetChromeTokens,
    /// Typography scale used by widgets and app-level styles.
    pub typography: TypographyTokens,
    /// Spacing scale used for padding, gaps, and margins.
    pub spacing: SpacingTokens,
    /// Corner radius scale for rounded containers and controls.
    pub radius: RadiusTokens,
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}

impl Theme {
    /// Returns the default light theme.
    pub fn default_light() -> Self {
        Self {
            colors: ColorTokens {
                background: Color::rgb8(248, 250, 252),
                surface: Color::rgb8(255, 255, 255),
                surface_hover: Color::rgb8(241, 245, 249),
                surface_pressed: Color::rgb8(226, 232, 240),
                foreground: Color::rgb8(15, 23, 42),
                foreground_secondary: Color::rgb8(71, 85, 105),
                foreground_disabled: Color::rgb8(148, 163, 184),
                on_accent: Color::rgb8(255, 255, 255),
                accent: Color::rgb8(59, 130, 246),
                accent_hover: Color::rgb8(37, 99, 235),
                accent_pressed: Color::rgb8(29, 78, 216),
                error: Color::rgb8(220, 38, 38),
                border: Color::rgb8(203, 213, 225),
            },
            chrome: WidgetChromeTokens {
                scrollbar_track: Color::rgb8(229, 231, 235),
                scrollbar_thumb: Color::rgb8(156, 163, 175),
                caret: Color::rgb8(15, 23, 42),
                focus_ring: Color::rgb8(59, 130, 246),
            },
            typography: TypographyTokens {
                font_size_sm: 12.0,
                font_size_md: 14.0,
                font_size_lg: 18.0,
                font_size_xl: 24.0,
                font_family: "system-ui".to_string(),
                line_height: 1.5,
                letter_spacing: 0.0,
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
                background: Color::rgb8(0, 0, 0),
                surface: Color::rgb8(35, 35, 52),
                surface_hover: Color::rgb8(45, 45, 66),
                surface_pressed: Color::rgb8(55, 55, 78),
                foreground: Color::rgb8(255, 255, 255),
                foreground_secondary: Color::rgb8(156, 163, 175),
                foreground_disabled: Color::rgb8(107, 114, 128),
                on_accent: Color::rgb8(255, 255, 255),
                accent: Color::rgb8(59, 130, 246),
                accent_hover: Color::rgb8(37, 99, 235),
                accent_pressed: Color::rgb8(29, 78, 216),
                error: Color::rgb8(248, 113, 113),
                border: Color::rgb8(80, 80, 110),
            },
            chrome: WidgetChromeTokens {
                scrollbar_track: Color::rgb8(30, 30, 40),
                scrollbar_thumb: Color::rgb8(120, 120, 140),
                caret: Color::rgb8(235, 235, 240),
                focus_ring: Color::rgb8(59, 130, 246),
            },
            typography: TypographyTokens {
                font_size_sm: 12.0,
                font_size_md: 14.0,
                font_size_lg: 18.0,
                font_size_xl: 24.0,
                font_family: "system-ui".to_string(),
                line_height: 1.5,
                letter_spacing: 0.0,
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
    /// Surface fill used for hover states on neutral controls.
    pub surface_hover: Color,
    /// Surface fill used for pressed states on neutral controls.
    pub surface_pressed: Color,
    pub foreground: Color,
    pub foreground_secondary: Color,
    /// Foreground color for disabled text and icons.
    pub foreground_disabled: Color,
    /// Text and icons on [`accent`](Self::accent) fills (buttons, select triggers).
    pub on_accent: Color,
    pub accent: Color,
    pub accent_hover: Color,
    /// Accent fill used for pressed states on accent controls.
    pub accent_pressed: Color,
    pub error: Color,
    pub border: Color,
}

/// Color tokens for paint-layer widget chrome that is drawn directly in the paint pass.
///
/// These tokens control the colors of UI decorations such as scrollbar tracks and thumbs,
/// the text-input caret, and the focus ring. They are read at paint time via
/// [`current_theme`], so swapping the active theme with [`set_active_theme`] or
/// [`run_with_theme`](crate::platform::run_with_theme) changes widget chrome without
/// recompiling any widget code.
///
/// # Example
///
/// ```no_run
/// use lemon::theme::{Theme, WidgetChromeTokens};
/// use lemon::element::style::Color;
///
/// let mut theme = Theme::default_dark();
/// theme.chrome.focus_ring = Color::rgb8(255, 200, 0); // bright yellow focus ring
/// lemon::theme::set_active_theme(theme);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetChromeTokens {
    /// Fill color for the scrollbar track (the background rail behind the thumb).
    pub scrollbar_track: Color,
    /// Fill color for the scrollbar thumb (the draggable indicator).
    pub scrollbar_thumb: Color,
    /// Stroke color for the text-input cursor/caret line.
    pub caret: Color,
    /// Stroke color for the focus ring drawn around focused text inputs.
    pub focus_ring: Color,
}

/// Typography token group used by text styles.
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTokens {
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    /// CSS-like font family list used by default text styles.
    pub font_family: String,
    /// Unitless line-height multiplier relative to font size.
    pub line_height: f32,
    /// Additional glyph spacing in logical points.
    pub letter_spacing: f32,
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
    static ACTIVE_THEME: RefCell<Rc<Theme>> = RefCell::new(Rc::new(Theme::default_dark()));
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
    use super::{current_theme, set_active_theme, Theme, WidgetChromeTokens};
    use crate::element::style::Color;

    #[test]
    fn active_theme_defaults_to_dark_on_new_thread() {
        let from_thread = std::thread::spawn(current_theme)
            .join()
            .expect("theme thread should join");
        assert_eq!(from_thread, Theme::default_dark());
    }

    #[test]
    fn active_theme_roundtrip_works() {
        let previous = current_theme();
        let dark = Theme::default_dark();
        set_active_theme(dark.clone());
        assert_eq!(current_theme(), dark);
        set_active_theme(previous);
    }

    #[test]
    fn default_light_chrome_tokens_are_set() {
        let theme = Theme::default_light();
        assert_ne!(theme.colors.surface_hover, theme.colors.surface);
        assert_ne!(theme.colors.surface_pressed, theme.colors.surface);
        assert_ne!(theme.colors.accent_pressed, theme.colors.accent);
        assert_ne!(theme.colors.foreground_disabled, theme.colors.foreground);
        // Focus ring matches the accent blue.
        assert_eq!(theme.chrome.focus_ring, Color::rgb8(59, 130, 246));
        // Caret matches the dark foreground for a light background.
        assert_eq!(theme.chrome.caret, Color::rgb8(15, 23, 42));
        // Scrollbar colors are distinguishable.
        assert_ne!(theme.chrome.scrollbar_track, theme.chrome.scrollbar_thumb);
    }

    #[test]
    fn default_dark_chrome_tokens_are_set() {
        let theme = Theme::default_dark();
        assert_ne!(theme.colors.surface_hover, theme.colors.surface);
        assert_ne!(theme.colors.surface_pressed, theme.colors.surface);
        assert_ne!(theme.colors.accent_pressed, theme.colors.accent);
        assert_ne!(theme.colors.foreground_disabled, theme.colors.foreground);
        // Focus ring matches the accent blue.
        assert_eq!(theme.chrome.focus_ring, Color::rgb8(59, 130, 246));
        // Caret matches the light foreground for a dark background.
        assert_eq!(theme.chrome.caret, Color::rgb8(235, 235, 240));
        // Scrollbar colors are distinguishable.
        assert_ne!(theme.chrome.scrollbar_track, theme.chrome.scrollbar_thumb);
    }

    #[test]
    fn widget_chrome_tokens_can_be_overridden_independently() {
        let previous = current_theme();

        let mut custom = Theme::default_light();
        custom.chrome = WidgetChromeTokens {
            scrollbar_track: Color::rgb8(1, 2, 3),
            scrollbar_thumb: Color::rgb8(4, 5, 6),
            caret: Color::rgb8(7, 8, 9),
            focus_ring: Color::rgb8(10, 11, 12),
        };
        set_active_theme(custom.clone());

        let active = current_theme();
        assert_eq!(active.chrome.scrollbar_track, Color::rgb8(1, 2, 3));
        assert_eq!(active.chrome.scrollbar_thumb, Color::rgb8(4, 5, 6));
        assert_eq!(active.chrome.caret, Color::rgb8(7, 8, 9));
        assert_eq!(active.chrome.focus_ring, Color::rgb8(10, 11, 12));
        // Other token groups remain unchanged from the base theme.
        assert_eq!(active.colors, custom.colors);

        set_active_theme(previous);
    }
}
