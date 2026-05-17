# Lemon Core Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Lemon's pure core (Layers 1–4 of the architecture spec): reactive runtime (signals, derived, effects, component context), element tree with fluent builders, and the diff/patch engine — all testable in `cargo test` without GPU or OS.

**Architecture:** Signals track reactive dependencies via a thread-local observer stack (Rc-based, single-threaded). Components are functions called with a `Cx` context that stores hook state by call index (React-style). Each component has a `ComponentSlot` in the `Runtime` that diffs old vs new `Element` trees on re-render and produces a `Vec<Patch>`. No GPU, no winit — just pure Rust.

**Tech Stack:** Rust 2024 edition; no additional dependencies for this plan (Cargo.toml already has the full stack for Plan 2).

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md`

---

## File Structure

```
src/
  lib.rs                    ← public re-exports; integration tests
  runtime/
    mod.rs                  ← Runtime, ComponentSlot, PatchQueue
    observer.rs             ← thread-local observer stack; Subscriber trait
    signal.rs               ← Signal<T>
    derived.rs              ← Derived<T>
    effect.rs               ← Effect
    cx.rs                   ← Cx (component context with hook index)
  element/
    mod.rs                  ← Element enum
    style.rs                ← StyleProps, PaintProps, PaintData, ColorSource, Color, CornerRadii, Edges, Dimension, Align, Justify
    content.rs              ← TextContent, TextStyle
    types.rs                ← BoxElement, TextElement, ButtonElement, ImageElement, ComponentElement, Key
    builders.rs             ← Column, Row, Box_, Text, Button fluent builders
  diff/
    mod.rs                  ← NodePath, Patch, diff()
```

**Dependency order (build bottom-up):**
`observer` → `signal`, `derived`, `effect` → `cx` → element types → builders → `diff` → `runtime::mod` → `lib`

---

### Task 1: Module Skeleton

**Files:**
- Delete: `src/main.rs`
- Create: `src/lib.rs`, all module stubs

- [ ] **Step 1: Replace main.rs with lib.rs and declare modules**

Delete `src/main.rs`. Create `src/lib.rs`:

```rust
pub mod diff;
pub mod element;
pub mod runtime;
```

Create `src/runtime/mod.rs`:
```rust
pub mod cx;
pub mod derived;
pub mod effect;
pub mod observer;
pub mod signal;
```

Create `src/element/mod.rs`:
```rust
pub mod builders;
pub mod content;
pub mod style;
pub mod types;
```

Create empty stub files (each containing just a comment `// TODO`):
- `src/runtime/observer.rs`
- `src/runtime/signal.rs`
- `src/runtime/derived.rs`
- `src/runtime/effect.rs`
- `src/runtime/cx.rs`
- `src/element/style.rs`
- `src/element/content.rs`
- `src/element/types.rs`
- `src/element/builders.rs`
- `src/diff/mod.rs`

- [ ] **Step 2: Verify the crate compiles**

```bash
cargo build 2>&1 | head -20
```

Expected: compiles cleanly. Empty stub files are valid Rust.

- [ ] **Step 3: Commit**

```bash
git add src/
git commit -m "chore: scaffold module structure for lemon core"
```

---

### Task 2: Observer Stack

**Files:**
- Modify: `src/runtime/observer.rs`

The observer stack is the foundation of all reactivity. It is a thread-local `Vec<Weak<dyn Subscriber>>`. When `signal.get()` is called inside a reactive scope, it registers the current observer as a subscriber.

- [ ] **Step 1: Write the failing tests**

Add to `src/runtime/observer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct MockSub(Cell<u32>);
    impl Subscriber for MockSub {
        fn mark_dirty(&self) { self.0.set(self.0.get() + 1); }
    }

    #[test]
    fn no_observer_outside_scope() {
        assert!(current_observer().is_none());
    }

    #[test]
    fn observer_present_inside_with_observer() {
        let sub = Rc::new(MockSub(Cell::new(0)));
        with_observer(Rc::downgrade(&sub) as Weak<dyn Subscriber>, || {
            assert!(current_observer().is_some());
        });
        assert!(current_observer().is_none());
    }

    #[test]
    fn with_observer_returns_closure_value() {
        let sub = Rc::new(MockSub(Cell::new(0)));
        let result = with_observer(Rc::downgrade(&sub) as Weak<dyn Subscriber>, || 42);
        assert_eq!(result, 42);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test runtime::observer::tests 2>&1 | head -20
```

Expected: compile error — `Subscriber`, `with_observer`, `current_observer` not defined.

- [ ] **Step 3: Implement the observer stack**

Replace the stub in `src/runtime/observer.rs`:

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub trait Subscriber {
    fn mark_dirty(&self);
}

thread_local! {
    static OBSERVER_STACK: RefCell<Vec<Weak<dyn Subscriber>>> = RefCell::new(Vec::new());
}

pub fn with_observer<R>(observer: Weak<dyn Subscriber>, f: impl FnOnce() -> R) -> R {
    OBSERVER_STACK.with(|stack| stack.borrow_mut().push(observer));
    let result = f();
    OBSERVER_STACK.with(|stack| { stack.borrow_mut().pop(); });
    result
}

