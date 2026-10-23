//! Bundled vector icon packs. Lucide icons retain their upstream license in
//! `licenses/lucide-LICENSE` and are rendered directly by Vello, without a font.
//!
//! ```
//! use lemon::{icons::lucide, Icon};
//! let icon = Icon::new(lucide::FOLDER).size(16.0);
//! ```

pub mod lucide;

/// An immutable icon definition on a 24 × 24 coordinate grid.
///
/// Use constants from [`lucide`] with [`crate::Icon`]. No asset files or network
/// requests are needed at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IconData {
    pub(crate) name: &'static str,
    pub(crate) stroke: &'static str,
    pub(crate) fill: &'static str,
}

impl IconData {
    /// Returns the original kebab-case name, such as `folder-open`.
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// Resolved icon paint attached to a container by [`crate::Icon`].
#[derive(Clone, Debug, PartialEq)]
pub struct IconPaint {
    /// The icon definition to render.
    pub data: IconData,
    /// Stroke and fill color.
    pub color: crate::Color,
    /// Stroke width in the icon's 24-unit coordinate grid (default: 2).
    pub stroke_width: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entire_catalog_has_valid_paths_and_unique_names() {
        let mut names = std::collections::HashSet::new();
        assert!(lucide::ALL.len() > 1900);
        for icon in lucide::ALL {
            assert!(names.insert(icon.name()));
            assert!(!icon.stroke.is_empty());
            assert!(
                vello::kurbo::BezPath::from_svg(icon.stroke).is_ok(),
                "{}",
                icon.name()
            );
            if !icon.fill.is_empty() {
                assert!(
                    vello::kurbo::BezPath::from_svg(icon.fill).is_ok(),
                    "{}",
                    icon.name()
                );
            }
        }
    }
}
