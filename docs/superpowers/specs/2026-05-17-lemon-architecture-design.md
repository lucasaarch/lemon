# Lemon — Arquitetura Completa do Framework

**Data:** 2026-05-17  
**Status:** Aprovado

## Visão Geral

Lemon é um toolkit de UI nativa para Rust com foco em aplicações desktop (macOS, Windows, Linux). O modelo de programação é **Reactive Retained Mode com Virtual DOM Style**: o usuário descreve UI com funções e builders fluentes; o runtime atualiza só o que mudou.

Stack físico: winit (windowing) + wgpu (GPU) + Vello (2D vector rendering) + Taffy (flexbox layout) + Parley (text shaping).

---

## Modelo de Programação

**Signals** são a primitiva de estado. Componentes são funções que leem signals e retornam uma árvore de elementos via builders fluentes. Quando um signal muda, o componente re-executa; o resultado é diffado contra a árvore anterior e só as diferenças são aplicadas.

```rust
fn counter(cx: &Cx) -> Element {
    let count = cx.use_signal(0i32);

    Column::new()
        .gap(12.0)
        .child(Text::new(move || count.get().to_string()))
        .child(
            Button::new("Incrementar")
                .on_click(move |_| count.update(|n| *n += 1))
        )
        .into_element()
}

fn main() {
    lemon::run(
        WindowConfig::default().title("App").size(900.0, 600.0),
        counter,
    );
}
```

---

## Camadas

```
┌─────────────────────────────────────────────────────────┐
│  1. Reactive Runtime       Signal<T>, Derived<T>, Effect │
├─────────────────────────────────────────────────────────┤
│  2. Component Model        fn(Cx) → Element, use_signal │
├─────────────────────────────────────────────────────────┤
│  3. Element Tree           Virtual tree (imutável)       │
├─────────────────────────────────────────────────────────┤
│  4. Diff + Patch           old ↔ new → lista de patches │
├─────────────────────────────────────────────────────────┤
│  5. Retained Tree          Nós vivos: Taffy + PaintData  │
├─────────────────────────────────────────────────────────┤
│  6. Layout Pass            Taffy → posições + tamanhos   │
├─────────────────────────────────────────────────────────┤
│  7. Paint Pass             Vello scene commands          │
├─────────────────────────────────────────────────────────┤
│  8. Platform               winit + wgpu + GPU present    │
└─────────────────────────────────────────────────────────┘
```

Camadas 1–4 são puras (sem GPU/OS) — testáveis com `#[test]` normal.  
Camadas 5–8 são o "mundo real" — só rodam com uma janela de verdade.  
O usuário do Lemon nunca toca nas camadas 5–8 diretamente.

---

## Camada 1 — Reactive Runtime

### Primitivas públicas

**`Signal<T>`** — estado mutável reativo.

```rust
let count = Signal::new(0i32);
count.set(5);
count.update(|n| *n += 1);
let v = count.get(); // registra dependência se dentro de um observer
```

Implementado como `Rc<SignalInner<T>>`. `get()` registra o caller como subscriber se há um observer ativo no thread-local stack. `set()` grava o valor e enfileira todos os subscribers para re-execução.

**`Derived<T>`** — valor computado cacheado.

```rust
let doubled = Derived::new(move || count.get() * 2);
```

Roda a closure na primeira leitura, registrando dependências. Cacheia o resultado; recomputa só se uma dependência mudou. Subscribers são notificados quando o valor calculado muda.

**`Effect`** — efeito colateral reativo.

```rust
Effect::new(move || println!("{}", count.get()));
```

Re-executa automaticamente quando qualquer signal lido muda. Usado internamente pelo Component Model para agendar re-renders. O usuário acessa via `cx.use_effect(...)`.

### Mecanismo de tracking (observer stack)

```
thread_local: Vec<WeakSubscriber>

signal.get():
  se stack não vazio → registra self como dep do topo

signal.set(v):
  self.value = v
  para cada subscriber → enfileira re-execução

Effect/Derived ao rodar:
  push(self) → executa closure → pop(self)
  deps capturadas durante execução = novas dependências
```

Todo single-threaded (winit exige). Sem `Arc`, sem `Mutex` no runtime.

### `Cx` — contexto de componente

- `cx.use_signal(init)` — signal com vida útil ligada ao componente.
- `cx.use_memo(f)` — `Derived` com vida útil ligada ao componente.
- `cx.use_effect(f)` — `Effect` que roda após o paint.