pub fn current_observer() -> Option<Weak<dyn Subscriber>> {
    OBSERVER_STACK.with(|stack| stack.borrow().last().cloned())
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test runtime::observer::tests
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add src/runtime/observer.rs
git commit -m "feat(runtime): add observer stack for reactive dependency tracking"
```

---

### Task 3: Signal\<T\>

**Files:**
- Modify: `src/runtime/signal.rs`

`Signal<T>` is the core reactive primitive. `get()` registers the caller as a subscriber when called inside an observer scope; `set()` notifies all live subscribers.

- [ ] **Step 1: Write the failing tests**

Add to `src/runtime/signal.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::observer::{with_observer, Subscriber};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Counter(Cell<u32>);
    impl Subscriber for Counter {
        fn mark_dirty(&self) { self.0.set(self.0.get() + 1); }
    }

    #[test]
    fn get_returns_initial_value() {
        let s = Signal::new(42i32);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn set_updates_value() {
        let s = Signal::new(0i32);
        s.set(99);
        assert_eq!(s.get(), 99);
    }

    #[test]
    fn update_mutates_value() {
        let s = Signal::new(10i32);
        s.update(|v| *v += 5);
        assert_eq!(s.get(), 15);
    }

    #[test]
    fn set_notifies_subscriber() {
        let s = Signal::new(0i32);
        let counter = Rc::new(Counter(Cell::new(0)));
        with_observer(Rc::downgrade(&counter) as _, || { s.get(); });
        s.set(1);
        assert_eq!(counter.0.get(), 1);
    }

    #[test]
    fn dead_subscriber_is_skipped() {
        let s = Signal::new(0i32);
        let counter = Rc::new(Counter(Cell::new(0)));
        with_observer(Rc::downgrade(&counter) as _, || { s.get(); });
        drop(counter);
        s.set(1); // must not panic
    }

    #[test]
    fn clone_shares_state() {
        let s1 = Signal::new(5i32);
        let s2 = s1.clone();
        s1.set(10);
        assert_eq!(s2.get(), 10);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test runtime::signal::tests 2>&1 | head -10
```

Expected: compile error — `Signal` not defined.

- [ ] **Step 3: Implement Signal\<T\>**

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use crate::runtime::observer::{current_observer, Subscriber};

struct SignalInner<T> {
    value: T,
    subscribers: Vec<Weak<dyn Subscriber>>,
}

pub struct Signal<T>(Rc<RefCell<SignalInner<T>>>);

impl<T: Clone + 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        Signal(Rc::new(RefCell::new(SignalInner { value, subscribers: Vec::new() })))
    }

    pub fn get(&self) -> T {
        if let Some(obs) = current_observer() {
            self.0.borrow_mut().subscribers.push(obs);
        }
        self.0.borrow().value.clone()
    }

    pub fn set(&self, value: T) {
        self.0.borrow_mut().value = value;
        self.notify();
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.0.borrow_mut().value);
        self.notify();
    }

    fn notify(&self) {
        let subs: Vec<Rc<dyn Subscriber>> = {
            let mut inner = self.0.borrow_mut();
            inner.subscribers.retain(|w| w.strong_count() > 0);
            inner.subscribers.iter().filter_map(|w| w.upgrade()).collect()
        };
        for sub in subs {
            sub.mark_dirty();
        }
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self { Signal(Rc::clone(&self.0)) }
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test runtime::signal::tests
```

Expected: `test result: ok. 6 passed`

- [ ] **Step 5: Commit**

```bash
git add src/runtime/signal.rs
git commit -m "feat(runtime): implement Signal<T> reactive primitive"
```

---

### Task 4: Derived\<T\>

**Files:**
- Modify: `src/runtime/derived.rs`

`Derived<T>` wraps a computation, tracks its signal dependencies, caches the result, and marks itself stale when a dependency changes.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::signal::Signal;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn computes_on_first_get() {
        let s = Signal::new(2i32);
        let d = Derived::new(move || s.get() * 3);
        assert_eq!(d.get(), 6);
    }

    #[test]
    fn caches_result() {
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s = Signal::new(1i32);
        let d = Derived::new(move || { c.set(c.get() + 1); s.get() });
        d.get();
        d.get();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn recomputes_when_signal_changes() {
        let s = Signal::new(10i32);
        let d = Derived::new(move || s.get() + 1);
        assert_eq!(d.get(), 11);
        s.set(20);
        assert_eq!(d.get(), 21);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test runtime::derived::tests 2>&1 | head -10
```

Expected: compile error — `Derived` not defined.

- [ ] **Step 3: Implement Derived\<T\>**

```rust
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use crate::runtime::observer::{current_observer, with_observer, Subscriber};

struct DerivedInner<T> {
    f: Box<dyn Fn() -> T>,
    cached: RefCell<Option<T>>,
    stale: Cell<bool>,
    self_weak: RefCell<Weak<DerivedInner<T>>>,
    downstream: RefCell<Vec<Weak<dyn Subscriber>>>,
}

impl<T: Clone + 'static> Subscriber for DerivedInner<T> {
    fn mark_dirty(&self) {
        self.stale.set(true);
        self.cached.borrow_mut().take();
        let subs: Vec<Rc<dyn Subscriber>> = self.downstream
            .borrow()
            .iter()
            .filter_map(|w| w.upgrade())
            .collect();
        for sub in subs { sub.mark_dirty(); }
    }
}

pub struct Derived<T>(Rc<DerivedInner<T>>);

impl<T: Clone + 'static> Derived<T> {
    pub fn new(f: impl Fn() -> T + 'static) -> Self {
        let inner = Rc::new(DerivedInner {
            f: Box::new(f),
            cached: RefCell::new(None),
            stale: Cell::new(true),
            self_weak: RefCell::new(Weak::new()),
            downstream: RefCell::new(Vec::new()),
        });
        *inner.self_weak.borrow_mut() = Rc::downgrade(&inner);
        Derived(inner)
    }

    pub fn get(&self) -> T {
        if let Some(obs) = current_observer() {
            self.0.downstream.borrow_mut().push(obs);
        }
        if self.0.stale.get() {
            let weak = self.0.self_weak.borrow().clone();
            let value = with_observer(weak as Weak<dyn Subscriber>, || (self.0.f)());
            self.0.stale.set(false);
            *self.0.cached.borrow_mut() = Some(value.clone());
            value
        } else {
            self.0.cached.borrow().as_ref().unwrap().clone()
        }
    }
}

impl<T> Clone for Derived<T> {
    fn clone(&self) -> Self { Derived(Rc::clone(&self.0)) }
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test runtime::derived::tests
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add src/runtime/derived.rs
git commit -m "feat(runtime): implement Derived<T> cached computed value"
```

---

### Task 5: Effect

**Files:**
- Modify: `src/runtime/effect.rs`

`Effect` runs a closure immediately and re-runs it each time any signal read during the previous run emits a notification.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::signal::Signal;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn runs_immediately() {
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        let _e = Effect::new(move || { r.set(true); });
        assert!(ran.get());
    }

    #[test]
    fn reruns_on_signal_change() {
        let s = Signal::new(0i32);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s2 = s.clone();
        let _e = Effect::new(move || { s2.get(); c.set(c.get() + 1); });
        assert_eq!(count.get(), 1);
        s.set(1);
        assert_eq!(count.get(), 2);
        s.set(2);
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn stops_rerunning_after_drop() {
        let s = Signal::new(0i32);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s2 = s.clone();
        { let _e = Effect::new(move || { s2.get(); c.set(c.get() + 1); }); }
        s.set(99);
        assert_eq!(count.get(), 1); // ran once on mount, not again after drop
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test runtime::effect::tests 2>&1 | head -10
```

Expected: compile error — `Effect` not defined.

- [ ] **Step 3: Implement Effect**

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use crate::runtime::observer::{with_observer, Subscriber};

struct EffectInner {
    f: Box<dyn Fn()>,
    self_weak: RefCell<Weak<EffectInner>>,
}

impl Subscriber for EffectInner {
    fn mark_dirty(&self) {
        if let Some(strong) = self.self_weak.borrow().upgrade() {
            with_observer(Rc::downgrade(&strong) as Weak<dyn Subscriber>, || {
                (strong.f)();
            });
        }
    }
}

pub struct Effect(Rc<EffectInner>);

impl Effect {
    pub fn new(f: impl Fn() + 'static) -> Self {
        let inner = Rc::new(EffectInner {
            f: Box::new(f),
            self_weak: RefCell::new(Weak::new()),
        });
        *inner.self_weak.borrow_mut() = Rc::downgrade(&inner);
        with_observer(Rc::downgrade(&inner) as Weak<dyn Subscriber>, || {
            (inner.f)();
        });
        Effect(inner)
    }
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test runtime::effect::tests
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add src/runtime/effect.rs
git commit -m "feat(runtime): implement Effect reactive side effect"
```

---

### Task 6: Cx (Component Context)

**Files:**
- Modify: `src/runtime/cx.rs`

`Cx` is given to component functions by the runtime. It stores signals and memos by call index — the nth call to `use_signal` always returns the nth signal, even across re-renders. This is the same pattern as React's rules of hooks.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_signal_returns_signal() {
        let cx = Cx::new();
        let s = cx.use_signal(42i32);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn use_signal_same_signal_on_second_call() {
        let cx = Cx::new();
        let s1 = cx.use_signal(0i32);
        s1.set(7);
        cx.reset_hooks();
        let s2 = cx.use_signal(0i32); // same hook index → same signal
        assert_eq!(s2.get(), 7);
    }

    #[test]
    fn use_memo_returns_derived() {
        let cx = Cx::new();
        let s = cx.use_signal(5i32);
        let s2 = s.clone();
        let m = cx.use_memo(move || s2.get() * 2);
        assert_eq!(m.get(), 10);
        s.set(8);
        assert_eq!(m.get(), 16);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test runtime::cx::tests 2>&1 | head -10
```

Expected: compile error — `Cx` not defined.

- [ ] **Step 3: Implement Cx**

```rust
use std::any::Any;
use std::cell::{Cell, RefCell};
use crate::runtime::derived::Derived;
use crate::runtime::effect::Effect;
use crate::runtime::signal::Signal;

pub struct Cx {
    hooks: RefCell<Vec<Box<dyn Any>>>,
    index: Cell<usize>,
    pub(crate) effects: RefCell<Vec<Effect>>,
}

impl Cx {
    pub fn new() -> Self {
        Cx {
            hooks: RefCell::new(Vec::new()),
            index: Cell::new(0),
            effects: RefCell::new(Vec::new()),
        }
    }

    /// Must be called before each re-render of this component.
    pub fn reset_hooks(&self) {
        self.index.set(0);
    }

    pub fn use_signal<T: Clone + 'static>(&self, initial: T) -> Signal<T> {
        let idx = self.index.get();
        self.index.set(idx + 1);
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx].downcast_ref::<Signal<T>>()
                .expect("use_signal: hook type mismatch — called with different type on re-render")
                .clone()
        } else {
            let s = Signal::new(initial);
            hooks.push(Box::new(s.clone()));
            s
        }
    }

    pub fn use_memo<T: Clone + 'static>(&self, f: impl Fn() -> T + 'static) -> Derived<T> {
        let idx = self.index.get();
        self.index.set(idx + 1);
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx].downcast_ref::<Derived<T>>()
                .expect("use_memo: hook type mismatch")
                .clone()
        } else {
            let d = Derived::new(f);
            hooks.push(Box::new(d.clone()));
            d
        }
    }

    pub fn use_effect(&self, f: impl Fn() + 'static) {
        self.effects.borrow_mut().push(Effect::new(f));
    }
}

impl Default for Cx {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test runtime::cx::tests
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add src/runtime/cx.rs
git commit -m "feat(runtime): implement Cx component context with hook index"
```

---

### Task 7: Element Data Types

**Files:**
- Modify: `src/element/style.rs`
- Modify: `src/element/content.rs`
- Modify: `src/element/types.rs`

Pure data structures — no logic, no closures that execute eagerly. These form the vocabulary of the virtual tree.

- [ ] **Step 1: Write construction tests**

In `src/element/types.rs` (add at the bottom):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{style::StyleProps, content::TextContent};

    #[test]
    fn box_element_default_has_no_children() {
        let el = BoxElement::default();
        assert!(el.children.is_empty());
    }

    #[test]
    fn text_element_resolves_static_content() {
        let el = TextElement {
            content: TextContent::Static("hello".into()),
            style: Default::default(),
            key: None,
        };
        assert_eq!(el.content.resolve(), "hello");
    }

    #[test]
    fn text_element_resolves_dynamic_content() {
        let el = TextElement {
            content: TextContent::Dynamic(Box::new(|| "dynamic".to_owned())),
            style: Default::default(),
            key: None,
        };
        assert_eq!(el.content.resolve(), "dynamic");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test element::types::tests 2>&1 | head -10
```

Expected: compile error — types not defined.

- [ ] **Step 3: Implement style.rs**

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edges<T: Clone> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Clone> Edges<T> {
    pub fn all(v: T) -> Self {
        Edges { top: v.clone(), right: v.clone(), bottom: v.clone(), left: v }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Dimension {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Align { #[default] Stretch, Start, End, Center, Baseline }

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Justify { #[default] Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub fn all(r: f32) -> Self {
        CornerRadii { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleProps {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Edges<f32>>,
    pub margin: Option<Edges<f32>>,
    pub gap: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub align_items: Option<Align>,
    pub justify_content: Option<Justify>,
}

/// Color as RGBA floats in 0.0–1.0.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

impl Color {
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
    }
    pub fn with_alpha(mut self, a: f32) -> Self { self.a = a; self }
}

/// A color that may be evaluated dynamically from a closure.
pub enum ColorSource {
    Static(Color),
    Dynamic(Box<dyn Fn() -> Color>),
}

impl ColorSource {
    pub fn resolve(&self) -> Color {
        match self { Self::Static(c) => *c, Self::Dynamic(f) => f() }
    }
}

impl From<Color> for ColorSource {
    fn from(c: Color) -> Self { ColorSource::Static(c) }
}

impl<F: Fn() -> Color + 'static> From<F> for ColorSource {
    fn from(f: F) -> Self { ColorSource::Dynamic(Box::new(f)) }
}

/// Visual decoration properties. May contain dynamic closures.
#[derive(Default)]
pub struct PaintProps {
    pub background: Option<ColorSource>,
    pub border_color: Option<ColorSource>,
    pub border_width: f32,
    pub radius: CornerRadii,
}

/// Resolved paint values with no closures — stored in Retained Tree and Patches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintData {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: CornerRadii,
}

impl PaintProps {
    pub fn resolve(&self) -> PaintData {
        PaintData {
            background: self.background.as_ref().map(|c| c.resolve()),
            border_color: self.border_color.as_ref().map(|c| c.resolve()),
            border_width: self.border_width,
            radius: self.radius.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_weight: u16,
    pub color: Option<Color>,
}
```

- [ ] **Step 4: Implement content.rs**

```rust
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
```

- [ ] **Step 5: Implement types.rs**

```rust
use crate::element::{
    content::{TextContent, TextStyle},
    style::{PaintProps, StyleProps},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u64);

/// Used by Box_, Row, and Column — all three are the same struct.
pub struct BoxElement {
    pub style: StyleProps,
    pub paint: PaintProps,
    pub children: Vec<crate::element::Element>,
    pub key: Option<Key>,
}

impl Default for BoxElement {
    fn default() -> Self {
        BoxElement { style: Default::default(), paint: Default::default(), children: Vec::new(), key: None }
    }
}

pub struct TextElement {
    pub content: TextContent,
    pub style: TextStyle,
    pub key: Option<Key>,
}

pub struct ButtonElement {
    pub label: TextContent,
    pub style: StyleProps,
    pub paint: PaintProps,
    pub on_click: Option<Box<dyn Fn()>>,
    pub key: Option<Key>,
}

pub struct ImageElement {
    pub src: String,
    pub style: StyleProps,
    pub key: Option<Key>,
}

pub struct ComponentElement {
    /// Closure that captures props and calls the component function.
    pub view: Box<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    /// Used for stable component identity across re-renders.
    pub type_id: std::any::TypeId,
    pub key: Option<Key>,
}
```

- [ ] **Step 6: Add Element enum forward declaration to element/mod.rs**

```rust
pub mod builders;
pub mod content;
pub mod style;
pub mod types;

use types::{BoxElement, ButtonElement, ComponentElement, ImageElement, TextElement};

pub enum Element {
    Text(TextElement),
    Box_(BoxElement),
    Row(BoxElement),
    Column(BoxElement),
    Button(ButtonElement),
    Image(ImageElement),
    Component(ComponentElement),
    Fragment(Vec<Element>),
    None,
}
```

Note: `Box_` avoids collision with Rust's built-in `Box<T>`.

- [ ] **Step 7: Run to verify it passes**

```bash
cargo test element::types::tests
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 8: Commit**

```bash
git add src/element/
git commit -m "feat(element): add data types, style, content, and Element enum"
```

---

### Task 8: Builders

**Files:**
- Modify: `src/element/builders.rs`

The fluent builder API — the primary surface users interact with when writing UI.

- [ ] **Step 1: Write the failing tests**

Add to `src/element/builders.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn column_builder_sets_gap() {
        let Element::Column(el) = Column::new().gap(8.0).into_element() else { panic!() };
        assert_eq!(el.style.gap, Some(8.0));
    }

    #[test]
    fn row_with_children() {
        let Element::Row(el) = Row::new()
            .child(Text::new("a"))
            .child(Text::new("b"))
            .into_element() else { panic!() };
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn text_static_content() {
        let Element::Text(el) = Text::new("hello").into_element() else { panic!() };
        assert_eq!(el.content.resolve(), "hello");
    }

    #[test]
    fn text_dynamic_content() {
        let value = Rc::new(Cell::new(7u32));
        let v = value.clone();
        let Element::Text(el) = Text::new(move || v.get().to_string()).into_element() else { panic!() };
        assert_eq!(el.content.resolve(), "7");
        value.set(42);
        assert_eq!(el.content.resolve(), "42");
    }

    #[test]
    fn button_on_click_fires() {
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        let Element::Button(el) = Button::new("OK")
            .on_click(move || f.set(true))
            .into_element() else { panic!() };
        el.on_click.unwrap()();
        assert!(fired.get());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test element::builders::tests 2>&1 | head -10
```

Expected: compile error — builders not defined.

- [ ] **Step 3: Implement builders.rs**

```rust
use crate::element::{
    Element,
    content::{TextContent, TextStyle},
    style::{Color, ColorSource, CornerRadii, Edges, PaintProps, StyleProps},
    types::{BoxElement, ButtonElement, TextElement},
};

// ── Macro to generate container builders (Column, Row, Box_) ──────────────

macro_rules! container_builder {
    ($name:ident, $variant:ident) => {
        pub struct $name(BoxElement);

        impl $name {
            pub fn new() -> Self { $name(BoxElement::default()) }

            pub fn gap(mut self, v: f32) -> Self { self.0.style.gap = Some(v); self }
            pub fn padding(mut self, v: f32) -> Self {
                self.0.style.padding = Some(Edges::all(v)); self
            }
            pub fn width(mut self, v: f32) -> Self {
                self.0.style.width = Some(crate::element::style::Dimension::Points(v)); self
            }
            pub fn height(mut self, v: f32) -> Self {
                self.0.style.height = Some(crate::element::style::Dimension::Points(v)); self
            }
            pub fn flex_grow(mut self, v: f32) -> Self {
                self.0.style.flex_grow = Some(v); self
            }
            pub fn align_items(mut self, v: crate::element::style::Align) -> Self {
                self.0.style.align_items = Some(v); self
            }
            pub fn justify_content(mut self, v: crate::element::style::Justify) -> Self {
                self.0.style.justify_content = Some(v); self
            }
            pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
                self.0.paint.background = Some(c.into()); self
            }
            pub fn border(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color = Some(ColorSource::Static(color));
                self.0.paint.border_width = width;
                self
            }
            pub fn radius(mut self, r: f32) -> Self {
                self.0.paint.radius = CornerRadii::all(r); self
            }
            pub fn child(mut self, el: impl Into<Element>) -> Self {
                self.0.children.push(el.into()); self
            }
            pub fn into_element(self) -> Element { Element::$variant(self.0) }
        }

        impl Default for $name { fn default() -> Self { $name::new() } }
        impl From<$name> for Element { fn from(b: $name) -> Self { b.into_element() } }
    };
}

container_builder!(Column, Column);
container_builder!(Row, Row);
container_builder!(Box_, Box_);

// ── Text ──────────────────────────────────────────────────────────────────

pub struct Text {
    content: TextContent,
    style: TextStyle,
}

impl Text {
    pub fn new(content: impl Into<TextContent>) -> Self {
        Text { content: content.into(), style: TextStyle::default() }
    }
    pub fn font_size(mut self, size: f32) -> Self { self.style.font_size = size; self }
    pub fn weight(mut self, w: u16) -> Self { self.style.font_weight = w; self }
    pub fn color(mut self, c: Color) -> Self { self.style.color = Some(c); self }
    pub fn into_element(self) -> Element {
        Element::Text(TextElement { content: self.content, style: self.style, key: None })
    }
}

impl From<Text> for Element { fn from(b: Text) -> Self { b.into_element() } }

// ── Button ────────────────────────────────────────────────────────────────

pub struct Button {
    label: TextContent,
    style: StyleProps,
    paint: PaintProps,
    on_click: Option<Box<dyn Fn()>>,
}

impl Button {
    pub fn new(label: impl Into<TextContent>) -> Self {
        Button { label: label.into(), style: Default::default(), paint: Default::default(), on_click: None }
    }
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(f)); self
    }
    pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
        self.paint.background = Some(c.into()); self
    }
    pub fn radius(mut self, r: f32) -> Self { self.paint.radius = CornerRadii::all(r); self }
    pub fn into_element(self) -> Element {
        Element::Button(crate::element::types::ButtonElement {
            label: self.label,
            style: self.style,
            paint: self.paint,
            on_click: self.on_click,
            key: None,
        })
    }
}

impl From<Button> for Element { fn from(b: Button) -> Self { b.into_element() } }
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test element::builders::tests
```

Expected: `test result: ok. 5 passed`

- [ ] **Step 5: Commit**

```bash
git add src/element/builders.rs src/element/mod.rs
git commit -m "feat(element): add fluent builder API for Column, Row, Box_, Text, Button"
```

---

### Task 9: Diff Algorithm

**Files:**
- Modify: `src/diff/mod.rs`

`diff(old, new, path)` takes two owned `Element` trees and produces a `Vec<Patch>` describing only what changed. Ownership is used because the Runtime always has both the old and new trees as owned values.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, builders::{Button, Column, Row, Text}};

    fn txt(s: &'static str) -> Element { Text::new(s).into_element() }

    #[test]
    fn identical_text_produces_no_patches() {
        assert!(diff(txt("hi"), txt("hi"), NodePath::root()).is_empty());
    }

    #[test]
    fn changed_text_produces_update_text() {
        let patches = diff(txt("old"), txt("new"), NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::UpdateText { content, .. } if content == "new"));
    }

    #[test]
    fn different_types_produces_replace() {
        let patches = diff(
            Text::new("x").into_element(),
            Button::new("x").into_element(),
            NodePath::root(),
        );
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::ReplaceNode { .. }));
    }

    #[test]
    fn style_change_produces_update_style() {
        let old = Column::new().gap(4.0).into_element();
        let new = Column::new().gap(8.0).into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::UpdateStyle { .. }));
    }

    #[test]
    fn child_added_produces_insert() {
        let old = Column::new().into_element();
        let new = Column::new().child(txt("a")).into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::InsertChild { index: 0, .. }));
    }

    #[test]
    fn child_removed_produces_remove() {
        let old = Column::new().child(txt("a")).into_element();
        let new = Column::new().into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::RemoveChild { index: 0, .. }));
    }

    #[test]
    fn unchanged_children_produce_no_patches() {
        let old = Column::new().child(txt("a")).child(txt("b")).into_element();
        let new = Column::new().child(txt("a")).child(txt("b")).into_element();
        assert!(diff(old, new, NodePath::root()).is_empty());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test diff::tests 2>&1 | head -10
```

Expected: compile error — `diff`, `Patch`, `NodePath` not defined.

- [ ] **Step 3: Implement NodePath and Patch types**

```rust
use crate::element::{Element, style::{PaintData, StyleProps}};

#[derive(Clone, Debug, PartialEq)]
pub struct NodePath(pub Vec<usize>);

impl NodePath {
    pub fn root() -> Self { NodePath(vec![]) }
    pub fn child(&self, index: usize) -> Self {
        let mut v = self.0.clone();
        v.push(index);
        NodePath(v)
    }
}

pub enum Patch {
    UpdateStyle   { node: NodePath, style: StyleProps },
    UpdatePaint   { node: NodePath, paint: PaintData },
    UpdateText    { node: NodePath, content: String },
    ReplaceNode   { node: NodePath, new_element: Element },
    InsertChild   { parent: NodePath, index: usize, element: Element },
    RemoveChild   { parent: NodePath, index: usize },
    MoveChild     { parent: NodePath, from: usize, to: usize },
}
```

- [ ] **Step 4: Implement the diff function**

```rust
pub fn diff(old: Element, new: Element, path: NodePath) -> Vec<Patch> {
    use Element::*;
    let mut patches = Vec::new();

    match (old, new) {
        (Text(o), Text(n)) => {
            let os = o.content.resolve();
            let ns = n.content.resolve();
            if os != ns {
                patches.push(Patch::UpdateText { node: path, content: ns });
            }
        }
        (Column(o), Column(n)) => diff_box(o, n, path, &mut patches),
        (Row(o),    Row(n))    => diff_box(o, n, path, &mut patches),
        (Box_(o),   Box_(n))   => diff_box(o, n, path, &mut patches),
        (Button(o), Button(n)) => {
            let ol = o.label.resolve();
            let nl = n.label.resolve();
            if ol != nl {
                patches.push(Patch::UpdateText { node: path.clone(), content: nl });
            }
            let op = o.paint.resolve();
            let np = n.paint.resolve();
            if op != np {
                patches.push(Patch::UpdatePaint { node: path, paint: np });
            }
        }
        (None, None) | (Image(_), Image(_)) => {}
        // Different types — replace entirely
        (_, new) => {
            patches.push(Patch::ReplaceNode { node: path, new_element: new });
        }
    }

    patches
}

fn diff_box(
    o: crate::element::types::BoxElement,
    n: crate::element::types::BoxElement,
    path: NodePath,
    patches: &mut Vec<Patch>,
) {
    if o.style != n.style {
        patches.push(Patch::UpdateStyle { node: path.clone(), style: n.style });
    }
    let op = o.paint.resolve();
    let np = n.paint.resolve();
    if op != np {
        patches.push(Patch::UpdatePaint { node: path.clone(), paint: np });
    }
    diff_children(o.children, n.children, &path, patches);
}

fn diff_children(
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
) {
    let min = old.len().min(new.len());
    let mut old_iter = old.into_iter();
    let mut new_iter = new.into_iter();

    for i in 0..min {
        let child_patches = diff(old_iter.next().unwrap(), new_iter.next().unwrap(), parent.child(i));
        patches.extend(child_patches);
    }
    // Extra new children → insert
    for (i, el) in new_iter.enumerate() {
        patches.push(Patch::InsertChild { parent: parent.clone(), index: min + i, element: el });
    }
    // Removed old children → remove in reverse order to keep indices stable
    let old_remaining: Vec<_> = old_iter.collect();
    for i in (0..old_remaining.len()).rev() {
        patches.push(Patch::RemoveChild { parent: parent.clone(), index: min + i });
    }
}
```

- [ ] **Step 5: Run to verify it passes**

```bash
cargo test diff::tests
```

Expected: `test result: ok. 7 passed`

- [ ] **Step 6: Commit**

```bash
git add src/diff/mod.rs
git commit -m "feat(diff): implement recursive diff algorithm producing Patch list"
```

---

### Task 10: Runtime

**Files:**
- Modify: `src/runtime/mod.rs`

`Runtime` orchestrates component mounting and re-rendering. Each mounted component has a `ComponentSlot` that holds its `Cx`, the previous `Element` tree, and a flag for pending re-renders.

- [ ] **Step 1: Write the failing tests**

Add to `src/runtime/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, builders::{Column, Text}};
    use crate::diff::Patch;

    #[test]
    fn no_patches_on_first_mount() {
        let mut rt = Runtime::new();
        rt.mount(|_cx| Text::new("hello").into_element());
        assert!(rt.take_patches().is_empty());
    }

    #[test]
    fn signal_change_produces_update_text_patch() {
        let s = Signal::new("before".to_owned());
        let s2 = s.clone();
        let mut rt = Runtime::new();
        rt.mount(move |cx| {
            let s3 = s2.clone();
            Text::new(move || s3.get()).into_element()
        });
        s.set("after".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        let found = patches.iter().any(|p| {
            matches!(p, Patch::UpdateText { content, .. } if content == "after")
        });
        assert!(found, "expected UpdateText with 'after'; got {patches:?} (len={})", patches.len());
    }

    #[test]
    fn signal_from_use_signal_persists_across_rerenders() {
        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut rt = Runtime::new();
        rt.mount(move |cx| {
            let count = cx.use_signal(0u32);
            count.update(|n| *n += 1);
            t2.get(); // track trigger signal to force re-render
            Text::new(move || count.get().to_string()).into_element()
        });
        // After mount: count is 1 (incremented on first render)
        trigger.set(1);
        rt.flush_effects();
        let patches = rt.take_patches();
        // count should now be 2 — same signal persisted via hook index
        let found = patches.iter().any(|p| {
            matches!(p, Patch::UpdateText { content, .. } if content == "2")
        });
        assert!(found, "expected UpdateText with '2'");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test runtime::tests 2>&1 | head -10
```

Expected: compile error — `Runtime`, `Signal` not in scope, etc.

- [ ] **Step 3: Implement Runtime**

```rust
pub mod cx;
pub mod derived;
pub mod effect;
pub mod observer;
pub mod signal;

use std::cell::RefCell;
use std::rc::Rc;

use crate::diff::{diff, NodePath, Patch};
use crate::element::Element;
use cx::Cx;
use effect::Effect;
use signal::Signal;

pub use signal::Signal;

type ComponentFn = Rc<dyn Fn(&Cx) -> Element>;

struct ComponentSlot {
    cx: Rc<RefCell<Cx>>,
    view: ComponentFn,
    previous: Option<Element>,
    pending: Rc<RefCell<Option<Element>>>,
    _effect: Effect,
}

pub struct Runtime {
    slots: Vec<ComponentSlot>,
    patch_queue: Vec<Patch>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime { slots: Vec::new(), patch_queue: Vec::new() }
    }

    pub fn mount(&mut self, f: impl Fn(&Cx) -> Element + 'static) {
        let cx = Rc::new(RefCell::new(Cx::new()));
        let view: ComponentFn = Rc::new(f);
        let pending: Rc<RefCell<Option<Element>>> = Rc::new(RefCell::new(None));

        // Initial render — no diff, just store the first tree
        let first_tree = {
            let mut cx_ref = cx.borrow_mut();
            cx_ref.reset_hooks();
            view(&*cx_ref)
        };

        // Reactive effect — re-runs whenever signals read inside view() change
        let view2 = Rc::clone(&view);
        let cx2 = Rc::clone(&cx);
        let pending2 = Rc::clone(&pending);

        let effect = Effect::new(move || {
            let mut cx_ref = cx2.borrow_mut();
            cx_ref.reset_hooks();
            let tree = view2(&*cx_ref);
            *pending2.borrow_mut() = Some(tree);
        });

        self.slots.push(ComponentSlot {
            cx,
            view,
            previous: Some(first_tree),
            pending,
            _effect: effect,
        });
    }

    /// Apply any pending re-renders: diff old vs new tree, accumulate patches.
    pub fn flush_effects(&mut self) {
        for slot in &mut self.slots {
            if let Some(new_tree) = slot.pending.borrow_mut().take() {
                if let Some(old_tree) = slot.previous.take() {
                    let patches = diff(old_tree, new_tree, NodePath::root());
                    // Reconstruct current tree for next diff by re-running view
                    let current = {
                        let mut cx_ref = slot.cx.borrow_mut();
                        cx_ref.reset_hooks();
                        (slot.view)(&*cx_ref)
                    };
                    slot.previous = Some(current);
                    self.patch_queue.extend(patches);
                }
            }
        }
    }

    pub fn take_patches(&mut self) -> Vec<Patch> {
        std::mem::take(&mut self.patch_queue)
    }
}

impl Default for Runtime {
    fn default() -> Self { Self::new() }
}
```

**Note on flush_effects:** After diffing, the slot re-runs the view function once more to get a fresh `previous` for the next diff. This is necessary because `diff()` consumes the old tree. An alternative is to make `Element` cloneable (deferred — closures block `Clone`).

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test runtime::tests
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add src/runtime/mod.rs
git commit -m "feat(runtime): add Runtime with ComponentSlot and patch queue"
```

---

### Task 11: Public API + Integration Tests

**Files:**
- Modify: `src/lib.rs`

Wire public re-exports and write end-to-end tests covering the full signal → component → diff → patch flow.

- [ ] **Step 1: Write the integration tests**

Replace `src/lib.rs` with:

```rust
pub mod diff;
pub mod element;
pub mod runtime;

pub use element::builders::{Box_, Button, Column, Row, Text};
pub use element::style::{Color, StyleProps};
pub use runtime::cx::Cx;
pub use runtime::signal::Signal;
pub use runtime::Runtime;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::Patch;

    #[test]
    fn counter_increments_produce_update_text_patches() {
        let count = Signal::new(0i32);
        let c = count.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let c2 = c.clone();
            Column::new()
                .child(Text::new(move || format!("Count: {}", c2.get())))
                .into_element()
        });

        assert!(rt.take_patches().is_empty(), "no patches on first mount");

        count.set(1);
        rt.flush_effects();

        let patches = rt.take_patches();
        assert!(!patches.is_empty());
        let has_patch = patches.iter().any(|p| {
            matches!(p, Patch::UpdateText { content, .. } if content == "Count: 1")
        });
        assert!(has_patch, "expected UpdateText 'Count: 1'");

        count.set(2);
        rt.flush_effects();
        let patches = rt.take_patches();
        let has_patch = patches.iter().any(|p| {
            matches!(p, Patch::UpdateText { content, .. } if content == "Count: 2")
        });
        assert!(has_patch, "expected UpdateText 'Count: 2'");
    }

    #[test]
    fn conditional_child_produces_insert_and_remove_patches() {
        let show = Signal::new(false);
        let s = show.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let visible = s.get();
            let mut col = Column::new();
            if visible { col = col.child(Text::new("visible")); }
            col.into_element()
        });

        assert!(rt.take_patches().is_empty());

        show.set(true);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches.iter().any(|p| matches!(p, Patch::InsertChild { .. })),
            "showing child must produce InsertChild");

        show.set(false);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches.iter().any(|p| matches!(p, Patch::RemoveChild { .. })),
            "hiding child must produce RemoveChild");
    }

    #[test]
    fn multiple_signals_each_trigger_patch() {
        let name = Signal::new("Alice".to_owned());
        let age  = Signal::new(30u32);
        let n = name.clone();
        let a = age.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let n2 = n.clone();
            let a2 = a.clone();
            Column::new()
                .child(Text::new(move || n2.get()))
                .child(Text::new(move || a2.get().to_string()))
                .into_element()
        });

        name.set("Bob".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches.iter().any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Bob")));

        age.set(31);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches.iter().any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "31")));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test tests 2>&1 | head -20
