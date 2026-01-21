# `runa - rn`

<div align="center">
 
[![Build](https://img.shields.io/github/actions/workflow/status/alexm-dev/runa/rust.yml?style=flat-square&logo=github&label=Build)](https://github.com/alexm-dev/runa/actions/workflows/rust.yml)
[![Latest Release](https://img.shields.io/github/v/release/alexm-dev/runa?style=flat-square&color=8839ef&label=Release)](https://github.com/alexm-dev/runa/releases)
[![Crates.io](https://img.shields.io/crates/v/runa-tui?style=flat-square&color=e67e22&logo=rust)](https://crates.io/crates/runa-tui)
[![AUR](https://img.shields.io/aur/version/runa?label=AUR&color=blue&style=flat-square&logo=archlinux)](https://aur.archlinux.org/packages/runa)
[![Language](https://img.shields.io/github/languages/top/alexm-dev/runa?style=flat-square&logo=rust&color=%23e67e22&label=Rust)](https://github.com/alexm-dev/runa)
[![License](https://img.shields.io/badge/License-MIT%20or%20Apache%202.0-blue?style=flat-square)](#license)

</div>
<div align="center">

> **A fast, ultra-lightweight, and extremely customizable terminal file browser carved in Rust.**

</div>
<div align="center">

<a href="docs/configuration.md">Full Configuration Guide</a> • 
<a href="#installation">Installation Guide</a> • 
<a href="docs/configuration.md#quick-start">Quick Start</a> • 
<a href="docs/configuration.md#general-settings">General Settings</a> • 
<a href="docs/configuration.md#theme-configuration">Theme Configuration</a> • 
<a href="docs/configuration.md#key-bindings">Key Bindings</a> • 

</div>

- **Rich File Actions:** Move, filter, copy, rename, and more, all with the keyboard.
- **Extremely Customizable:** Tweak keybindings, colors, and layout via TOML.
- **Blazing Fast:** Instant navigation, even in huge directory trees.
- **Minimal Dependencies:** Only the essential crates; advanced features are opt-in.
- **Cross-Platform:** Works on Linux, macOS, and Windows.
- **Keyboard-Driven:** Every action accessible by keybindings. No mouse needed.
- **Integrated Find (`fd` required):** Blazingly fast file search.

<img src="https://github.com/user-attachments/assets/5590719c-016d-41a6-ba83-1acbe32dab1b" alt="runa" width="100%" style="max-width: 1274px; height: auto;" />
<br></br>

<details>

<summary><strong>Why is runa fast?</strong></summary>

- **Multi-threaded engine:** Spawns 4 dedicated worker threads (with `crossbeam_channel`) for I/O, preview, find, and file operations, so UI is never blocked.
- **Essential-only Rust crates:** No external TUI frameworks or bloat.
- **Direct terminal rendering:** Uses [ratatui](https://ratatui.rs/) & [crossterm](https://github.com/crossterm-rs/crossterm).
- **Optional blazing-fast find:** Integrates with [fd](https://github.com/sharkdp/fd) for recursive fuzzy search.
- **Small, native binary:** The `rn` binary is compact (typically 1.3–2 MB, depending on operating system and architecture).

</details>

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

# or for binaries through the AUR
yay -S runa-bin
```

### Homebrew

You can install runa via homebrew.

```bash
brew tap alexm-dev/tap
brew install runa
```

### Scoop (Windows)

You can install runa via scoop.

```bash
scoop add bucket https://github.com/alexm-dev/scoop-bucket
scoop install runa
```

### Pre-compiled Binaries

If you'd like to download Pre-compiled binaries instead of installing runa as a crate in cargo or via the AUR,
you can grab the latest binaries for Linux, Windows and macOS from the [Release](https://github.com/alexm-dev/runa/releases) page.

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

After installation, start runa with: `rn`

> [!TIP]
> **Icons** (for files, folders, etc.) are disabled by default, but can be enabled in your `runa.toml`.  
> To display them correctly, use a **Nerd Font** or a patched font in your terminal.  
> Without a Nerd Font, icons may appear incorrectly and the UI may not render as intended.  

## Optional Enhancements

`runa` is designed to be lightweight and standalone. However, some advanced features leverage specialized external tools:

* **Fuzzy Search:** To enable fast, recursive fuzzy finding, install **[fd](https://github.com/sharkdp/fd)**.
  * If `fd` is detected in your `PATH`, the search feature will be enabled automatically.
  * Without it, `runa` remains a fully functional file manager but will notify you if you attempt a recursive search.
  * Also enables `move_file` auto-complete if `fd` is enabled.

* **Preview Syntax coloring**: To enable syntax coloring in the preview pane, install **[bat](https://github.com/sharkdp/bat)**
  * If `bat` is detected and installed, you can switch method in the runa.toml to `method = "bat"`.
  * Without it, `runa` uses the `internal` preview method, which is a plain preview method useful for extra speed without syntax highlighting.

## Configuration

By default, runa checks for configuration in these locations, in order:

1. If the environment variable `RUNA_CONFIG` is set, its value is used as the config path.
2. If `XDG_CONFIG_HOME` is set, runa looks for:  
    ```sh
    $XDG_CONFIG_HOME/runa/runa.toml
    ```

3. Otherwise, the fallback is:  
    ```sh
    $HOME/.config/runa/runa.toml
    ```

    (On Windows, this means inside the user's config folder.)

#### Overriding the config path

You can explicitly set a custom config location by setting the environment variable `RUNA_CONFIG`.

**Example:**  
```sh
export RUNA_CONFIG=/path/to/your/runa.toml
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

runa is in active development, with future releases focusing on expanding functionality while keeping it fast and lightweight.  

### Planned Features

- [ ] Image Previews: Support for Sixel/Kitty graphics protocols

### Completed Features

- [x] Syntax Highlighting: `bat` integration for the preview pane  
- [x] Search & Discovery: Integrated fuzzy finding (`fd` support)  
- [x] File Operations: Copy, move, delete, and rename from within the UI  
- [x] Content Search: Text search and filtering  
- [x] Performance: Reactive rendering  
- [x] UI Customization: Pane-specific styling and Hex color support  
- [x] Navigation Context: Persistent Parent (Origin) and Preview panes  

> See [CHANGELOG.md](./CHANGELOG.md) for version details and release history.

---


<img src="https://github.com/user-attachments/assets/778b80cc-3e6f-45ab-a770-e4d88059995a" alt="runa_default_3" style="max-width: 100%; height: auto;">


---

## Support & Contribute
If you enjoy using **runa**, you can help the project grow:

* ⭐ **Star the Repo:** It helps more people discover runa :)
* 🐛 **Report Bugs:** Open an issue if something doesn't work as expected.
* 💡 **Feature Requests:** Suggest new ideas in the [Discussions](https://github.com/alexm-dev/runa/discussions) tab.
* 🦀 **Contribute:** Pull requests are always welcome! Checkout the [CONTRIBUTING guide](CONTRIBUTING.md) for more info.


## Special Thanks
Many thanks to [@lmartinez-mirror](https://github.com/lmartinez-mirror), the maintainer of [runa-bin](https://aur.archlinux.org/packages/runa-bin), for maintaining the binary AUR package.

Many thanks to [@sharkdp](https://github.com/sharkdp) for `fd` and `bat`, excellent CLI tools runa integrates with for fuzzy finding and syntax preview.

## Credits & Ecosystem
`runa` stands on the shoulders of these incredible Rust crates:

- **Terminal UI:**
    - [Ratatui](https://ratatui.rs): Direct, fast terminal rendering.
    - [Crossterm](https://github.com/crossterm-rs/crossterm): Cross-platform terminal I/O.
- **Configuration:**
    - [Serde](https://serde.rs): Data serialization/deserialization.
    - [toml-rs](https://github.com/toml-rs/toml): TOML parsing.
- **Concurrency:**
    - [Crossbeam-channel](https://github.com/crossbeam-rs/crossbeam): Multi-threaded communication (worker threads).
- **Optional Integrations:**
    - [fd](https://github.com/sharkdp/fd): High-performance fuzzy finder (search enhancement).
    - [bat](https://github.com/sharkdp/bat): Syntax-highlighted file previews.

## License
This project is dual licensed under the MIT OR Apache-2.0 license.  

See the [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) files for details.
