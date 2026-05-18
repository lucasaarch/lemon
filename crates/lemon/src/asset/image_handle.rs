use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Raw decoded image pixels in RGBA8 format.
///
/// Stored row-major: pixel `(x, y)` starts at byte index `(y * width + x) * 4`.
/// Each pixel is four bytes: `[R, G, B, A]`, each in the range `0..=255`.
pub struct ImageData {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// Raw RGBA8 pixel data, row-major. Length is `width * height * 4`.
    pub pixels: Vec<u8>,
}

/// A cheaply-cloneable handle to a decoded RGBA8 image.
///
/// `ImageHandle` wraps an [`Arc<ImageData>`] and uses **pointer equality** for
/// `PartialEq`, `Eq`, and `Hash`. Two handles are equal only if they point to
/// the same allocation, not if their pixel contents happen to match.
///
/// # Loading images
///
/// ```no_run
/// use lemon::ImageHandle;
///
/// // From in-memory bytes (PNG or JPEG):
/// let handle = ImageHandle::from_bytes(&std::fs::read("icon.png").unwrap()).unwrap();
///
/// // From a file path:
/// let handle = ImageHandle::from_path("assets/photo.jpg").unwrap();
///
/// println!("{}x{}", handle.width(), handle.height());
/// ```
#[derive(Clone)]
pub struct ImageHandle(Arc<ImageData>);

/// Error returned when image decoding or I/O fails.
///
/// The inner `String` contains a human-readable description of the failure,
/// forwarded from the underlying `image` crate or `std::io`.
#[derive(Debug)]
pub struct ImageError(pub String);

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ImageError {}

impl ImageHandle {
    /// Decode `bytes` as a PNG or JPEG image and return an `ImageHandle`.
    ///
    /// The bytes are decoded into RGBA8 format regardless of the source color
    /// space or channel count. Returns [`ImageError`] if the data is not a
    /// recognized image format or is malformed.
    ///
    /// # Errors
    ///
    /// Returns `Err(ImageError)` when the `image` crate cannot decode `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| ImageError(e.to_string()))?
            .to_rgba8();
        Ok(Self(Arc::new(ImageData {
            width: img.width(),
            height: img.height(),
            pixels: img.into_raw(),
        })))
    }

    /// Read the file at `path` and decode it as a PNG or JPEG image.
    ///
    /// Equivalent to reading the file to bytes and calling [`from_bytes`](Self::from_bytes).
    ///
    /// # Errors
    ///
    /// Returns `Err(ImageError)` when the file cannot be read or the contents
    /// cannot be decoded as a supported image format.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let bytes = std::fs::read(path).map_err(|e| ImageError(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Width of the decoded image in pixels.
    pub fn width(&self) -> u32 {
        self.0.width
    }

    /// Height of the decoded image in pixels.
    pub fn height(&self) -> u32 {
        self.0.height
    }

    /// Raw RGBA8 pixel data, row-major.
    ///
    /// Length is `width * height * 4`. Each group of four bytes is
    /// `[R, G, B, A]` for one pixel.
    pub fn pixels(&self) -> &[u8] {
        &self.0.pixels
    }

    /// Wrap an existing [`Arc<ImageData>`] in an `ImageHandle`.
    ///
    /// Useful in tests and in the paint layer where decoded pixel data is
    /// already available in an `Arc`. Both resulting handles share the same
    /// backing allocation, so they compare equal.
    ///
    /// ```
    /// use lemon::ImageHandle;
    /// use lemon::asset::image_handle::ImageData;
    /// use std::sync::Arc;
    ///
    /// let data = Arc::new(ImageData { width: 2, height: 2, pixels: vec![0u8; 16] });
    /// let handle = ImageHandle::from_arc(data);
    /// assert_eq!(handle.width(), 2);
    /// assert_eq!(handle.height(), 2);
    /// ```
    pub fn from_arc(data: Arc<ImageData>) -> Self {
        Self(data)
    }
}

/// Pointer equality — two handles are equal only if they share the same
/// backing [`Arc<ImageData>`] allocation.
impl PartialEq for ImageHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ImageHandle {}

/// Hash based on the pointer address of the backing [`Arc<ImageData>`].
impl Hash for ImageHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

impl std::fmt::Debug for ImageHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImageHandle({}x{})", self.0.width, self.0.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a valid 1×1 white RGBA PNG using the `image` crate encoder.
    fn minimal_rgba_png() -> Vec<u8> {
        use image::{ImageBuffer, ImageFormat, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(1, 1, Rgba([255u8, 255, 255, 255]));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
            .expect("in-memory PNG encode should not fail");
        buf
    }

    #[test]
    fn from_bytes_decodes_png() {
        let handle = ImageHandle::from_bytes(&minimal_rgba_png())
            .expect("hand-crafted 1×1 RGBA PNG should decode successfully");
        assert_eq!(handle.width(), 1);
        assert_eq!(handle.height(), 1);
        assert_eq!(handle.pixels().len(), 4, "1×1 RGBA image has 4 bytes");
    }

    #[test]
    fn from_bytes_invalid_returns_err() {
        let result = ImageHandle::from_bytes(b"not a png");
        assert!(result.is_err());
    }

    #[test]
    fn two_handles_from_same_arc_are_equal() {
        let a = ImageHandle(Arc::new(ImageData {
            width: 1,
            height: 1,
            pixels: vec![255, 0, 0, 255],
        }));
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn two_independent_handles_are_not_equal() {
        let make = || {
            ImageHandle(Arc::new(ImageData {
                width: 1,
                height: 1,
                pixels: vec![255, 0, 0, 255],
            }))
        };
        assert_ne!(make(), make());
    }

    #[test]
    fn width_height_accessors() {
        let h = ImageHandle(Arc::new(ImageData {
            width: 42,
            height: 7,
            pixels: vec![0u8; 42 * 7 * 4],
        }));
        assert_eq!(h.width(), 42);
        assert_eq!(h.height(), 7);
    }
}
