# `runa - rn`

[![Build Status](https://img.shields.io/github/actions/workflow/status/alexm-dev/runa/rust.yml?style=flat-square&logo=github&label=build)](https://github.com/alexm-dev/runa/actions)
[![Latest Release](https://img.shields.io/github/v/release/alexm-dev/runa?style=flat-square&color=8839ef&label=release)](https://github.com/alexm-dev/runa/releases)
[![Crates.io](https://img.shields.io/crates/v/runa-tui?style=flat-square&color=e67e22&logo=rust)](https://crates.io/crates/runa-tui)
[![AUR](https://img.shields.io/aur/version/runa?color=blue&style=flat-square)](https://aur.archlinux.org/packages/runa)
[![Language](https://img.shields.io/badge/language-Rust-DEA584?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/github/license/alexm-dev/runa?style=flat-square&color=3498db)](LICENSE)

**Documentation**
- [Configuration](docs/configuration.md)


<img src="https://github.com/user-attachments/assets/202ec419-80fe-427f-975b-0ef1d31d501a" alt="runa_default" width="100%" style="max-width: 1274px; height: auto;" />


A fast and lightweight console file browser written in Rust

**rn - runa** is a minimalist terminal file manager focused on speed and simplicity.  
It allows you to navigate directories, view file listings, and perform typical file browser actions.  


> [!IMPORTANT]
> **runa** is a work in progress. It is being actively developed and features may change over time.

## Changelog
For a detailed list of changes and release notes, see [CHANGELOG.md](./CHANGELOG.md).

## Installation

### Installation via cargo:

```bash
cargo install runa-tui
```

### Arch Linux (AUR)

You can install runa from the [AUR](https://aur.archlinux.org/packages/runa) using an AUR helper like `paru` or `yay`:

```bash
yay -S runa
```

### Pre-compiled Binaries

If you'd like to download Pre-compiled binaries instead of isntalling runa as a crate in cargo or via the AUR,
you can grab the latest binaries for Linux, Windows and macOS form the [Release](https://github.com/alexm-dev/runa/releases) page.

> [!TIP]
> **Checksum Check:** You can verify the integrity of the release binaries using the `SHA256SUMS.txt` file in the [Release](https://github.com/alexm-dev/runa/releases) page.
> ```bash
> sha256sum -c SHA256SUMS.txt
> ```

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

- [ ] Syntax Highlighting: Treesitter integration for the preview pane.  

### Completed

- [x] File Operations: Copy, move, delete, and rename from within the UI.  ( Completed in 0.3.0 )  

- [x] Content Search: Text search and filtering.  (Completed in 0.3.0)  

- [x] Performance: Reactive rendering (Completed in 0.2.0).

- [x] UI Customization: Pane-specific styling and Hex color support (Completed in 0.2.2).

- [x] Navigation Context: Persistent Parent (Origin) and Preview panes (Completed in 0.2.0)  

---


<img src="https://github.com/user-attachments/assets/778b80cc-3e6f-45ab-a770-e4d88059995a" alt="runa_default_3" style="max-width: 100%; height: auto;">


---

## Support & Contribute
If you enjoy using **runa**, you can help the project grow:

* ⭐ **Star the Repo:** It helps more people discover runa :)
* 🐛 **Report Bugs:** Open an issue if something doesn't work as expected.
* 💡 **Feature Requests:** Suggest new ideas in the [Issues](https://github.com/alexm-dev/runa/issues) tab.
* 🦀 **Contribute:** Pull requests are always welcome!


## Built With
`runa` stands on the shoulders of these incredible Rust crates:

- **TUI Framework:** [Ratatui](https://ratatui.rs) & [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Configuration:** [Serde](https://serde.rs) & [TOML](https://github.com/toml-rs/toml)
- **System Integration:** [Dirs](https://github.com/dirs-dev/dirs-rs) (Standard config locations)
- **Concurrency:** [Crossbeam-channel](https://github.com/crossbeam-rs/crossbeam)

## License
This project is Licensed under the MIT License  
See the [LICENSE](LICENSE) file for details.
