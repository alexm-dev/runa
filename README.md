# `runa - rn`

[![Build](https://github.com/alexm-dev/runa/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/alexm-dev/runa/actions)
[![Crates.io](https://img.shields.io/crates/v/runa-tui.svg)](https://crates.io/crates/runa-tui)
[![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**Documentation**
- [Configuration](docs/configuration.md)


<img src="https://github.com/user-attachments/assets/202ec419-80fe-427f-975b-0ef1d31d501a" alt="runa_default" width="100%" style="max-width: 1274px; height: auto;" />


A fast and lightweight console file browser written in Rust

**rn - runa** is a minimalist terminal file manager focused on speed and simplicity.  

It allows you to navigate directories, view file listings, and perform typical file browser actions.  

**runa** is a work in progress.  
It is being actively developed and will change over time.  

## Changelog

For a detailed list of changes and release notes, see [CHANGELOG.md](./CHANGELOG.md).

## Installation

Installation via cargo:

```bash
cargo install runa-tui
```

### Build from source

Clone the repo and build with Cargo:

```bash
git clone https://github.com/alexm-dev/runa.git
cd runa
cargo build --release
```

### Usage

Run runa with:
`rn`

## Configuration

A full configuration documentation will follow.  

runa uses a runa.toml file for configuration.
By default, it is located at:

`$HOME/.config/runa/runa.toml` (on both Unix and Windows, inside the user folder)

You can override the config path by setting an environment variable:

```bash
# Unix
export RUNA_CONFIG=/path/to/runa.toml

# PowerShell (Windows)
$env:RUNA_CONFIG="C:\path\to\runa.toml"
```

You can generate a default config using the --init or --init-minimal flag:

```bash
rn --init

# For the whole configuration options runa.toml
rn --init-full

# For help with all the configuration options.
rn --config-help
```

This will generate a config in the default config path.

## Roadmap

runa is in active development.  
Future releases will focus on expanding functionality while keeping it fast and lightweight.  

### Planned features

- [ ] Search & Discovery: Integrated fuzzy finding and fast directory traversal.  

- [ ] Image Previews: Support for Sixel/Kitty graphics protocols.  

### Completed

- [x] File Operations: Copy, move, delete, and rename from within the UI.  ( Completed in 0.3.0 )  

- [x] Content Search: Text search and filtering.  (Completed in 0.3.0)  

- [x] Performance: Reactive rendering (Completed in 0.2.0).

- [x] UI Customization: Pane-specific styling and Hex color support (Completed in 0.2.2).

- [x] Navigation Context: Persistent Parent (Origin) and Preview panes (Completed in 0.2.0)  


<img src="https://github.com/user-attachments/assets/778b80cc-3e6f-45ab-a770-e4d88059995a" alt="runa_default_3" style="max-width: 100%; height: auto;">


## License
This project is Licensed under the MIT License  
See the [LICENSE](LICENSE) file for details.