Signals/effects criados via `cx` são dropped quando o componente é desmontado.

---

## Camada 2 — Component Model

### Definição

Um componente é uma função Rust comum:

```rust
fn greeting(cx: &Cx, props: GreetingProps) -> Element { ... }
```

Sem trait, sem struct, sem macro obrigatória. Componentes com props externos recebem uma struct:

```rust
struct GreetingProps { name: Signal<String> }
```

Composição via `Component::new`:

```rust
Component::new(greeting, GreetingProps { name: name_signal })
```

### Ciclo de vida

1. **Montagem** — view fn executa pela primeira vez dentro de um `Effect`; Element tree inserido no Retained Tree sem diff; `use_effect` callbacks agendados para após o primeiro paint.
2. **Atualização** — signal lido na última execução muda → Effect re-executa → novo Element tree diffado → patches aplicados.
3. **Desmontagem** — nó removido pelo diff do pai → signals/effects do `cx` são dropped → Taffy node removido.

### Estabilidade de componentes filhos

`Component::new` tem identidade por **tipo de função + key opcional**. O mesmo componente não é desmontado/remontado só porque o pai re-renderizou. O runtime compara tipo + key antes de decidir montar ou apenas atualizar props.

### Keys para listas dinâmicas

```rust
for item in items.get().iter() {
    Component::new(list_item, item.clone()).key(item.id)
}
```

Sem key → diff por índice. Com key → diff por identidade (preserva estado mesmo se reordenado).

---

## Camada 3 — Element Tree

### Enum `Element`

```rust
pub enum Element {
    Text(TextElement),
    Box(BoxElement),
    Row(BoxElement),
    Column(BoxElement),
    Button(ButtonElement),
    Image(ImageElement),
    Component(ComponentElement),
    Fragment(Vec<Element>),
    None,
}
```

Widgets customizados são composições de componentes — não há variant extensível no enum.

### Structs concretas

```rust
pub struct BoxElement {
    pub style: StyleProps,
    pub paint: PaintProps,
    pub children: Vec<Element>,
    pub key: Option<Key>,
}

pub struct TextElement {
    pub content: TextContent,   // String estática ou Fn() → String
    pub style: TextStyle,
    pub key: Option<Key>,
}

pub struct ButtonElement {
    pub label: String,
    pub style: StyleProps,
    pub paint: PaintProps,
    pub on_click: Option<EventHandler>,
    pub key: Option<Key>,
}
```

### `StyleProps`

Mapeamento direto para estilos Taffy:

```rust
pub struct StyleProps {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Edges<Dimension>,
    pub margin: Edges<Dimension>,
    pub gap: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub align_items: Option<Align>,
    pub justify_content: Option<Justify>,
}
```

### `TextContent`

```rust
pub enum TextContent {
    Static(String),
    Dynamic(Box<dyn Fn() -> String>),
}
```

`impl Into<TextContent>` é implementado para `&str`, `String`, e `Fn() -> String`. A closure de `Dynamic` é avaliada pelo diff quando o componente re-executa — o resultado é comparado com o valor anterior e emite `UpdateText` se mudou.

### `PaintProps` vs `PaintData`

`PaintProps` (no Element Tree) pode conter `ColorSource` — um enum que aceita tanto `Color` estático quanto `Fn() -> Color` reativo. Quando o diff compara dois Elements, avalia as closures de `PaintProps` e produz `PaintData` (valores concretos resolvidos) para inserir no patch. O Retained Tree armazena só `PaintData`.

```rust
pub enum ColorSource {
    Static(Color),
    Dynamic(Box<dyn Fn() -> Color>),
}

pub struct PaintProps {
    pub background: Option<ColorSource>,
    pub border_color: Option<ColorSource>,
    pub border_width: f32,
    pub radius: CornerRadii,
}
```

### Builders fluentes

```rust
Column::new()
    .width(300.0)
    .gap(8.0)
    .padding(16.0)
    .background(Color::SURFACE)
    .radius(12.0)
    .child(Text::new("Saldo").font_size(11.0).color(Color::MUTED))
    .child(Text::new(move || format!("R$ {:.2}", balance.get())).font_size(24.0))
```

Propriedades reativas são closures: `Text::new(move || label.get())`. Os builders aceitam `impl Into<TextContent>` e `impl Into<ColorSource>` — traits implementadas tanto para valores estáticos quanto para `Fn() → T`. As closures são armazenadas no Element e avaliadas pelo diff, não no build.

