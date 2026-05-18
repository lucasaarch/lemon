//! Opt-in verbose tracing for debugging Lemon's reactive pipeline.
//!
//! # Enabling
//!
//! Set an environment variable before launching your app:
//!
//! ```bash
//! LEMON_DEBUG=1 cargo run -p components
//! # or filter categories:
//! LEMON_DEBUG=runtime,patches,input,layout cargo run -p components
//! ```
//!
//! `LEMON_TRACE=1` is an alias for `LEMON_DEBUG=1` (all categories).
//!
//! # Categories
//!
//! | Category   | What gets logged                                              |
//! |------------|---------------------------------------------------------------|
//! | `runtime`  | Effect flush, component slots, render/diff of virtual trees  |
//! | `patches`  | Each patch applied to the retained tree                       |
//! | `layout`   | Layout pass entry/exit                                        |
//! | `paint`    | Paint pass stats                                              |
//! | `input`    | Clicks, keyboard dispatch, hit targets                         |
//! | `signal`   | Signal `set` / `update` (very noisy)                          |
//! | `retained` | Retained-tree mount and structural changes                    |
//!
//! Use `1`, `true`, `yes`, or `all` to enable every category.
//!
//! # Programmatic API
//!
//! ```
//! lemon::debug::enable_all();
//! lemon::debug::set_categories(&["runtime", "patches"]);
//! ```

use std::cell::Cell;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use crate::diff::{NodePath, Patch};
use crate::element::Element;

/// Trace category switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Runtime,
    Patches,
    Layout,
    Paint,
    Input,
    Signal,
    Retained,
}

impl Category {
    const ALL: &'static [Category] = &[
        Category::Runtime,
        Category::Patches,
        Category::Layout,
        Category::Paint,
        Category::Input,
        Category::Signal,
        Category::Retained,
    ];

    fn name(self) -> &'static str {
        match self {
            Category::Runtime => "runtime",
            Category::Patches => "patches",
            Category::Layout => "layout",
            Category::Paint => "paint",
            Category::Input => "input",
            Category::Signal => "signal",
            Category::Retained => "retained",
        }
    }

    fn parse(name: &str) -> Option<Category> {
        match name.trim().to_ascii_lowercase().as_str() {
            "runtime" => Some(Category::Runtime),
            "patches" | "patch" => Some(Category::Patches),
            "layout" => Some(Category::Layout),
            "paint" => Some(Category::Paint),
            "input" => Some(Category::Input),
            "signal" | "signals" => Some(Category::Signal),
            "retained" => Some(Category::Retained),
            _ => None,
        }
    }
}

struct DebugConfig {
    enabled: bool,
    categories: u32,
}

impl DebugConfig {
    const fn disabled() -> Self {
        Self {
            enabled: false,
            categories: 0,
        }
    }

    fn enable_all(&mut self) {
        self.enabled = true;
        self.categories = u32::MAX;
    }

    fn set_categories(&mut self, names: &[&str]) {
        self.enabled = true;
        self.categories = 0;
        for name in names {
            let normalized = name.trim();
            if matches!(normalized, "1" | "true" | "yes" | "on" | "all" | "*") {
                self.enable_all();
                return;
            }
            if let Some(cat) = Category::parse(normalized) {
                self.categories |= category_bit(cat);
            } else if !normalized.is_empty() {
                eprintln!("[lemon:debug] unknown category {normalized:?} (ignored)");
            }
        }
    }

    fn enabled(&self, cat: Category) -> bool {
        self.enabled && (self.categories & category_bit(cat)) != 0
    }
}

fn category_bit(cat: Category) -> u32 {
    1 << (cat as u32)
}

static CONFIG: OnceLock<DebugConfig> = OnceLock::new();
static FRAME: AtomicU64 = AtomicU64::new(0);

fn config() -> &'static DebugConfig {
    CONFIG.get_or_init(|| {
        if let Ok(value) = std::env::var("LEMON_DEBUG") {
            let mut cfg = DebugConfig::disabled();
            parse_env_value(&mut cfg, &value);
            log_startup(&cfg, "LEMON_DEBUG", &value);
            return cfg;
        }
        if let Ok(value) = std::env::var("LEMON_TRACE") {
            let mut cfg = DebugConfig::disabled();
            parse_env_value(&mut cfg, &value);
            log_startup(&cfg, "LEMON_TRACE", &value);
            return cfg;
        }
        DebugConfig::disabled()
    })
}

fn parse_env_value(cfg: &mut DebugConfig, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if matches!(trimmed, "1" | "true" | "yes" | "on" | "all" | "*") {
        cfg.enable_all();
        return;
    }
    let parts: Vec<&str> = trimmed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    cfg.set_categories(&parts);
}

fn log_startup(cfg: &DebugConfig, var: &str, raw: &str) {
    if !cfg.enabled {
        eprintln!("[lemon:debug] {var}={raw:?} (no categories enabled)");
        return;
    }
    let mut names = Vec::new();
    for cat in Category::ALL {
        if cfg.enabled(*cat) {
            names.push(cat.name());
        }
    }
    eprintln!(
        "[lemon:debug] enabled via {var}={raw:?} categories=[{}]",
        names.join(", ")
    );
}

/// Re-read is not supported; call before `run` if using env vars.
pub fn configure_from_env() {
    let _ = config();
}

/// Enable every trace category (overrides env until [`disable`]).
pub fn enable_all() {
    PROGRAMMATIC.with(|p| {
        p.enabled.set(true);
        p.categories.set(u32::MAX);
    });
    eprintln!("[lemon:debug] enable_all()");
}

/// Disable all tracing.
pub fn disable() {
    PROGRAMMATIC.with(|p| {
        p.enabled.set(false);
        p.categories.set(0);
    });
    eprintln!("[lemon:debug] disable()");
}

