# Lemon — Post-V1 Roadmap (V0.2–V0.5)

**Status:** Approved  
**Date:** 2026-05-17  
**Scope:** Todos os itens fora do escopo da V1, organizados em milestones semânticos publicáveis no crates.io.

---

## Princípios

- Cada milestone é publicável no crates.io e passa em `cargo test --workspace` + `cargo clippy --workspace -- -D warnings` antes do release.
- Foco em tornar o Lemon utilizável para apps reais: interatividade e widgets primeiro.
- **`lemon` core** contém apenas primitivas de renderização e o runtime. **`lemon-widgets`** contém todos os widgets com semântica de produto.
- A11y e multi-window em V0.5 podem ser splitados em V0.6 se o milestone ficar grande demais.

---

## Fronteira core / widgets

### `lemon` (core — permanece)

- `Element` enum reduzido a primitivas de renderização: `Prim`, `TextNode`, `Component`, `Fragment`, `None`
- `StyleProps`, `PaintProps`, `EventHandlers`
- `Cx` e hooks (`use_signal`, `use_memo`, `use_effect`)
- Camadas 1–8: runtime reativo, diff+patch, retained tree, layout (Taffy), paint (Vello), platform (winit + wgpu)
- `lemon::run()`

### `lemon-widgets` (novo crate)

- Builders migrados do core: `Button`, `Row`, `Column`, `Text`, `Image`
- Widgets novos: `Scroll`, `TextInput`, `Select`, `Slider`
- Contexto de tema: `Theme`, `use_theme()`

Os widgets de `lemon-widgets` constroem sobre as primitivas do core — um `Button`, por exemplo, é um `Prim` com style, paint e `on_click`.

---

## V0.2 — DX Baseline + Events Foundation

**Objetivo:** Fundar a qualidade do projeto e a infraestrutura de eventos antes de adicionar features.

### Extração `lemon-widgets` *(pré-requisito de todo o resto)*

- Criar `crates/lemon-widgets/` com `lemon = { workspace = true }`
- Refatorar `Element` enum para primitivas (`Prim`, `TextNode`, `Component`, `Fragment`)
- Migrar builders `Button`, `Row`, `Column`, `Text`, `Image` para `lemon-widgets`
- Atualizar `examples/counter` para importar de `lemon-widgets`

### CI

- GitHub Actions workflow em `.github/workflows/ci.yml`
- Jobs: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- Roda em push e pull request

### Licença

- `license = "MIT"` em todos os `Cargo.toml` do workspace
- Arquivo `LICENSE` (MIT) no root do repositório

### Exemplos

- 2 novos exemplos além de `counter`:
  - Layout composto (card/form com múltiplos widgets)
  - Lista keyed dinâmica (demonstra `MoveChild` / `key`)

### Keyboard

- Platform layer processa `KeyboardInput` do winit
- Sistema de foco no retained tree: qual nó tem foco ativo
- Tab cycling entre nós focáveis
- Handlers `on_key_down(KeyEvent)` / `on_key_up(KeyEvent)` em `EventHandlers`

### Hover

- `CursorMoved` já processado na plataforma — adicionar estado de hover no retained tree
- Hit-test de hover pós-ordem a cada `CursorMoved`
- Callbacks `on_hover_enter` / `on_hover_leave` em `EventHandlers`
- Prop `cursor: CursorIcon` em `StyleProps` para mudar o cursor do sistema

---

## V0.3 — Real App Core

**Objetivo:** O que um app real precisa para ser funcional — campos editáveis e listas roláveis.

**Dependências:** Requer V0.2 completo (TextInput depende de Keyboard; Scroll depende de clipping).

### `overflow: hidden` / Clipping

- Campo `overflow: Overflow` em `StyleProps` (visível por padrão)
- Paint pass usa `push_layer` do Vello com clip rect quando `overflow: hidden`
- Pré-requisito do widget `Scroll`

### Scroll / Viewports Roláveis

- Widget `Scroll` em `lemon-widgets`
- Scroll offset interno como `Signal<f64>` (vertical; horizontal como extensão futura)
- Layout: conteúdo mede seu tamanho natural; viewport tem tamanho fixo
- Paint: conteúdo renderizado com `Affine::translate` dentro do clip rect
- Platform: `MouseWheel` do winit dispara atualização do offset
- Scrollbar visual: entra em V0.4 junto com Image/Z-index (não é pré-requisito para o scroll funcionar)

### TextInput

- Widget `TextInput` em `lemon-widgets`
- Props: `value: Signal<String>`, `on_change: Fn(String)`, `placeholder: &str`
- Cursor de texto posicionado via Parley; renderizado no paint pass
- Seleção de texto (click + drag; Shift+setas)
- Edição: caracteres inseridos via `on_key_down`, Backspace/Delete, Ctrl+A/C/V/X
- Foco visual (borda destacada quando focado)

