# `rn - runner`

> ⚠️ Note: This crate is deprecated. Please use [`runa`](https://crates.io/crates/runa-tui) for all new installations.

# ⚠️ DEPRECATED: runner is renamed to runa

This crate has been **renamed** to [`runa`](https://crates.io/crates/runa-tui).  
The repository remains at [https://github.com/alexm-dev/runa](https://github.com/alexm-dev/runa)

runa supports legacy configuration at `$HOME/.config/runner/runner.toml`.
The default config path for runa-tui will be `$HOME/.config/runa/runa.toml` in future releases.

**Reason for rename:**  
The original name `runner` could cause confusion with GitHub Action runners, code runners, and other general "runner" terminology.  
The new name is `runa` while the CLI binary still being `rn`.

### Installation (new):

```bash
cargo install runa-tui
```


# `runner` (DEPRECATED)

[![Build](https://github.com/alexm-dev/runa/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/alexm-dev/runa/actions)
[![Crates.io](https://img.shields.io/crates/v/runa-tui.svg)](https://crates.io/crates/runa-tui)
[![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**Documentation**
- [Configuration](docs/configuration.md)

A fast and lightweight console file browser written in Rust

**rn - runner** is a minimalist terminal file manager focused on speed and simplicity.  

It allows you to navigate directories, view file listings, and perform typical file browser actions.  

This project is a work in progress.  
It is being actively developed and will change over time.  

## Changelog

For a detailed list of changes and release notes, see [CHANGELOG.md](./CHANGELOG.md).

## Installation

Installation via cargo:

```bash
cargo install runner-tui
```

### Build from source

Clone the repo and build with Cargo:

```bash
git clone https://github.com/alexm-dev/runner.git
cd runner
cargo build --release
```

### Usage

Run runner with:
`rn`

## Configuration

A full configuration documentation will follow.  

runner uses a runner.toml file for configuration.
By default, it is located at:

`$HOME/.config/runner/runner.toml` (on both Unix and Windows, inside the user folder)

You can override the config path by setting an environment variable:

```bash
# Unix
export RUNNER_CONFIG=/path/to/runner.toml

# PowerShell (Windows)
$env:RUNNER_CONFIG="C:\path\to\runner.toml"
```

You can generate a default config using the --init or --init-minimal flag:

```bash
rn --init

# For a very minimal config
rn --init-minimal
```

This will generate a config in the default config path.

## Roadmap

runner is in active development.  
Future releases will focus on expanding functionality while keeping it fast and lightweight.  

### Planned features

- [ ] Search & Discovery: Integrated fuzzy finding and fast directory traversal.  

- [ ] Content Search: Text search and filtering.  

- [ ] File Operations: Copy, move, delete, and rename from within the UI.  

- [ ] Image Previews: Support for Sixel/Kitty graphics protocols.  

### Completed

- [x] Performance: Reactive rendering (Completed in 0.2.0).

- [x] UI Customization: Pane-specific styling and Hex color support (Completed in 0.2.2).  

- [x] Navigation Context: Persistent Parent (Origin) and Preview panes (Completed in 0.2.0)  

## License
This project is Licensed under the MIT License  
See the [LICENSE](LICENSE) file for details.