/// Enable only the listed categories (`"runtime"`, `"patches"`, …).
pub fn set_categories(names: &[&str]) {
    PROGRAMMATIC.with(|p| {
        p.enabled.set(true);
        p.categories.set(0);
        for name in names {
            if matches!(*name, "1" | "true" | "yes" | "on" | "all" | "*") {
                p.categories.set(u32::MAX);
                break;
            }
            if let Some(cat) = Category::parse(name) {
                p.categories.set(p.categories.get() | category_bit(cat));
            }
        }
    });
    eprintln!("[lemon:debug] set_categories({names:?})");
}

struct ProgrammaticFlags {
    enabled: Cell<bool>,
    categories: Cell<u32>,
}

impl ProgrammaticFlags {
    const fn new() -> Self {
        Self {
            enabled: Cell::new(false),
            categories: Cell::new(0),
        }
    }
}

thread_local! {
    static PROGRAMMATIC: ProgrammaticFlags = const { ProgrammaticFlags::new() };
}

/// Returns whether tracing is enabled for `cat`.
pub fn enabled(cat: Category) -> bool {
    let programmatic = PROGRAMMATIC.with(|p| {
        if p.enabled.get() {
            Some((p.categories.get() & category_bit(cat)) != 0)
        } else {
            None
        }
    });
    if let Some(on) = programmatic {
        return on;
    }
    config().enabled(cat)
}

/// Log a formatted line when `cat` is enabled.
#[inline]
pub fn trace(cat: Category, message: impl AsRef<str>) {
    if enabled(cat) {
        eprintln!("[lemon:{}] {}", cat.name(), message.as_ref());
    }
}

/// Log with `format!` args when `cat` is enabled.
#[macro_export]
macro_rules! lemon_trace {
    ($cat:ident, $($tt:tt)*) => {
        $crate::debug::trace(
            $crate::debug::Category::$cat,
            format!($($tt)*),
        )
    };
}

/// Bump the per-frame counter (used in log prefixes during `update_frame`).
pub fn next_frame() -> u64 {
    FRAME.fetch_add(1, Ordering::Relaxed) + 1
}

pub fn frame_tag() -> String {
    format!("f{}", FRAME.load(Ordering::Relaxed))
}

/// Format a [`NodePath`] as `[0,1,2]`.
pub fn format_path(path: &NodePath) -> String {
    if path.0.is_empty() {
        return "[]".to_string();
    }
    let mut out = String::from("[");
    for (i, seg) in path.0.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "{seg}");
    }
    out.push(']');
    out
}

/// Short element kind label for logs.
pub fn element_kind(element: &Element) -> &'static str {
    match element {
        Element::View(_) => "View",
        Element::Row(_) => "Row",
        Element::Column(_) => "Column",
        Element::Text(_) => "Text",
        Element::Button(_) => "Button",
        Element::Image(_) => "Image",
        Element::Component(_) => "Component",
        Element::Fragment(_) => "Fragment",
        Element::None => "None",
    }
}

/// One-line patch description.
pub fn format_patch(patch: &Patch) -> String {
    match patch {
        Patch::UpdateComponent { node, component } => format!(
            "UpdateComponent path={} key={:?} identity={}",
            format_path(node),
            component.key(),
            component.identity()
        ),
        Patch::MountComponent { node, component } => format!(
            "MountComponent path={} key={:?} identity={}",
            format_path(node),
            component.key(),
            component.identity()
        ),
        Patch::UnmountComponent { node } => {
            format!("UnmountComponent path={}", format_path(node))
        }
        Patch::UpdateStyle { node, .. } => {
            format!("UpdateStyle path={}", format_path(node))
        }
        Patch::UpdatePaint { node, .. } => {
            format!("UpdatePaint path={}", format_path(node))
        }
        Patch::UpdateText { node, content } => {
            format!("UpdateText path={} content={content:?}", format_path(node))
        }
        Patch::UpdateWidgetChrome { node, .. } => {
            format!("UpdateWidgetChrome path={}", format_path(node))
        }
        Patch::ReplaceNode { node, new_element } => format!(
            "ReplaceNode path={} new={}",
            format_path(node),
            element_kind(new_element)
        ),
        Patch::InsertChild {
            parent,
            index,
            element,
        } => format!(
            "InsertChild parent={} index={index} child={}",
            format_path(parent),
            element_kind(element)
        ),
        Patch::RemoveChild { parent, index } => {
            format!("RemoveChild parent={} index={index}", format_path(parent))
        }
        Patch::MoveChild { parent, from, to } => format!(
            "MoveChild parent={} from={from} to={to}",
            format_path(parent)
        ),
    }
}

/// Log a batch of patches (patches category).
pub fn trace_patches(context: &str, patches: &[Patch]) {
    if !enabled(Category::Patches) && !enabled(Category::Runtime) {
        return;
    }
    if patches.is_empty() {
        trace(Category::Patches, format!("{context}: (no patches)"));
        return;
    }
    trace(
        Category::Patches,
        format!("{context}: {} patch(es)", patches.len()),
    );
    for (i, patch) in patches.iter().enumerate() {
        trace(
            Category::Patches,
            format!("  [{i}] {}", format_patch(patch)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_parse_and_format_patch() {
        assert_eq!(Category::parse("runtime"), Some(Category::Runtime));
        assert_eq!(Category::parse("PATCHES"), Some(Category::Patches));
        assert!(Category::parse("unknown").is_none());

        let patch = Patch::UpdateText {
            node: NodePath(vec![5, 0]),
            content: "1".to_string(),
        };
        let text = format_patch(&patch);
        assert!(text.contains("UpdateText"));
        assert!(text.contains("[5,0]"));
        assert!(text.contains("\"1\""));
    }
}
