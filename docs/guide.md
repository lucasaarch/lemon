# Application guide

For installation and runnable applications, see the [README](../README.md).
For every public item, consult the [API reference](https://docs.rs/lemon).

## Application API

Lemon describes the intent of a view through fluent builders:

- `cx.state(initial)` declares shared component state; `cx.derive(...)` computes comparable data from it.
- `cx.watch(...)` runs a callback after mount and whenever its reactive reads change.
- `cx.motion(...)` declares an animation; `cx.theme()` reads the originating host's theme.
- `.fill()` follows the parent content size (the viewport for a root view); `.fill_width()` and `.fill_height()` select an axis.
- `.center()` and `.place(Placement::BottomRight)` position children in physical directions, the same for rows and columns.
- `.spacing(...)` separates children; `.inset(...)` adds inner space; `.outset(...)` adds outer space.
- `.surface(...)`, `.rounding(...)`, `.gradient(...)`, `.shadow(...)`, and `.layer(...)` decorate a container.
- Text uses `.size(...)`, `.font(...)`, `.leading(...)`, and `.color(...)`; primary activation uses `.action(...)`.

Component resources still require stable declaration order across renders. Widget constructors
also declare state, so put dynamic widget instances in component scopes rather than changing
loops on a shared `Cx`. `State<T>` clones share a value; element trees are built during rendering,
not cached with `cx.derive(...)`.

### Shared decoration

Every builder and custom widget implementing `Into<Element>` supports the same
root styling through `Decorate` (included in the prelude):

```rust
use lemon::prelude::*;

fn field(view: View) -> View {
    view.width(300.0).inset(12.0).rounding(10.0)
}

fn login(cx: &Cx) -> Element {
    let name = cx.state("".to_owned());
    let password = cx.state("".to_owned());
    ColumnView::new().spacing(12.0)
        .child(TextField::new(cx, name).placeholder("Name").decorate(field))
        .child(SecureField::new(cx, password).placeholder("Password").decorate(field))
        .into_element()
}
```

Decoration modifies the existing root after widget defaults; explicit changes win.
Containers preserve their kind, keys, handlers and children. Text, raw leaves and
components receive a surrounding view. Widget-specific `.style(...)` still controls
internal parts such as a slider fill or select dropdown. See `custom` for a runnable example.

## System appearance and custom themes

`run(|cx| ...)` follows the native light/dark appearance, both
when the window opens and when the system changes it. Widget defaults, implicit
text colors, and the window background update together without resetting component
state. Read semantic tokens with `cx.theme()` instead of hardcoding a light/dark
surface and relying on an unrelated foreground color.

The built-in palettes live in [`src/theme.rs`](../src/theme.rs): edit
`Theme::default_dark()` or `Theme::default_light()` to change Lemon's defaults.
For application-specific changes, load a named `ThemePair` with
`cx.load_theme("brand", themes)` and select it with `cx.set_theme("brand")`. Its `colors` controls fills, text and borders; `chrome` controls
focus rings, carets and scrollbars; `typography`, `spacing` and `radius` control
the remaining shared defaults.

Applications can supply their own light/dark pair using the same entry point:

```rust
use lemon::{Theme, ThemePair};
use lemon::prelude::*;

fn app(_cx: &Cx) -> Element {
    Text::new("Hello").into_element()
}

let mut light = Theme::default_light();
let mut dark = Theme::default_dark();
light.radius.md = 10.0;
dark.radius.md = 10.0;
dark.colors.background = Color::rgb8(24, 24, 24);
dark.colors.surface = Color::rgb8(36, 36, 36);
run(move |cx| {
    cx.load_theme("brand", ThemePair::new(light, dark));
    cx.set_theme("brand").unwrap();
    cx.open_window(WindowConfig::default(), app);
});
```

Loading a single `Theme` keeps that palette fixed. Loading a `ThemePair` follows system appearance.
Loading a theme does not select it; the default active name is `"lemon"`. Unknown
names return `ThemeNotFound` without changing the current theme. Local widget overrides continue to take precedence.
Native window decorations retain the system appearance. Choose chrome through
`WindowConfig::default().chrome(WindowChrome::Overlay)` to extend content behind
the title bar while keeping traffic lights on macOS, or `WindowChrome::Borderless`
to remove title bar and buttons. On Windows and Linux both custom modes remove
native decorations and buttons; behavior of shadows and resizing depends on the
window manager. The default is `WindowChrome::System`. Reserve space for macOS
traffic lights in overlay layouts; Lemon does not insert an extra content inset.
To position the native macOS group, add `.traffic_lights_position(20.0, 20.0)`
to `WindowConfig` with `WindowChrome::Overlay`. Coordinates are logical points
from the client area's top-left to the close button's top-left. Native spacing
and actions stay intact; the group stays inside the available window space on
resize. Other platforms and chrome modes ignore this macOS-only option.


Detection and live change events use winit's macOS and Windows support. When the
platform does not report an appearance (including the current X11/Wayland API),
Lemon uses the dark palette. `Theme::default()` remains a deterministic dark palette
for headless use; it does not query the OS. See the [winit platform limits](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.theme).

## Primitive-First Customization

Base builders and controls such as `ScrollView`, `Button`, `Select`, `Slider`, `TextField`, `SecureField`, and `TextArea` cover common
application UI, and their defaults come from the active `Theme`. Start there when the widget shape
matches your product. Use `cx.load_theme` and `cx.set_theme` for app-wide tokens and widget style types such as
`SelectStyle` and `SliderStyle` for local overrides, and `ButtonStyle` / `TextFieldStyle` for ready-made appearances.

`TextField` and `SecureField` accept `State<String>`; cursor and selection stay internal.
`SecureField` shares `TextFieldStyle`, displays bullets, and
blocks clipboard copy/cut shortcuts. Its string state retains the original value for app code.

`Checkbox::new(cx, value)` and `Switch::new(cx, value)` bind to `State<bool>`;
`.label(...)` and `.disabled(...)` configure their shared interaction behavior.
`Popover::new(cx, open, anchor, content)` and `Dialog::new(cx, open, content)`
bind visibility to `State<bool>`. Keep them mounted while closed for exit fades.
Dialogs block background input, contain Tab traversal and restore focus when
closed. Popovers dismiss on outside presses or Escape from within the control.
Use `ScrollView` inside a dialog for content taller than the viewport.

`Toaster::new(cx)` declares a component-owned queue. Mount one clone and capture
another in actions: `toaster.show("Saved")`. Notifications fade and expire after
four seconds; `show_for`, `dismiss` and `clear` control their lifetime. These
panels and notifications render in the Lemon window; they are not OS dialogs or
system notifications. See the `tasks` and `preferences` applications.

When a widget almost fits but the visual structure does not, rebuild the control from primitives:
`View`, `ColumnView`, `RowView`, `Text`, `Button`, `State`, and event handlers. This keeps the app in the
normal Lemon pipeline while giving you control over layout, paint, hover/press state, and copy.
For repeated dynamic rows, give siblings stable `.key(id)` values so diffing preserves identity.
For colors that depend on live state, pass a closure to paint builders such as `.surface(...)`;
the closure is stored as a dynamic `ColorSource` and is resolved as the tree is frozen.

Use this path when the customization is mostly visual or structural. Prefer the built-in widget
plus theme/style overrides when the interaction model is the same. Fork or write a custom widget
when you need to own identity, overlay positioning, pointer capture, keyboard handling, or internal
state transitions as a reusable app abstraction.

Current limits live at the paint layer. Widget-managed details such as scrollbars and text-input
carets are painted by Lemon's retained widget metadata, not by arbitrary app paint hooks. If those
details need brand-specific behavior beyond existing style overrides, use a custom primitive
composition where possible and track the relevant roadmap work:
[#40](https://github.com/lucasaarch/lemon/issues/40) for theme tokens,
[#42](https://github.com/lucasaarch/lemon/issues/42) for theme-backed widget defaults,
[#92](https://github.com/lucasaarch/lemon/pull/92) for positioned select overlays, and
[#93](https://github.com/lucasaarch/lemon/pull/93) for scroll behavior polish.

### Positioning

Use `.position(Position::Relative | Absolute | Fixed | Sticky)` with `.top()`,
`.right()`, `.bottom()` and `.left()` offsets in logical points. Relative is the
default. Absolute uses the parent, fixed uses the window and escapes ancestor
clipping, and sticky stays in flow within its parent and nearest clipped region.
Unset edges remain automatic. Fixed percentage dimensions use the window size;
opposing edges stretch an otherwise auto-sized fixed view.

```rust
use lemon::{Distribute, Position, RowView, View};
let row = RowView::new().fill_width().distribute(Distribute::Between);
let badge = View::new().position(Position::Fixed).top(16.0).right(16.0)
    .width(80.0).height(32.0);
```

### Visual effects and events

Containers accept `.opacity(0.8)`, `.blur(4.0)` and `.brightness(1.2)`.
Use `.decorate(|view| ...)` to apply them to any widget. Effects include all
children, text and images without changing layout or hit geometry. Blur affects
content, uses a Gaussian standard deviation in logical points (capped at 1024),
and does not sample the background behind the view. Brightness preserves alpha:
`1` is unchanged, `0` is black, and values above `1` brighten.

```rust
use lemon::prelude::*;
let card = View::new().width(240.0).height(120.0)
    .gradient(RadialGradient::new((0.5, 0.5), 0.7,
        Color::rgb8(100, 160, 255), Color::rgb8(30, 40, 80)))
    .opacity(0.9).blur(2.0).brightness(1.1)
    .on_hover(|inside| println!("hover: {inside}"))
    .focusable().on_focus(|focused| println!("focus: {focused}"));
```

`LinearGradient::new(start, end, first, last)` and
`RadialGradient::new(center, radius, inner, outer)` use normalized coordinates;
radial radius is relative to the shorter side. Both support `.stops([...])` with
`GradientStop::new(position, color)` for multi-color fills. Stops are sorted,
with at least two positions in `0..=1`.

Hover includes descendants and sends `false` on exit, removal, disabled input or
window leave. Focus observes direct element focus, including loss on removal;
plain views need `.focusable()`, while controls already support focus. Existing
`.action`, `.on_hover_enter`, `.on_hover_leave`, keyboard and pointer events
remain available.

Opacity and filters isolate descendant layers into their own compositing group.
Blur and brightness use intermediate GPU textures; strong blurs and textures
larger than 4096 pixels are downsampled. Native hosts handle this automatically.
Custom renderers must use `lemon::Renderer` with `lemon::Scene` from
`UiState::scene()` rather than passing that scene to a raw Vello renderer.

### Visual transitions

Apply `.transition(...)` to a view, or use `.decorate(...)` on a widget. The setting
covers its subtree; a descendant can override it. Focus, hover, theme and reactive
color changes then animate without app-owned animation state:

```rust
use std::time::Duration;
use lemon::prelude::*;
fn field(cx: &Cx) -> Element {
    let value = cx.state(String::new());
    TextField::new(cx, value).decorate(|view| view.transition(
        Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut)
    ))
}
```

Supported properties: `Surface` (solid color), `BorderColor`, `FocusRing`,
`TextColor`, `Opacity`, `Rounding`, and `BorderWidth`. All are enabled by default;
`.properties([TransitionProperty::Surface, TransitionProperty::BorderColor])`
selects only those properties. Duration defaults to 150 ms with linear easing.
Initial mounting is immediate; interrupted transitions continue from the current
appearance. Selected properties restart together on a new target. Zero duration
snaps to the target. Layout dimensions, gradients, blur, brightness, shadows and images change
immediately. Pointer interaction always uses the current layout.

The `focus` application uses motion to schedule its timer UI. Implicit transitions can be configured separately.
Native windows schedule frames automatically; custom hosts use `UiState::update`
or `update_at` with a monotonic timestamp and honor their host's frame requests.

### Application startup

`run` calls its initialization closure once before the event loop starts. `AppCx`
loads shared resources and queues native windows; component `Cx` builds reactive
views and owns hooks. The context argument is inferred, so both can be named `cx`.
The closure may borrow local configuration. Window roots remain `'static`.

```rust
use lemon::prelude::*;
use lemon::ThemePair;
fn app(_cx: &Cx) -> Element { Text::new("Hello").font("Brand").into_element() }
run(|cx| {
    cx.load_font_file("Brand", "assets/Brand.ttf").expect("load font");
    // For bundled bytes: cx.load_font("Brand", bytes).unwrap();
    cx.load_theme("brand", ThemePair::default());
    cx.set_theme("brand").unwrap();
    cx.open_window(WindowConfig::default().title("App"), app);
});
```

Fonts loaded through `load_font(family, bytes)` or `load_font_file(family, path)`
are validated and inherited by all windows. Loading errors are returned to the
caller. Call `open_window` multiple times to start with multiple windows. Returning
without opening one exits immediately. The `run_with_*` entry points were removed.

### Additional windows

`run` receives one application initialization closure with `&mut AppCx`.
Use `cx.open_window(WindowConfig, root)` for every initial window; no window is created implicitly.
The component `Cx` also provides `open_window` for windows opened later. The same
initialization applies registered fonts and the application theme to every window.
Each request opens an independent window; closing the last window exits the app.
`WindowConfig` also accepts `.on_open(|| ...)`, `.on_close(|| ...)`,
`.on_focus(|active| ...)`, `.on_resize(|width, height| ...)` and `.on_move(|x, y| ...)`.
Sizes and positions use logical points; some window managers do not report movement.
`.on_close_requested(|| bool)` can veto a close by returning `false` (for example,
to show a save confirmation). `on_close` runs once after acceptance, before native
resources are dropped; forced OS destruction cannot be vetoed. Callbacks apply
to both main and newly opened windows and may update shared `State` values.

The `preferences` example includes a Settings button showing the same API:

```rust
use lemon::prelude::*;
fn settings(_cx: &Cx) -> Element { Text::new("Settings").into_element() }
fn app(cx: &Cx) -> Element {
    let windows = Cx::new();
    Button::new(cx, "Settings").action(move || {
        windows.open_window(WindowConfig::default().title("Settings").size(480.0, 360.0), settings);
    }).into_element()
}
```

`State` is the reactive cell type throughout the toolkit. Buttons and images render
through ordinary views and text; low-level controls use these same primitives.

### Built-in control styles

Fields and buttons accept theme-aware presets through `.style(...)`. They keep
matching typography and padding across light/dark appearances:

```rust
use lemon::prelude::*;
fn form(cx: &Cx) -> Element {
    let name = cx.state(String::new());
    ColumnView::new().spacing(12.0)
        .child(TextField::new(cx, name).style(TextFieldStyle::Bordered))
        .child(Button::new(cx, "Save").style(ButtonStyle::Filled))
        .into_element()
}
```

Both enums provide `Clean`, `Bordered`, and `Filled`. Fields default to `Bordered`;
buttons default to `Filled`. `Clean` removes the resting border and fill while
retaining focus feedback. `Filled` fields have a surface without a resting border;
`Filled` buttons use the accent with contrasting text. Explicit overrides apply
after the preset, regardless of call order; `.decorate(...)` runs last. For example,
use `.placeholder_color(...)` on a field or `.surface(...)` on a button for custom colors.
Buttons and fields include subtle hover/focus transitions, overridable through root decoration.

`Select` uses a neutral bordered trigger with separate label and indicator, selected
and hovered option feedback, and a shadowed overlay that fades in and out. Arrow
keys navigate, Enter/Space choose, Escape dismisses. The closed menu stays mounted
at opacity zero and rejects input immediately, including throughout the exit fade.
`View::interactive(false)` exposes this input behavior for custom overlays too.
Use `SelectStyle::default().dropdown_transition(transition)` to configure the fade;
zero duration disables it.

### Icons and file trees

Lucide's complete bundled catalog is available through `lemon::icons::lucide` (also
in the prelude). Icons use vector strokes with round joins and caps; no icon font
or runtime asset download is needed. Upstream licensing is in
[`licenses/lucide-LICENSE`](../licenses/lucide-LICENSE).

```rust
use lemon::prelude::*;

let icon = Icon::new(lucide::SETTINGS).size(18.0).stroke_width(1.5);
```

`FileTree` displays application-supplied files and folders with automatic icons,
hierarchy guides, selection, hover, and keyboard navigation. IDs must be stable
and unique throughout the entire tree. It does not scan the filesystem.

```rust
use lemon::prelude::*;

fn explorer(cx: &Cx) -> Element {
    FileTree::new(cx, vec![FileTreeNode::folder(1, "src", vec![
        FileTreeNode::file(2, "main.rs"),
    ])])
    .on_select(|id| println!("Selected {id}"))
    .decorate(|view| view.width(280.0))
    .into()
}
```

Use `.selection(State<Option<u64>>)` and `.expanded(State<BTreeSet<u64>>)` to
control selection and disclosure from your application. `FileTreeStyle` controls
row height, indentation, icon size and guide/hover/selection colors. Nodes can
supply an explicit `.icon(...)` and `.color(...)`. Wrap the tree in `ScrollView` for a
bounded viewport. See `cargo run --example editor` for workspace navigation.

To refresh the bundled catalog from the latest stable Lucide release:

```bash
python3 scripts/update_lucide.py
```

The updater queries GitHub's latest release, downloads that release's source,
converts its SVGs to Lemon vectors, copies the upstream license and formats the
Rust catalog with `rustfmt`. The generated header records the release version and
source URL. Python 3, `rustfmt` and network access are required; no Python packages
or manual checkout are needed. Download, conversion or formatting failures leave
the existing catalog intact. Offline updater tests are included in `make check`.

### Multiline fields and scroll views

`TextField`, `SecureField` and `TextArea` live together in `src/widget/field.rs`.
`TextArea` accepts `State<String>`, wraps text, scrolls vertically and follows its
caret while editing. Its resize handle changes height only and enforces a minimum:

```rust
use lemon::prelude::*;
let cx = Cx::new();
let notes = State::new(String::new());
let area = TextArea::new(&cx, notes).width(320.0)
    .height(140.0).min_height(80.0).resizable(true);
```

Use `.resizable(false)` for a fixed height. The defaults are 120 points high,
a 60-point minimum and resizing enabled. The family shares `TextFieldStyle` and
root `.decorate(...)` styling.

`ScrollView` is vertical by default. Select `ScrollAxis::Horizontal` for a strip,
or `ScrollAxis::Both` for a canvas. Trackpads preserve both axes, including diagonal
movement; Shift+wheel scrolls horizontally. Calling `.axis(...)` again replaces
its previous value. Scrollbars appear independently for overflowing axes.

```rust
use lemon::prelude::*;
let cx = Cx::new();
let canvas = ScrollView::new(&cx, View::new().width(800.0).height(600.0))
    .axis(ScrollAxis::Both).width(320.0).height(200.0);
```

For caller-owned positions, `ScrollView::with_offset` takes
`State<ScrollOffset>` (`ScrollOffset::new(x, y)`) instead of a scalar state.
Offsets use logical points from the content origin. Optional pre-layout estimates
use `.content_size(width, height)`; measured bounds determine the final limits.

Base composition and presentation (`View`, `RowView`, `ColumnView`, `Text`,
`Component`, `Icon`, `Image`, `ScrollView`) belong to `element/builders`. Semantic
controls (`Button`, fields, `Select`, `Slider`, `FileTree`) belong to `widget`.
This keeps construction separate from interaction policy while preserving the
runtime → diff → retained → layout → paint pipeline. Public imports remain on the
crate root and prelude. The old scroll and separate field module paths were removed.
