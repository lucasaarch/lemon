---
name: resolve-issue
description: "Resolve a GitHub issue end to end in this repository: read and obey AGENTS.md, fetch the issue with the gh CLI, implement the fix, validate with cargo check/fmt/clippy/tests, record a CVM version change, commit the work, and open a pull request linked to the issue. Use when the user asks to work from a GitHub issue number or URL, fix an assigned issue, or ship issue-scoped Rust changes in this repo."
---

# Resolve Issue

## Overview

Use this skill to take a GitHub issue from intake to pull request in the Lemon repository.

This is a workflow skill. Follow the sequence strictly. Do not skip `AGENTS.md`, do not guess at issue scope if the issue text is available through `gh`, and do not claim completion without running the full validation gates.

After implementation and validation, you MUST stage a CVM change file so the release pipeline can version the crates.

## Workflow

### Step 1: Establish repository rules

Before doing any implementation work:

1. Read `AGENTS.md`.
2. Treat `AGENTS.md` as the authoritative repository policy.
3. Ignore `CONTEXT.md` and `docs/superpowers/*`.
4. Check git status so you understand whether the worktree is already dirty.

If local changes exist, do not revert them unless the user explicitly asked for that.

### Step 2: Resolve the issue context with `gh`

The issue is the source of truth for scope.

If the user gave:

- an issue number: use it directly
- an issue URL: extract the issue number and repository
- ambiguous prose only: locate the matching issue with `gh issue list` or `gh issue view` before coding

Use `gh` to fetch the issue, not memory.

Minimum commands:

```bash
gh issue view <issue-number> --json number,title,body,labels,assignees,url,state
gh issue view <issue-number> --comments
```

If the issue references a linked PR, design doc, or follow-up issue, inspect that context before editing code.

Summarize the issue to yourself in terms of:

- expected behavior
- affected layer(s)
- explicit acceptance criteria
- unclear or risky assumptions

If the issue is too ambiguous to implement safely, stop and ask the user. Otherwise proceed.

### Step 3: Choose the implementation surface

Use `AGENTS.md` to determine the owning layer:

- `runtime/` for reactive update bugs
- `diff/` for reconciliation and keyed behavior
- `retained/` for retained-node and patch application behavior
- `layout/` for geometry and text measurement
- `paint/` for scene generation
- `platform/` for input, hit testing, window-loop behavior
- `element/` for public builders, style, events, and types

Keep the change local to the owning layer whenever possible.

### Step 4: Create a branch

Do not implement issue work directly on a generic long-lived branch if you can avoid it.

Create a branch name derived from the issue number and title, for example:

```bash
git checkout -b issue-123-hover-hit-test
```

Use a short, readable slug.

If you are already on a dedicated branch for the issue, reuse it.

### Step 5: Implement the fix

During implementation:

1. Re-read the nearest relevant code before editing.
2. Extend existing patterns instead of inventing a new style.
3. Add or update tests close to the modified module.
4. If public API changes, update examples and exports.
5. If new fields are added to core structs, propagate them everywhere required by `AGENTS.md`.

Repository-specific requirements:

- Preserve the pipeline: `runtime -> diff -> retained -> layout -> paint -> platform`
- Do not treat `Component::new(...)` as if it accepted arbitrary captured closures
- Re-export app-facing builder changes through `src/widget` when needed

### Step 6: Run progressive verification during development

Run the smallest relevant checks first.

Examples:

```bash
cargo test
cargo test::widget
make build-examples
```

If the issue affects one module, run the narrowest useful test target first before widening.

Do not wait until the very end to discover a basic compile failure.

### Step 7: Run mandatory final validation

Before recording a version change or opening a pull request, run all required checks.

Run:

