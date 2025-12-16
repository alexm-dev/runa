# runner

A fast and lightweight console file browser written in Rust

runner is a minimal terminal file browser focused on speed and simplicity.  

It allows you to naviagate directories, view file listings, and perform typical file browser actions.  

This project is a work in progess.  
It is being actively developed and will change over time.  

## Features
- Naviagate directories in the Terminal
- Lightweight and minimal memory usage
- Cross-platform: Works on Linux, Windows and macOS.
- Configurable keybindings via TOML configuration file.

## Installation
Clone the repo and build with Cargo:

```bash
https://github.com/alexm-dev/runner.git
cd runner
cargo build --release
```

## Configuration
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
runner --gen-config
```

This will generate a config in the default config path.

## Roadmap

runner is in active development.  
Future releases will focus on expanding functionality while keeping it fast and lightweight.  

Planned features for future releases include:

- Search functionality: search with other find or grep functions instead
- Optional preview: preview the files (text, images and other file types)
- Additional customization: fully customize the appearance of runner
- Predefined themes: choose from several built-in themes
- Performance improvements and UI enhancements

## License
MIT License
