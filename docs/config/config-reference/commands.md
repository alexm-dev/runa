# Commands

Own commands bound to keys.

## Overview

The `[commands]` section binds programs to keys. Each command has a name, the keys that run it and the command itself. Commands start on demand and run like editor commands, in the terminal or in the background.

```toml
[commands]
lazygit = { keys = "<m-g>", cmd = "lazygit" }
extract = { keys = "x", cmd = ["7z", "x", "-o{unused_stem}", "{path}"], terminal = false }
```

## Options

### `keys`

- **Type**: `string or array of strings`

Keys that run the command. Uses the same syntax as [Key Bindings](keys.md). A command key replaces a built-in action on the same key.

The prefix keys (`g` and `o` by default) always start go-to and sort, so a command on one of them never runs.

### `cmd`

- **Type**: `string or array of strings`

The program and its arguments. Unlike editor commands, no path is appended.

[File variables](editor.md#file-variables) refer to the selected entry. A command that uses them needs a selected entry. `{stem}` and `{unused_stem}` drop only the last extension.

### `terminal`

- **Type**: `boolean`
- **Default**: `true`

With `true`, the command takes over the terminal, like an editor. With `false`, it runs in the background and runa shows a message if it fails. See [Running Without the Terminal](editor.md#running-without-the-terminal).

## Working Directory

Commands run in the directory runa shows.
