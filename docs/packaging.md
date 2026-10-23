# Packaging desktop applications

Lemon apps are ordinary Rust executables. Build the app for the target operating
system, then use a platform packager to add its application metadata, icon,
resources and installer format. Lemon's recommended path is
[`cargo-packager`](https://github.com/crabnebula-dev/cargo-packager); it creates
macOS `.dmg`, Windows `.msi`, and Linux `.deb` packages from one configuration
format. You can use another packager when your distribution needs call for it.

## Configure your application

Install the packager once:

```sh
cargo install cargo-packager --version 0.11.8 --locked
```

Add a `Packager.toml` beside your app's `Cargo.toml` and adapt this example to
your binary name, metadata, and native runtime dependencies:

```toml
name = "workspace"
product-name = "Workspace"
identifier = "com.example.workspace"
version = "0.1.0"
out-dir = "target/packages"
binaries-dir = "target/release"
binaries = [{ path = "workspace", main = true }]
before-packaging-command = "cargo build --locked --release --bin workspace"

[deb]
# Keep the libraries this application actually links at runtime.
depends = ["libfontconfig1", "libxkbcommon0", "libvulkan1"]
```

Then build the installer for the current operating system:

```sh
# macOS: creates a .dmg
cargo packager --formats dmg

# Linux: creates a .deb
cargo packager --formats deb

# Windows: creates an .msi (requires the WiX Toolset)
cargo packager --formats wix
```

Packages are written to `target/packages/`. Set a reverse-domain identifier
that belongs to your application and keep its version in sync with the app's
release. Add your platform icon with the `icons` configuration field, and list
files the executable needs at runtime under `resources`. The
[packager configuration reference](https://docs.crabnebula.dev/packager/configuration/)
covers platform-specific settings, signing and additional package formats.

## Optional GitHub Actions workflow

This example builds packages for the application configured above and uploads
them as workflow artifacts. It does not publish a release. Copy it to your app's
`.github/workflows/package.yml` and adjust the build dependencies as needed.

```yaml
name: Package

on:
  workflow_dispatch:
  pull_request:

permissions:
  contents: read

jobs:
  package:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            format: dmg
            artifact: app-macos
          - os: ubuntu-latest
            format: deb
            artifact: app-linux
          - os: windows-latest
            format: wix
            artifact: app-windows
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Install Linux build dependencies
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y libfontconfig1-dev
      - name: Install WiX Toolset
        if: runner.os == 'Windows'
        run: choco install wixtoolset --no-progress --yes
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cargo-packager
        run: cargo install cargo-packager --version 0.11.8 --locked
      - name: Build package
        shell: bash
        run: cargo packager --formats "${{ matrix.format }}"
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/packages/*
          if-no-files-found: error
```

Signing, macOS notarization, store submission and publishing require credentials
owned by the application publisher. They are intentionally not enabled by the
example workflow.