```

Expected: compile errors for any missing re-exports or type mismatches.

- [ ] **Step 3: Fix any compile errors**

Run `cargo build` and fix any errors. Common issues:
- `Signal` re-exported from `runtime::mod` conflicts with `runtime::signal::Signal` — use `pub use runtime::signal::Signal` in `lib.rs` directly.
- `Patch` variants need `#[allow(dead_code)]` if not all are used in tests — add to `diff/mod.rs`.

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: all tests pass (unit tests from all tasks + integration tests).

```
test result: ok. N passed; 0 failed; 0 ignored
```

- [ ] **Step 5: Final commit**

```bash
git add src/lib.rs
git commit -m "feat: wire public API and integration tests for lemon core runtime"
```

---

## Self-Review

### Spec coverage check

| Spec section | Tasks |
|---|---|
| Signal\<T\> (get, set, update, subscriber notification) | Task 3 |
| Derived\<T\> (cached computed, stale on dep change) | Task 4 |
| Effect (run immediately, re-run on dep change, drop stops it) | Task 5 |
| Cx (use_signal, use_memo, use_effect, hook index stability) | Task 6 |
| Element enum (Text, Box_, Row, Column, Button, Image, Component, Fragment, None) | Task 7 |
| StyleProps, PaintProps, PaintData, ColorSource, TextContent, TextStyle | Task 7 |
| Fluent builder API (Column, Row, Box_, Text, Button) | Task 8 |
| Diff algorithm — UpdateText, UpdateStyle, UpdatePaint, ReplaceNode | Task 9 |
| Diff algorithm — InsertChild, RemoveChild (unkeyed) | Task 9 |
| Runtime — mount, flush_effects, take_patches | Task 10 |
| Cx hook index stable across re-renders | Task 6 + Task 10 test |
| PatchQueue batch (never mid-event-handler) | Task 10 (flush_effects is called explicitly) |

