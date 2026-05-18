use lemon::{
    element::{builders::View, Element},
    ImageHandle,
};

/// Image widget backed by Lemon's built-in container paint support.
///
/// `Image` is a small builder that configures a [`View`] with an image paint handle.
/// The image is drawn by Lemon's existing paint pipeline using contain scaling.
///
/// # Examples
///
/// ```no_run
/// use lemon::ImageHandle;
/// use lemon_widgets::Image;
///
/// # fn example(handle: ImageHandle) {
/// let _ = Image::new(handle).width(160.0).height(90.0).into_element();
/// # }
/// ```
pub struct Image {
    handle: ImageHandle,
    width: Option<f32>,
    height: Option<f32>,
}

impl Image {
    /// Creates a new image widget from a decoded [`ImageHandle`].
    ///
    /// The handle is attached to an underlying [`View`] so the existing paint pass
    /// can render it. Size defaults to auto unless overridden with [`width`](Self::width)
    /// and [`height`](Self::height).
    pub fn new(handle: ImageHandle) -> Self {
        Self {
            handle,
            width: None,
            height: None,
        }
    }

    /// Sets a fixed width in logical points.
    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value);
        self
    }

    /// Sets a fixed height in logical points.
    pub fn height(mut self, value: f32) -> Self {
        self.height = Some(value);
        self
    }

    /// Builds and returns the underlying [`Element`].
    pub fn into_element(self) -> Element {
        let mut view = View::new().image(self.handle);

        if let Some(width) = self.width {
            view = view.width(width);
        }

        if let Some(height) = self.height {
            view = view.height(height);
        }

        view.into_element()
    }
}

impl From<Image> for Element {
    fn from(image: Image) -> Self {
        image.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::asset::image_handle::ImageData;
    use lemon::element::style::Dimension;
    use std::sync::Arc;

    #[test]
    fn image_widget_produces_view_with_image_paint() {
        let handle = ImageHandle::from_arc(Arc::new(ImageData {
            width: 2,
            height: 2,
            pixels: vec![255u8; 2 * 2 * 4],
        }));
        let expected = handle.clone();

        let element = Image::new(handle).width(100.0).height(80.0).into_element();

        let Element::View(view) = element else {
            panic!("expected View element from Image");
        };

        assert_eq!(view.paint.image, Some(expected));
        assert_eq!(view.style.width, Some(Dimension::Points(100.0)));
        assert_eq!(view.style.height, Some(Dimension::Points(80.0)));
    }

    #[test]
    fn image_widget_implements_into_element_api() {
        let handle = ImageHandle::from_arc(Arc::new(ImageData {
            width: 1,
            height: 1,
            pixels: vec![0u8; 4],
        }));

        let _ = Image::new(handle).into_element();
    }
}
