/// Text that may be static or dynamically evaluated from a signal-reading closure.
pub enum TextContent {
    Static(String),
    Dynamic(Box<dyn Fn() -> String>),
}

impl TextContent {
    pub fn resolve(&self) -> String {
        match self { Self::Static(s) => s.clone(), Self::Dynamic(f) => f() }
    }
}

impl From<&str> for TextContent {
    fn from(s: &str) -> Self { TextContent::Static(s.to_owned()) }
}

impl From<String> for TextContent {
    fn from(s: String) -> Self { TextContent::Static(s) }
}

impl<F: Fn() -> String + 'static> From<F> for TextContent {
    fn from(f: F) -> Self { TextContent::Dynamic(Box::new(f)) }
}