---

## Camada 4 — Diff + Patch

### Tipos de patch

```rust
pub enum Patch {
    UpdateStyle      { node: NodePath, style: StyleProps },
    UpdatePaint      { node: NodePath, paint: PaintProps },
    UpdateText       { node: NodePath, content: String },
    ReplaceNode      { node: NodePath, new: Element },
    InsertChild      { parent: NodePath, index: usize, element: Element },
    RemoveChild      { parent: NodePath, index: usize },
    MoveChild        { parent: NodePath, from: usize, to: usize },
    MountComponent   { node: NodePath, component: ComponentElement },
    UnmountComponent { node: NodePath },
}
```

`NodePath` é uma sequência de índices do root ao nó (`[0, 2, 1]`).

### Algoritmo

Recursivo, O(n) para árvores sem listas reordenadas:

1. Tipos diferentes → `ReplaceNode` (sem descer).
2. Mesmo tipo:
   - Compara `StyleProps` campo a campo → `UpdateStyle` se diferente.
   - Avalia closures de `PaintProps` (old e new), compara `PaintData` resultante → `UpdatePaint` se diferente.
   - Para `Text`: avalia `TextContent` (old e new), compara strings → `UpdateText` se diferente.
3. Filhos sem keys → diff por índice; filhos com keys → diff keyed (map por key, detecta inserções/remoções/moves).
4. `ComponentElement`: mesmo tipo de função + mesma key → componente sobrevive; diferente → `UnmountComponent` + `MountComponent`.

### Acionamento

Cada componente tem um `ComponentSlot` no runtime que guarda o último `Element` produzido. Quando o `Effect` do componente re-executa:

```
new_tree = view_fn(cx)
patches  = diff(&slot.previous, &new_tree, slot.path)
slot.previous = new_tree
patch_queue.push_all(patches)
```

Patches são acumulados e aplicados em batch antes do próximo frame — nunca no meio de um event handler.

---

## Camada 5 — Retained Tree

### Estrutura de nó

```rust
pub struct RetainedNode {
    pub kind: RetainedKind,
    pub taffy_id: taffy::NodeId,
    pub style: StyleProps,
    pub paint: PaintData,
    pub children: Vec<RetainedNode>,
    pub handlers: EventHandlers,
    pub text: Option<TextCache>,
}

pub struct PaintData {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: CornerRadii,
}

pub struct TextCache {
    pub content: String,
    pub style: TextStyle,
    pub parley_layout: Option<parley::Layout<Brush>>,  // None = recompute necessário
}

pub struct EventHandlers {
    pub on_click:       Option<EventHandler>,
    pub on_hover_enter: Option<EventHandler>,
    pub on_hover_leave: Option<EventHandler>,
    pub on_key_down:    Option<EventHandler>,
}
```

### Aplicação de patches

| Patch | Ação |
|---|---|
| `UpdateStyle` | `node.style = style`; `taffy.set_style(taffy_id, style.into())` |
| `UpdatePaint` | `node.paint = paint` (recebe `PaintData` já resolvido pelo diff) |
| `UpdateText` | `node.text.content = content`; invalida `parley_layout` |
| `InsertChild` | cria `RetainedNode`; `taffy.new_leaf()`; `taffy.insert_child()` |
| `RemoveChild` | `taffy.remove()` recursivo; remove de `children` |
| `MoveChild` | `taffy.move_child()`; swap em `children` |
| `ReplaceNode` | `RemoveChild` + `InsertChild` no mesmo índice |

### Componentes no Retained Tree

```rust
pub enum RetainedKind {
    Box, Row, Column, Text, Button, Image,
    Component { view_fn: ComponentFn, cx: Cx, effect_id: EffectId },
}
```

Nós `Component` são opacos para layout e paint — as camadas 6 e 7 descem diretamente pelos filhos concretos. O nó `Component` não tem `taffy_id` próprio; seu filho raiz ocupa o espaço no Taffy.

### Invariantes

1. Todo nó `Box/Row/Column/Button/Image` tem exatamente um `taffy_id` válido.
2. Todo `taffy_id` no Retained Tree está registrado no Taffy.
3. A ordem de `children` espelha a ordem de filhos no Taffy.
4. Se `text.parley_layout` é `Some`, bate com `text.content` e `text.style`.

---

## Camada 6 — Layout Pass

