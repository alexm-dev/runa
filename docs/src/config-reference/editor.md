# Editor Configuration

Editor program selection and file association settings.

## Overview

The `[editor]` section controls which editor opens files and allows per-extension and per-filename overrides.

## Main Options

### `default`

- **Type**: `string`
- **Default**: `"nvim"`

Command to open files (e.g., "nvim", "code", "vim", etc.). This is the default editor used when no extension or filename override matches.

## File-Specific Overrides

### `ext`

- **Type**: `table or inline table`

Map file extensions to editors/program commands. Use single command or array of command + arguments.

**Inline format**:
```toml
ext = { rs = "nvim", md = "code", txt = ["code", "-d"] }
```

**Section format**:
```toml
[editor.ext]
rs = ["vim"]
md = ["code"]
```

### `filename`

- **Type**: `table or inline table`

Map specific filenames to editors/program commands. Use single command or array of command + arguments.

**Inline format**:
```toml
filename = { "Cargo.toml" = "nvim" }
```

**Section format**:
```toml
[editor.filename]
"Cargo.toml" = ["vim"]
```

## Editor Resolution

runa resolves the editor to use in the following order:

1. Check if there's a filename override for the exact filename
2. Check if there's an extension override for the file extension
3. Use the default editor

The resolved editor can be a single command string or an array of command + arguments. If you set a custom filename to a certain program/cmd, the actual file path will be handled in the actual file path directory and never in the starting directory of runa.

