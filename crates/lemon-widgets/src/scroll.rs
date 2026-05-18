use lemon::{
    element::{builders::View, Element},
    Cx, Overflow, Signal,
};

/// Vertical scroll viewport with internal offset state and a painted scrollbar.
pub struct Scroll {
    child: Element,
    offset: Signal<f64>,
    height: f32,
    width: Option<f32>,
}

impl Scroll {
    /// Creates a scroll region that owns its scroll offset (starts at `0`).
    pub fn new(cx: &Cx, child: impl Into<Element>) -> Self {
        Self::with_offset(cx.use_signal(0.0f64), child)
    }

    /// Creates a scroll region backed by a caller-owned offset signal (advanced use).
    pub fn with_offset(offset: Signal<f64>, child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            offset,
            height: 200.0,
            width: None,
        }
    }

    pub fn height(mut self, value: f32) -> Self {
        self.height = value;
        self
    }

    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value);
        self
    }

    pub fn into_element(self) -> Element {
        let offset_value = self.offset.get().max(0.0) as f32;
        let offset = self.offset.clone();

        let mut viewport = View::new()
            .height(self.height)
            .overflow(Overflow::Hidden)
            .scroll_viewport()
            .on_scroll(move |delta| {
                let next = (offset.get() - delta).max(0.0);
                offset.set(next);
            });

        if let Some(width) = self.width {
            viewport = viewport.width(width);
        }

        viewport
            .child(View::new().margin_top(-offset_value).child(self.child))
            .into_element()
    }
}

impl From<Scroll> for Element {
    fn from(scroll: Scroll) -> Self {
        scroll.into_element()
    }
}
