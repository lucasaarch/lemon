---
name: resolve-issue
description: "Resolve a GitHub issue end to end in this repository: read and obey AGENTS.md, fetch the issue with the gh CLI, implement the fix, validate with cargo check/fmt/clippy/tests, commit the work, and open a pull request linked to the issue. Use when the user asks to work from a GitHub issue number or URL, fix an assigned issue, or ship issue-scoped Rust changes in this repo."
---

# Resolve Issue

## Overview

Use this skill to take a GitHub issue from intake to pull request in the Lemon repository.

This is a workflow skill. Follow the sequence strictly. Do not skip `AGENTS.md`, do not guess at issue scope if the issue text is available through `gh`, and do not claim completion without running the full validation gates.

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
- Re-export app-facing builder changes through `crates/lemon-widgets` when needed

### Step 6: Run progressive verification during development

Run the smallest relevant checks first.

Examples:

```bash
cargo test -p lemon
cargo test -p lemon-widgets
cargo build -p counter
cargo build -p card
cargo build -p list_keyed
```

If the issue affects one module, run the narrowest useful test target first before widening.

Do not wait until the very end to discover a basic compile failure.

### Step 7: Run mandatory final validation

Before opening a pull request, run all required checks.

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

### Step 8: Commit the work

Stage only the files relevant to the issue.

Use a focused commit message, for example:

```bash
git add <relevant-files>
git commit -m "fix: handle hover hit-testing for retained nodes"
```

Do not sweep unrelated local changes into the commit.

### Step 9: Open the pull request and associate it with the issue

Push the branch, then create the PR with `gh`.

The PR body MUST associate the PR with the issue using a closing keyword.

Use a body that contains at least:

- a short summary
- validation performed
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

Closes #123
EOF
)"
```

If the repository has a PR template, use `gh pr create --fill` only if the filled result still includes the issue-closing line and an accurate validation section.

### Step 10: Report completion

When done, report:

- branch name
- commit SHA
- PR URL
- exact validation commands run
- any notable assumptions or follow-up risks

## Guardrails

- Read `AGENTS.md` before implementation.
- Use `gh` to fetch issue details; do not rely on memory.
- Do not skip the full validation set.
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
git push -u origin <branch-name>
gh pr create --title "<title>" --body "<body with Closes #<issue-number>>"
```