### Quando roda

```
apply_patches() → layout_dirty = true
window resized  → layout_dirty = true

frame tick:
  se layout_dirty:
    layout_pass(retained_root, viewport, scale_factor)
    layout_dirty = false
    paint_dirty  = true
```

### Passos

1. **Sync de estilos** — patches de `UpdateStyle` já chamaram `taffy.set_style()` na camada 5. O layout pass não sincroniza de novo.

2. **Measure callback para texto** — closure fornecida ao `taffy.compute_layout_with_measure()`. Usa Parley para medir o nó de texto dado `known_dims` e `available_space`. Resultado cacheado no `TextCache`; reutilizado se conteúdo, estilo, e max_width não mudaram.

3. **Coleta de rects absolutos** — percorre o Retained Tree acumulando offsets para converter posições relativas em absolutas:

```rust
fn collect_layouts(taffy, node, offset, map) {
    let layout = taffy.layout(node.taffy_id);
    let abs = Rect { x: offset.x + layout.location.x, y: offset.y + layout.location.y, ... };
    map.insert(node.taffy_id, abs);
    for child in &node.children {
        collect_layouts(taffy, child, abs.origin(), map);
    }
}
```

Nós `Component` são transparentes — `collect_layouts` desce direto nos filhos concretos.

### `LayoutMap`

```rust
pub struct LayoutMap {
    rects: HashMap<taffy::NodeId, Rect>,
}
```

Passado inteiro para o Paint Pass e para o hit-test de eventos.

### HiDPI

Layout trabalha em **pontos lógicos** do início ao fim. O `scale_factor` é passado ao Parley em cada `measure_text`. Conversão para pixels físicos ocorre só no paint via transform global do Vello.

---

## Camada 7 — Paint Pass

### Quando roda

```
frame tick:
  se paint_dirty:
    scene.reset()
    paint_pass(retained_root, &layout_map, &mut scene, scale_factor)
    paint_dirty = false
    → submit GPU
```

### Travessia

Pré-ordem (pai antes dos filhos) — garante que o background do pai é pintado antes dos filhos. Nós `Component` são transparentes: desce direto nos filhos.

### Paint por tipo

**Box / Row / Column:**
1. `scene.fill(RoundedRect, background)` se background definido.
2. `scene.stroke(RoundedRect, border_color, border_width)` se border definido.

**Button:** idem Box + label pintado como filho Text na iteração seguinte.

**Text:**
1. Se `parley_layout` é `None` → recomputa (Parley `build` + `break_lines`).
2. Para cada `GlyphRun`: `scene.draw_glyphs(font).brush(color).transform(origin).draw(glyphs)`.

### HiDPI

Transform global aplicado uma vez no topo da cena:

```rust
scene.push_layer(BlendMode::default(), 1.0, Affine::scale(scale_factor as f64), &Rect::EVERYTHING);
// todo o paint pass em pontos lógicos
scene.pop_layer();
```

### Z-order e Clipping

Z-order pela ordem da árvore (sem z-index na v1). Clipping (`overflow: hidden`) via `push_layer`/`pop_layer` — reservado para extensão futura; na v1 overflow é sempre visível.

---

## Camada 8 — Platform + Frame Loop

### Estado da aplicação

```rust
struct AppState {
    window:       Arc<Window>,
    render_cx:    RenderContext,
    surface:      RenderSurface<'static>,
    renderer:     vello::Renderer,
    scene:        Scene,
    runtime:      Runtime,
    retained:     RetainedNode,
    taffy:        TaffyTree,
    layout_map:   LayoutMap,
    font_cx:      FontContext,
    layout_dirty: bool,
    paint_dirty:  bool,
}
```

### Entry point

```rust
fn main() {
    lemon::run(
        WindowConfig::default().title("App").size(900.0, 600.0),
        root_component,
    );
}
```

`lemon::run` cria o `EventLoop`, monta o `AppState`, inicializa o runtime com o componente raiz (primeiro render sem diff), e entrega o controle ao winit.

### Ciclo de eventos (winit `ApplicationHandler`)

| Evento | Ação |
|---|---|
| `resumed` | cria Window + GPU surface + Vello renderer; monta componente raiz; `layout_dirty = paint_dirty = true`; `request_redraw()` |
| `RedrawRequested` | `flush_patches()` → layout pass (se dirty) → paint pass (se dirty) → GPU present |
| `Resized` | `resize_surface()`; `layout_dirty = true`; `request_redraw()` |
| `CursorMoved / MouseInput / KeyboardInput` | `event_pass()` → hit-test → dispatch handler → `flush_effects()` → `request_redraw()` se signals mudaram |
| `about_to_wait` | `request_redraw()` se dirty; senão dorme |