---

## V0.4 — Rich Content

**Objetivo:** Conteúdo visual rico e widgets de seleção para apps que exibem dados reais.

**Dependências:** Requer V0.3 completo (Slider depende de hover do V0.2; Select depende de z-index).

### Image Paint

- `Element::Image` hoje é placeholder — wiring real no paint pass usando suporte de imagem do Vello
- `ImageData` no retained tree: textura wgpu carregada e cacheada
- `Image` builder em `lemon-widgets` aceita `ImageHandle`

### Assets / Carregamento de Imagens

- Tipo `ImageHandle` criado a partir de bytes (`from_bytes`) ou caminho (`from_path`)
- Cache interno no runtime (deduplication por handle)
- Carregamento síncrono inicial; async pode ser adicionado depois via `use_effect`

### Z-index Explícito

- Campo `z_index: i32` em `StyleProps` (padrão 0)
- Paint pass coleta nós com z-index não-zero e os renderiza em camadas ordenadas após os nós normais
- Pré-requisito do widget `Select` (popup precisa renderizar acima do restante da UI)

### Select

- Widget `Select<T>` em `lemon-widgets`
- Props: `value: Signal<T>`, `options: Vec<(T, &str)>`, `on_change: Fn(T)`
- Dropdown popup usa z-index para renderizar acima de tudo
- Fecha ao clicar fora (hit-test global)

### Slider

- Widget `Slider` em `lemon-widgets`
- Props: `value: Signal<f32>`, `min`, `max`, `step`, `on_change: Fn(f32)`
- Drag via mouse: `on_hover_enter` + mouse down + `CursorMoved`
- Track + thumb renderizados via primitivas `Prim`

---

## V0.5 — Platform & Polish

**Objetivo:** Robustez de plataforma, acessibilidade e qualidade de API.

**Dependências:** Requer V0.4 completo. A11y e multi-window podem ser splitados em V0.6 se necessário.

### Themes / Design Tokens

- Struct `Theme` com campos: `colors` (background, foreground, accent, error...), `typography` (font sizes, weights), `spacing` (padding/gap scales)
- Hook `use_theme() -> Theme` disponível em `Cx`
- Widgets de `lemon-widgets` leem o tema por padrão, sobrescrevível por prop
- `lemon::run()` aceita `Theme` opcional como parâmetro

### Animações / Transições

- Hook `use_animation(from, to, duration, easing) -> Signal<f32>` em `Cx`
- Integrado ao frame loop: enquanto animação ativa, runtime solicita redraw a cada frame
- Suficiente para: fade, slide, transições de cor e tamanho
- Easing: linear, ease-in, ease-out, ease-in-out (básico)

### Acessibilidade (a11y)

- Integração com [AccessKit](https://github.com/AccessKit/accesskit)
- Retained tree emite accessibility tree paralela sincronizada com cada frame
- Widgets de `lemon-widgets` declaram roles semânticos (button, textinput, etc.)
- Suporte a leitores de tela no macOS (VoiceOver), Windows (NVDA/JAWS), Linux (Orca)

### Multi-window

- `lemon::open_window(config, root_fn)` para janelas adicionais
- Cada janela tem seu próprio retained tree, surface wgpu e event loop
- Janelas compartilham o mesmo runtime reativo (signals podem cruzar janelas)

### Typed Props

- `Component::new(fn, Props)` com props tipadas genéricas
- Hoje props são capturadas via closures — este modelo continua válido para casos simples
- Typed props clarificam APIs de componentes reutilizáveis e permitem diff de props explícito

### Fragment no Retained Tree

- `Fragment` existe no element tree mas tem suporte parcial no retained tree
- Completar: patch apply, layout traversal e paint traversal lidam com Fragment corretamente em todos os casos
- Elimina edge cases em listas mistas de fragments e elementos

---

## Mapa de dependências

```
V0.1 (atual)
  └── V0.2: lemon-widgets + CI + licença + exemplos + Keyboard + Hover
        └── V0.3: clipping + Scroll + TextInput
              └── V0.4: Image + Assets + Z-index + Select + Slider
                    └── V0.5: Themes + Animations + a11y + Multi-window + Props + Fragment
```

Itens dentro de cada milestone também têm dependências internas explicitadas em cada seção.

---

## Crate sugerido para sequência de publish

```
V0.2: cargo publish -p lemon && cargo publish -p lemon-widgets
V0.3: cargo publish -p lemon-widgets  (lemon core pode não mudar)
V0.4: cargo publish -p lemon-widgets
V0.5: cargo publish -p lemon && cargo publish -p lemon-widgets
```

---

## Itens deliberadamente fora deste roadmap

Nenhum item do backlog do CONTEXT.md foi omitido. Todos os 18 itens estão mapeados acima.