**Gaps (deferred to Plan 2 or noted):**
- **Keyed children diff** (`MoveChild`) — unkeyed diff is implemented; keyed diff is a follow-on optimization.
- **ComponentElement diff** — `Component::new` builder exists as a type but the Runtime only handles single top-level components. Nested component mounting is Plan 2.
- **Layers 5–8** — Retained Tree, Taffy layout, Vello paint, winit platform — covered in Plan 2.

### Type consistency

- `PaintProps.resolve() → PaintData` used consistently in `diff_box` and `diff` for Button ✓
- `TextContent.resolve() → String` used consistently in Text and Button diff ✓
- `diff(old: Element, new: Element) → Vec<Patch>` takes owned values; callers (Runtime) always have owned trees ✓
- `NodePath::child(index)` used consistently in `diff_children` ✓
- `BoxElement` used for `Column`, `Row`, and `Box_` variants — builders use the macro; diff matches on each variant separately ✓

---

## Plan 2 Preview

Plan 2 will implement Layers 5–8:

- **Retained Tree** — live nodes with `taffy::NodeId`, `PaintData`, `EventHandlers`, `TextCache`
- **Patch application** — `apply_patches(patches, retained, taffy)` mutates the retained tree
- **Layout Pass** — `taffy.compute_layout_with_measure()` with Parley text measure callback
- **Paint Pass** — pre-order walk of retained tree → Vello `Scene` commands with HiDPI transform
- **Platform** — winit `ApplicationHandler`, wgpu surface, Vello renderer, event hit-test
- **`lemon::run()`** — the final entry point that boots a real window

Plan 2 requires a GPU. Verification is via `cargo run` with a reference app.