```bash
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If any command fails:

1. fix the problem
2. rerun the failing command
3. rerun the full validation set before claiming success

Do not open a PR with known failing validation unless the user explicitly asked for that.

### Step 8: Record version change with CVM (required)

After implementation and validation succeed, stage a version bump using [CVM](https://github.com/lucasaarch/cvm) **non-interactive** mode. Do this from the **repository root** (where `.cvm/` lives).

#### 8.1 Install `cvm_cli`

Requires **cvm_cli 1.1.2+** (non-interactive `--crate` / `--bump` / `--summary`, workspace `version.workspace = true` apply).

```bash
cargo install cvm_cli --version 1.1.2 --locked
```

Verify:

```bash
cvm --crate lemon --bump patch --summary "dry-run check" --dry-run
```

#### 8.2 Decide bump level

Choose **one** bump type per staged change based on what you shipped. Use your judgment from the actual diff, not the issue title alone.

| Bump | When to use |
|------|-------------|
| **patch** | Bug fixes, regressions, internal refactors, tests-only, CI/docs, performance fixes without API changes |
| **minor** | New backward-compatible features, new public types/builders/exports, new widgets in `lemon::widget`, new optional behavior |
| **major** | Breaking public API changes, removed or renamed exports, intentional semantic breaks consumers must react to |

If unsure between patch and minor, prefer **patch** for fixes and **minor** for user-visible additions.

#### 8.3 Choose crates

This workspace publishes **`lemon`** only. Examples are `[[example]]` targets in the root `Cargo.toml` — do **not** use `--crate all`.

| What changed | Crates to bump |
|--------------|----------------|
| Only `src` (no public widget surface) | `--crate lemon` |
| Only `src/widget` | `--crate lemon::widget` |
| Public API / widgets / re-exports in both crates | `--crate lemon --crate lemon::widget` (same `--bump` for both) |

When both library crates need the same bump, pass one `--bump` and multiple `--crate` flags.

#### 8.4 Write an effective `--summary`

The summary is the changelog line for this release. It MUST be:

- **Specific** — what changed, not "fix issue" or "implement feature"
- **User- or maintainer-facing** — outcome, not file names
- **One line** — concise TL;DR (roughly 8–120 characters)

Good examples:

- `Fix hover hit-testing for nested retained nodes`
- `Add TextFieldState and wire keyboard input for text fields`
- `Export Scroll widget from lemon::widget`

Bad examples:

- `Fix #123`
- `Update code`
- `WIP`

#### 8.5 Run CVM

```bash
cvm --crate lemon --crate lemon::widget --bump minor --summary "Add TextFieldState for controlled text input"
```

Adjust `--crate` and `--bump` to match your decision.

Optional preview:

```bash
cvm --crate lemon --bump patch --summary "..." --dry-run
```

Confirm:

```bash
cvm status   # should report pending changes (exit 1)
ls .cvm/changes/
```

Do **not** run `cvm apply` — CI on branch `alpha` applies bumps and opens the version PR. Only stage the change file.

#### 8.6 Include CVM files in the commit

The new file under `.cvm/changes/` MUST be committed with the implementation (same PR).

### Step 9: Commit the work

Stage implementation files **and** the new `.cvm/changes/*.toml` file.

Use a focused commit message, for example:

```bash
git add <relevant-files> .cvm/changes/
git commit -m "fix: handle hover hit-testing for retained nodes"
```

Do not sweep unrelated local changes into the commit.

### Step 10: Open the pull request and associate it with the issue

Push the branch, then create the PR with `gh`.

The PR body MUST associate the PR with the issue using a closing keyword.

Use a body that contains at least:

- a short summary
- validation performed
- CVM change staged (bump level, crates, summary text)
- `Closes #<issue-number>`

Example flow:

```bash
git push -u origin <branch-name>
gh pr create \
  --title "fix: handle hover hit-testing for retained nodes" \
  --body "$(cat <<'EOF'
## Summary
- fix hover hit-testing for retained nodes
- add regression coverage for hover lookup

## Validation
- cargo check --workspace
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace

## Version (CVM)
- **Bump:** patch on `lemon`
- **Summary:** Fix hover hit-testing for nested retained nodes

Closes #123
EOF
)"
```

If the repository has a PR template, use `gh pr create --fill` only if the filled result still includes the issue-closing line, the CVM section, and an accurate validation section.

### Step 11: Report completion

When done, report:

- branch name
- commit SHA
- PR URL
- exact validation commands run
- CVM bump level, crate(s), and summary used
- path to the staged `.cvm/changes/` file
- any notable assumptions or follow-up risks

## Guardrails

- Read `AGENTS.md` before implementation.
- Use `gh` to fetch issue details; do not rely on memory.
- Do not skip the full validation set.
- Do not skip CVM change staging after successful validation.
- Do not run `cvm apply` or `cvm publish` in this workflow unless the user explicitly asked.
- Do not use `--crate all` in Lemon (it includes non-publishable examples).
- Do not open a PR without linking it to the issue with `Closes #<n>` or equivalent.
- Do not revert unrelated user changes.
- Do not broaden scope beyond the issue unless that is necessary to make the fix correct.

## Quick Reference

```bash
gh issue view <issue-number> --json number,title,body,labels,assignees,url,state
gh issue view <issue-number> --comments
git checkout -b issue-<number>-<slug>
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo install cvm_cli --version 1.1.2 --locked
cvm --crate lemon --crate lemon::widget --bump patch --summary "<effective one-line changelog>"
cvm status
git add .cvm/changes/ <files>
git commit -m "<type>: <description>"
git push -u origin <branch-name>
gh pr create --title "<title>" --body "<body with CVM section and Closes #<issue-number>>"
```
