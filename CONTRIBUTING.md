# Contributing to Lemon

Start with a [bug report](https://github.com/lucasaarch/lemon/issues) or describe the
change you want to make. Include a small reproduction, operating system, Rust
version and the expected behavior. For UI bugs, a screenshot and the relevant
view code help.

## Local development

Use stable Rust, Python 3.9 or newer, and Make. Linux needs Fontconfig development
headers (`sudo apt-get install libfontconfig1-dev` on Ubuntu). A compatible GPU and
a desktop session are required to run windows; most tests are headless.

```sh
git clone https://github.com/lucasaarch/lemon.git
cd lemon
cargo run --example tasks
make check
```

`make check` runs formatting, Clippy with warnings denied, library/integration/doc
tests, example application tests, maintenance script tests, strict Rustdoc and all
example builds. CI runs it on Linux, macOS and Windows. Rust 1.88 is checked
separately; update the minimum version deliberately if dependencies require it.

On Windows, use Git Bash and Make (`choco install make`), then run
`make check PYTHON=python SHELL=bash`, matching CI.

GPU pixel tests require a working adapter and are opt-in:

```sh
cargo test --test effects_gpu -- --ignored
```

## Changes

- Read [AGENTS.md](AGENTS.md) for module ownership and implementation constraints.
- Keep logic in its owning layer; UI builders must not depend on native backends.
- Document public APIs in source, update the application guide when needed and add
  a regression test for changed behavior.
- Preserve hook order and stable keys. Use a component scope for each dynamic row
  that creates stateful controls.
- Examples are working applications, not widget catalogs. Give each one a clear
  purpose, working actions, empty/error states and tests for its main workflow.
- Keep unrelated edits out of the pull request. Update [CHANGELOG.md](CHANGELOG.md)
  for user-visible changes.

## Releases

The only package version is `package.version` in `Cargo.toml`; commit the resulting
`Cargo.lock` change. There is no automatic version bump on branch pushes.

1. Update the version and add its section to `CHANGELOG.md`.
2. Run `make check` and `cargo publish --dry-run --locked` from a clean checkout.
3. Merge/push the reviewed commit and confirm CI.
4. Create an annotated tag matching the package version, for example
   `git tag -a v1.0.1 -m 'Lemon 1.0.1'`, then `git push origin v1.0.1`.
5. The Release workflow validates the tag and notes, runs every CI gate, publishes
   to crates.io using the repository's `CARGO_REGISTRY_TOKEN` secret and then creates
   the GitHub release. The token needs permission to publish `lemon`.
6. Verify the registry version, GitHub release and docs.rs build.

If GitHub release creation fails after a successful Cargo upload, rerun only the
failed GitHub release job; package versions cannot be republished. Never move a
published release tag. Yank a broken package version and publish a corrected one.
GitHub release deletion and crates.io yank are separate operations. Yank preserves
package archives and existing lockfiles.

## License

Contributions are licensed under the repository's [MIT license](LICENSE).
The bundled Lucide icons retain their [ISC license](licenses/lucide-LICENSE).
