#!/usr/bin/env bash
# Creates linear GitHub issues for Lemon roadmap. Run from repo root.
set -euo pipefail
REPO="lucasaarch/lemon"
LABEL="lemon-roadmap,enhancement"

create_issue() {
  local num="$1"
  local title="$2"
  local body="$3"
  gh issue create --repo "$REPO" --title "$title" --label "$LABEL" --body "$body"
}

# Returns issue number from URL (last path segment)
issue_num() { basename "$1"; }

echo "Creating roadmap issues..."

URL=$(create_issue "01" "[01/13] Keyed child diff (MoveChild patches)" "$(cat <<'EOF'
## Goal
Implement keyed child diffing per architecture spec (Camada 4).

## Plan
`docs/superpowers/plans/2026-05-17-lemon-runtime-semantics.md` — Task 1

## Spec
- Children with `key` → diff by identity (reorder emits `MoveChild`, not remove+insert)
- Children without `key` → keep index-based diff

## Files
- `src/diff/mod.rs`

## Depends on
None (Phase 0 complete)

## Gate
```bash
cargo test diff::
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
```

## Blocks
Issue [02] is independent; [03] can proceed in parallel but recommended order is 01 → 02 → layout chain.
EOF
)")
I01=$(issue_num "$URL")
echo "Created #$I01"

