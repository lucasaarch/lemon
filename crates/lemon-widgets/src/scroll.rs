use lemon::{
    element::{builders::View, Element},
    Overflow, Signal,
};

/// Simple vertical scroll viewport that updates a caller-owned offset signal.
pub struct Scroll {
    child: Element,
    offset: Signal<f64>,
    height: f32,
    width: Option<f32>,
}

impl Scroll {
    pub fn new(child: impl Into<Element>, offset: Signal<f64>) -> Self {
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
