# `runner`

[![Build](https://github.com/alexm-dev/runner/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/alexm-dev/runner/actions)
[![Crates.io](https://img.shields.io/crates/v/runner-tui.svg)](https://crates.io/crates/runner-tui)
[![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**Documentation**
- [Configuration](docs/configuration.md)

A fast and lightweight console file browser written in Rust

runner is a minimalist terminal file manager focused on speed and simplicity.  

It allows you to navigate directories, view file listings, and perform typical file browser actions.  

This project is a work in progress.  
It is being actively developed and will change over time.  

## What's New in v0.2.1 and v0.2.2

#### UI improvements
- Improved pane customization by adding pane specific selection cache.
- Custom Path Styling (v0.2.2): Addded a dedicated theme.path configuration to customize the path at the top of the UI.
- Pane-Specific Selections (v0.2.2): Added selection foreground and background colors for each pane for more customization.

#### Performance
- Switched to crossbeam-channel for better thread sync.
- Performance improvements by reducing event_loop string creation.
- Optimized `always_show` and other flags by using Atomics.
- Migrated text rendering to use Line and Span for better performance and future-proofing.

#### Fixes
- Fixed preview sorting issue. Now shows the directories in the correct oder.


## Features
- Navigate directories in the Terminal
- Lightweight and minimal memory usage
- Cross-platform: Works on Linux, Windows and macOS.
- Configurable keybindings via TOML configuration file.

## Installation

Installation via cargo:

```bash
cargo install runner-tui
```

### Build from source

Clone the repo and build with Cargo:

```bash
https://github.com/alexm-dev/runner.git
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

You can override the config path by setting a environment variable:

```bash
# Unix
export RUNNER_CONFIG=/path/to/runner.toml

# PowerShell (Windows)
$env:RUNNER_CONFIG="C:\path\to\runner.toml
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

- [x] Navigation Context: Persisent Parent (Origin) and Preview panes (Completed in 0.2.0)  

## License
This project is Licensed under the MIT License  
See the [LICENSE](LICENSE) file for details.