URL=$(create_issue "02" "[02/13] Derived equality-aware subscriber notification" "$(cat <<EOF
## Goal
\`Derived<T>\` only marks subscribers dirty when the computed value actually changes (\`PartialEq\`).

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-runtime-semantics.md\` — Task 2

## Spec
Camada 1 — Derived caches result and notifies when **value** changes.

## Files
- \`src/runtime/derived.rs\`

## Depends on
#$I01 (recommended sequence; not a hard code dependency)

## Gate
\`cargo test runtime::derived::\`
EOF
)")
I02=$(issue_num "$URL")
echo "Created #$I02"

URL=$(create_issue "03" "[03/13] Layout: LayoutMap and transparent layout collection" "$(cat <<EOF
## Goal
Camada 6 — \`LayoutMap\`, \`layout_pass\` skeleton, \`collect_layouts\` with transparent \`Component\` nodes.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-layout-pass.md\` — Task 1

## Files
- \`src/layout/mod.rs\` (new)
- \`src/lib.rs\`

## Depends on
#$I02

## Gate
\`cargo test layout::tests::column_children_stack_vertically\`
EOF
)")
I03=$(issue_num "$URL")
echo "Created #$I03"

URL=$(create_issue "04" "[04/13] Layout: Parley text measurement in Taffy" "$(cat <<EOF
## Goal
Wire \`taffy::compute_layout_with_measure\` + Parley; extend \`TextCache\` per spec (\`parley_layout\`, \`needs_layout\`).

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-layout-pass.md\` — Task 2

## Files
- \`src/layout/mod.rs\`
- \`src/retained/mod.rs\`

## Depends on
#$I03

## Gate
\`cargo test layout::\`
EOF
)")
I04=$(issue_num "$URL")
echo "Created #$I04"

URL=$(create_issue "05" "[05/13] Layout: integrate layout_pass with retained patches" "$(cat <<EOF
## Goal
End-to-end: Runtime patches → \`RetainedTree::apply_patch\` → \`layout_pass\` → non-empty \`LayoutMap\`.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-layout-pass.md\` — Task 3

## Depends on
#$I04

## Gate
\`cargo test\` (full suite)
EOF
)")
I05=$(issue_num "$URL")
echo "Created #$I05"

URL=$(create_issue "06" "[06/13] Paint: container backgrounds and paint_pass skeleton" "$(cat <<EOF
## Goal
Camada 7 — \`paint_pass\`, Vello scene fills for Box/Row/Column backgrounds.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-paint-pass.md\` — Task 1

## Files
- \`src/paint/mod.rs\` (new)
- \`src/lib.rs\`

## Depends on
#$I05

## Gate
\`cargo test paint::\`
EOF
)")
I06=$(issue_num "$URL")
echo "Created #$I06"

URL=$(create_issue "07" "[07/13] Paint: borders and text glyphs" "$(cat <<EOF
## Goal
Border strokes + glyph emission from cached Parley layout in \`TextCache\`.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-paint-pass.md\` — Task 2

## Depends on
#$I06

## Gate
\`cargo test paint::\`
EOF
)")
I07=$(issue_num "$URL")
echo "Created #$I07"

URL=$(create_issue "08" "[08/13] Paint: button, component transparency, HiDPI layer" "$(cat <<EOF
## Goal
Button paint, skip transparent \`Component\` nodes, root HiDPI \`push_layer\` transform per spec.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-paint-pass.md\` — Task 3

## Depends on
#$I07

## Gate
\`cargo test\` — layout + paint integration without panic
EOF
)")
I08=$(issue_num "$URL")
echo "Created #$I08"

URL=$(create_issue "09" "[09/13] Platform: WindowConfig and AppState skeleton" "$(cat <<EOF
## Goal
Camada 8 — \`WindowConfig\`, \`AppState\` struct per spec, \`platform\` module skeleton.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-platform.md\` — Task 1

## Files
- \`src/platform/mod.rs\`, \`src/platform/window.rs\`

## Depends on
#$I08

## Gate
\`cargo check\`
EOF
)")
I09=$(issue_num "$URL")
echo "Created #$I09"

URL=$(create_issue "10" "[10/13] Platform: wgpu surface and Vello renderer bootstrap" "$(cat <<EOF
## Goal
Open a real window: winit + wgpu surface + \`vello::Renderer\` on \`resumed\`.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-platform.md\` — Task 2

## Depends on
#$I09

## Gate
\`cargo run --example counter\` (blank window OK)
EOF
)")
I10=$(issue_num "$URL")
echo "Created #$I10"

URL=$(create_issue "11" "[11/13] Platform: frame loop (runtime → patches → layout → paint)" "$(cat <<EOF
## Goal
Wire frame tick: \`flush_effects\` → apply patches → \`layout_pass\` → \`paint_pass\` → present; \`lemon::run\`.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-platform.md\` — Task 3

## Depends on
#$I10

## Gate
\`cargo run --example counter\` — UI updates on signal change
EOF
)")
I11=$(issue_num "$URL")
echo "Created #$I11"

URL=$(create_issue "12" "[12/13] Platform: hit-test and on_click routing" "$(cat <<EOF
## Goal
Map cursor to logical coords; hit-test \`LayoutMap\`; invoke \`on_click\`; counter button works.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-platform.md\` — Task 4

## Depends on
#$I11

## Gate
\`cargo run --example counter\` — click increments label
EOF
)")
I12=$(issue_num "$URL")
echo "Created #$I12"

URL=$(create_issue "13" "[13/13] Deferred use_effect (run after first paint)" "$(cat <<EOF
## Goal
\`cx.use_effect\` queues until \`flush_deferred_effects()\` after first paint — matches spec lifecycle.

## Plan
\`docs/superpowers/plans/2026-05-17-lemon-runtime-semantics.md\` — Task 3

## Files
- \`src/runtime/cx.rs\`, \`effect.rs\`, \`mod.rs\`
- \`src/platform/mod.rs\`

## Depends on
#$I12 (requires platform frame loop)

## Gate
\`cargo test\` + \`cargo run --example counter\`
EOF
)")
I13=$(issue_num "$URL")
echo "Created #$I13"

TRACKER=$(create_issue "00" "[00] Lemon v1 implementation tracker (linear roadmap)" "$(cat <<EOF
# Lemon v1 — linear execution order

**Spec:** \`docs/superpowers/specs/2026-05-17-lemon-architecture-design.md\` (see **Implementation Status** section)  
**Roadmap:** \`docs/superpowers/ROADMAP.md\`

Execute **in order**. Each issue blocks the next unless noted.

| Order | Issue | Layer | Milestone |
|------:|-------|-------|-----------|
| 1 | #$I01 | 4 | Keyed diff |
| 2 | #$I02 | 1 | Derived equality |
| 3 | #$I03 | 6 | LayoutMap |
| 4 | #$I04 | 6 | Parley measure |
| 5 | #$I05 | 6 | Layout integration |
| 6 | #$I06 | 7 | Paint backgrounds |
| 7 | #$I07 | 7 | Paint glyphs |
| 8 | #$I08 | 7 | Paint complete |
| 9 | #$I09 | 8 | AppState |
| 10 | #$I10 | 8 | GPU bootstrap |
| 11 | #$I11 | 8 | Frame loop |
| 12 | #$I12 | 8 | Hit-test |
| 13 | #$I13 | 2 | Deferred use_effect |

## Already done (Phase 0)
- Layers 1–5 core: runtime, element, diff, retained, component lifecycle (\`cargo test\`, 68 tests)

## Out of v1 (no issues)
Multi-window, keyboard input, scroll, image paint, overflow clip, a11y

## Per-issue checklist
\`\`\`bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
# After #11+: cargo run --example counter
\`\`\`
EOF
)")
echo "Created tracker #$TRACKER"
echo "Done. Tracker: https://github.com/$REPO/issues/$TRACKER"