### Event pass (hit-test + dispatch)

Percorre o Retained Tree em pós-ordem (filho antes do pai) para acertar o nó mais à frente:

```rust
fn event_pass(event, layout_map, retained) {
    let pos = cursor_position(event);
    let target = hit_test(retained, layout_map, pos);  // pós-ordem
    if let Some(node) = target {
        match event_kind(event) {
            Click      => node.handlers.on_click.call(event),
            HoverEnter => node.handlers.on_hover_enter.call(event),
            KeyDown    => node.handlers.on_key_down.call(event),
        }
    }
}
```

### Garantia de frame ordering

```
event handler  → signal.set() → Effects agendados (não rodam ainda)
flush_effects() → todos os patches do frame acumulados
RedrawRequested → flush_patches() → layout → paint → GPU
```

Signals e patches são sempre resolvidos antes do frame começar. Nenhum frame exibe estado parcial.

---

## Fluxos de dados resumidos

**Atualização de estado:**
```
Signal.set(v)
  → notifica ComponentEffects dependentes
    → re-executa view fn → novo Element subtree
      → diff(old, new) → Patch list
        → apply patches → Retained Tree atualizado
          → mark_dirty() → próximo frame: Layout → Paint → GPU
```

**Input do usuário:**
```
winit::Event
  → hit-test nos rects do Retained Tree
    → dispara EventHandler
      → handler atualiza Signal
        → (entra no fluxo acima)
```

---

## O que fica fora da v1

- Multi-window
- Z-index explícito
- Overflow / clipping
- Animações / transições
- Acessibilidade (a11y)
- Temas / design tokens formais
- Widgets de input (TextInput, Select, Slider)
- Imagens e assets

---

## Implementation Status

**Last updated:** 2026-05-17  
**Tracker:** [GitHub #14](https://github.com/lucasaarch/lemon/issues/14) (issues `#1`–`#13`, label `lemon-roadmap`)  
**Execution order:** `docs/superpowers/ROADMAP.md`

### By layer

| Layer | Spec section | Status | Plan / notes |
|-------|----------------|--------|----------------|
| 1 | Reactive runtime | **Done** | `plans/2026-05-17-lemon-core-runtime.md` |
| 2 | Component model | **Done** (gaps below) | `plans/2026-05-17-lemon-component-lifecycle.md` |
| 3 | Element tree | **Done** | core-runtime plan |
| 4 | Diff + patch | **Partial** | Keyed `MoveChild` not implemented yet |
| 5 | Retained tree | **Done** | retained-tree + component-lifecycle |
| 6 | Layout pass | **Not started** | `plans/2026-05-17-lemon-layout-pass.md` |
| 7 | Paint pass | **Not started** | `plans/2026-05-17-lemon-paint-pass.md` |
| 8 | Platform + `lemon::run` | **Not started** | `plans/2026-05-17-lemon-platform.md` |

### Spec alignment notes (implemented code)

| Topic | Spec | Implementation |
|-------|------|----------------|
| Component identity | Function type + `key` | Function pointer `identity()` + `key` (distinguishes distinct fns) |
| Component props | `Component::new(fn, Props)` | `Component::new(fn)` — props via Rust closures |
| Retained `Component` node | Stores `view_fn`, `cx`, `effect_id` | Metadata only (`type_id`, `key`); runtime owns `Cx` / `Effect` |
| `taffy_id` on all nodes | Required | `Option<NodeId>`; `None` for component wrappers |
| Component patch on same identity | Mount / unmount only | Also `UpdateComponent` to swap `view` |
| `use_effect` timing | After first paint | Runs on mount today — deferred queue planned |
| `Derived` notify | Only when value changes | Notifies on any dependency change — equality planned |
| `EventHandlers` | click, hover, key | **click only** in retained |
| `TextCache.parley_layout` | Cached Parley layout | Field not yet present — layout plan |

### Verification gates

| Milestone | Command |
|-----------|---------|
| Pure core (current) | `cargo test` (68 tests) |
| After layout + paint | `cargo test` |
| First window | `cargo run --example counter` |
| Per change | `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test` |
