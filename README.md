# `runa - rn`

[![Build](https://img.shields.io/github/actions/workflow/status/alexm-dev/runa/rust.yml?style=flat-square&logo=github&label=Build)](https://github.com/alexm-dev/runa/actions/workflows/rust.yml)
[![Latest Release](https://img.shields.io/github/v/release/alexm-dev/runa?style=flat-square&color=8839ef&label=Release)](https://github.com/alexm-dev/runa/releases)
[![Crates.io](https://img.shields.io/crates/v/runa-tui?style=flat-square&color=e67e22&logo=rust)](https://crates.io/crates/runa-tui)
[![AUR](https://img.shields.io/aur/version/runa?label=AUR&color=blue&style=flat-square&logo=archlinux)](https://aur.archlinux.org/packages/runa)
[![Language](https://img.shields.io/github/languages/top/alexm-dev/runa?style=flat-square&logo=rust&color=%23e67e22&label=Rust)](https://github.com/alexm-dev/runa)
[![License](https://img.shields.io/github/license/alexm-dev/runa?label=License&style=flat-square&color=3498db)](LICENSE)

**Documentation**
- [Configuration](docs/configuration.md)

<img src="https://github.com/user-attachments/assets/22663c5a-3fbb-4480-856e-4a1efa4bd5b6" alt="runa" width="100%" style="max-width: 1274px; height: auto;" />
<br><br>

A fast and lightweight console file browser written in Rust

**runa - rn** is a minimalist terminal file manager focused on speed and simplicity.  
It allows you to navigate directories, view file listings, and perform typical file browser actions.  

**runa** is very customizable, checkout the [Configuration](docs/configuration.md) docs for all the available options.

> [!IMPORTANT]
> **runa** is a work in progress. It is being actively developed and features may change over time.

## Changelog
For a detailed list of changes and release notes, see [CHANGELOG.md](./CHANGELOG.md).

## Installation

### Cargo:

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

After downloading, add the `rn` (Linux/macOS) or `rn.exe` (Windows) binary to your system `PATH` to use runa from your terminal.

> [!TIP]
> **Checksum Check:** You can verify the integrity of the release archives using the `SHA256SUMS.txt` file in the [Release](https://github.com/alexm-dev/runa/releases) page.
>
> Unix
> ```bash
> sha256sum -c SHA256SUMS.txt
> ```
> This checks all the checksums of the SHA256SUMS.txt.
> To check a specific release archive:
> ```bash
> grep runa-linux-x86_64.tar.gz SHA256SUMS.txt | sha256sum -c
> ```
>
> Windows
> ```powershell
> Get-FileHash runa-windows-x86_64.zip -Algorithm SHA256
> ```
> Compare the output with the corresponding entry in `SHA256SUMS.txt`.

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

- [ ] Image Previews: Support for Sixel/Kitty graphics protocols.  

- [ ] Syntax Highlighting: Treesitter (or similar) integration for the preview pane.  

### Completed

- [x] Search & Discovery: Integrated fuzzy finding (fd support) (Completed in 0.4.0)

- [x] File Operations: Copy, move, delete, and rename from within the UI.  ( Completed in 0.3.0 )  

- [x] Content Search: Text search and filtering.  (Completed in 0.3.0)  

- [x] Performance: Reactive rendering (Completed in 0.2.0).

- [x] UI Customization: Pane-specific styling and Hex color support (Completed in 0.2.2).

- [x] Navigation Context: Persistent Parent (Origin) and Preview panes (Completed in 0.2.0)  

---


<img src="https://github.com/user-attachments/assets/778b80cc-3e6f-45ab-a770-e4d88059995a" alt="runa_default_3" style="max-width: 100%; height: auto;">


---

## Optional Enhancements

`runa` is designed to be lightweight and standalone. However, some advanced features leverage specialized external tools:

* **Fuzzy Search:** To enable fast, recursive fuzzy finding, install **[fd](https://github.com/sharkdp/fd)**.
  * If `fd` is detected in your `PATH`, the search feature will be enabled automatically.
  * Without it, `runa` remains a fully functional file manager but will notify you if you attempt a recursive search.


## Support & Contribute
If you enjoy using **runa**, you can help the project grow:

* ⭐ **Star the Repo:** It helps more people discover runa :)
* 🐛 **Report Bugs:** Open an issue if something doesn't work as expected.
* 💡 **Feature Requests:** Suggest new ideas in the [Issues](https://github.com/alexm-dev/runa/issues) tab.
* 🦀 **Contribute:** Pull requests are always welcome!


## Credits & Ecosytem
`runa` stands on the shoulders of these incredible Rust crates:

- **TUI Framework:** [Ratatui](https://ratatui.rs) & [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Configuration:** [Serde](https://serde.rs) & [TOML](https://github.com/toml-rs/toml)
- **System Integration:** [Dirs](https://github.com/dirs-dev/dirs-rs) (Standard config locations)
- **External Integration:** [fd-find](https://github.com/sharkdp/fd) (Optional backend for fuzzy find)
- **Concurrency:** [Crossbeam-channel](https://github.com/crossbeam-rs/crossbeam)

## License
This project is Licensed under the MIT License  
See the [LICENSE](LICENSE) file for details.
