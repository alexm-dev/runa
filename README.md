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

## What's New in v0.2.0  
- Multi-Pane Layout: New Preview and Origin (Parent) panes for better context.
- Extended Configuration: Customize border style, colors of entries for each pane and more...
- Reactive Engine: Re-engineerd event loop for much better performance

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

You can generate a default config using the --gen-config flag:

```bash
rn --init
```

This will generate a config in the default config path.

## Roadmap

runner is in active development.  
Future releases will focus on expanding functionality while keeping it fast and lightweight.  

Planned features for future releases include:

- [ ] Search: Integrated fuzzy find and grep support.

- [ ] File Operations: Copy, move, delete, and rename from within the UI.

- [ ] Image Previews: Support for Sixel/Kitty graphics protocols.

- [x] Performance: Reactive rendering (Completed in 0.2.0).

## License
MIT License
