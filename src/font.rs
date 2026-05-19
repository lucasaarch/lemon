use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;

use parley::fontique::FontInfoOverride;
use parley::FontContext;
use vello::peniko::Blob;

/// Errors returned when registering custom fonts into a [`FontContext`].
#[derive(Debug)]
pub enum FontRegistrationError {
    /// Reading a font file from disk failed.
    Io(std::io::Error),
    /// The provided data did not contain a readable font face.
    InvalidFontData { family_name: String },
}

impl Display for FontRegistrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read font file: {err}"),
            Self::InvalidFontData { family_name } => write!(
                f,
                "font data for family \"{family_name}\" did not contain any readable faces"
            ),
        }
    }
}

impl std::error::Error for FontRegistrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidFontData { .. } => None,
        }
    }
}

impl From<std::io::Error> for FontRegistrationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Registers a bundled font (for example from [`include_bytes!`]) under `family_name`.
///
/// After registration, you can select this family from text styles with
/// [`.font_family(...)`](crate::Text::font_family), for example:
///
/// ```no_run
/// use lemon::register_font_bytes;
/// use parley::FontContext;
///
/// let bundled = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));
/// let mut fonts = FontContext::new();
/// register_font_bytes(&mut fonts, "MyApp Sans", bundled.to_vec()).unwrap();
/// ```
pub fn register_font_bytes(
    font_cx: &mut FontContext,
    family_name: impl Into<String>,
    bytes: impl Into<Vec<u8>>,
) -> Result<(), FontRegistrationError> {
    register_font_data(font_cx, family_name.into(), Arc::new(bytes.into()))
}

/// Loads a font file from `path` and registers it under `family_name`.
///
/// This is equivalent to reading the file into memory and calling
/// [`register_font_bytes`].
pub fn register_font_path(
    font_cx: &mut FontContext,
    family_name: impl Into<String>,
    path: impl AsRef<Path>,
) -> Result<(), FontRegistrationError> {
    let bytes = std::fs::read(path.as_ref())?;
    register_font_bytes(font_cx, family_name, bytes)
}

pub(crate) fn register_font_data(
    font_cx: &mut FontContext,
    family_name: String,
    bytes: Arc<Vec<u8>>,
) -> Result<(), FontRegistrationError> {
    let added = font_cx.collection.register_fonts(
        Blob::new(bytes),
        Some(FontInfoOverride {
            family_name: Some(family_name.as_str()),
            ..Default::default()
        }),
    );
    if added.is_empty() {
        return Err(FontRegistrationError::InvalidFontData { family_name });
    }
    Ok(())
}
