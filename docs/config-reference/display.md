# Display Settings

Configuration options for UI layout, panes, borders, previews, and file information display.

## Overview

The `[display]` section controls how runa's interface is laid out and rendered, including pane visibility, borders, preview options, and file information display.

## Main Options

### `selection_marker`

- **Type**: `boolean`
- **Default**: `false`

Show the selection icon next to the file/directory name.

### `dir_marker`

- **Type**: `boolean`
- **Default**: `true`

Show the default `/` symbol next to directory names.

### `borders`

- **Type**: `string`
- **Default**: `"unified"`
- **Options**: `"none"`, `"unified"`, `"split"`

Border style for the panes.

### `border_shape`

- **Type**: `string`
- **Default**: `"square"`
- **Options**: `"square"`, `"rounded"`, `"double"`

Border shape for the panes.

### `titles`

- **Type**: `boolean`
- **Default**: `true`

Show pane titles at the top (e.g., "Main", "Preview").

### `icons`

- **Type**: `boolean`
- **Default**: `false`

Show Nerd Font icons. Requires a Nerd Font to be installed and used.

### `separators`

- **Type**: `boolean`
- **Default**: `true`

Draw vertical lines between panes.

### `parent`

- **Type**: `boolean`
- **Default**: `true`

Show the parent directory pane (left).

### `preview`

- **Type**: `boolean`
- **Default**: `true`

Show the file preview pane (right).

### `preview_underline`

- **Type**: `boolean`
- **Default**: `true`

Enable underline in the preview pane.

### `preview_underline_color`

- **Type**: `boolean`
- **Default**: `false`

Use independent color for the preview underline.

### `entry_padding`

- **Type**: `integer`
- **Default**: `1`
- **Range**: `0–4`

Padding from entry to pane edge.

### `scroll_padding`

- **Type**: `integer`
- **Default**: `5`

Scroll padding of the main pane.

### `toggle_marker_jump`

- **Type**: `boolean`
- **Default**: `false`

Toggle if the marker selection should jump to the first entry whenever selection is at the bottom.

### `instant_preview`

- **Type**: `boolean`
- **Default**: `true`

Toggle previews to instantly render on every selection change. False results in pending preview when holding down a navigation key.

### `sort_date_format`

- **Type**: `string`
- **Default**: `"%b %e %H:%M"`

Configuration of the sorting date format when sorting by a date (Modified, Created, Accessed).
Uses the standard strftime-style format codes.

**Common specifiers:**
- `%Y` - 4 digit year (2026)
- `%y` - 2 digit year (26)
- `%m` - Month number (01-12)
- `%b` - Month name (Jan, Feb, etc.)
- `%d` - Day of the month
- `%H` - Hour
- `%M` - Minute
- `%S` - Second
- `%p` - AM/PM

**Examples:**
- `"%Y-%m-%d"` → 2026-04-04
- `"%d.%m.%Y %H:%M"` → 04.04.2026 14:30
- `"%b %d, %Y"` → Apr 04, 2026

This will show up in the sorting column next to entries.

## Preview Options

### `[display.preview_options]`

Options for the preview method used by the preview pane.

#### `method`

- **Type**: `string`
- **Default**: `"internal"`
- **Options**: `"internal"`, `"bat"`

Options: "internal" and "bat". For "bat" you will need to have `bat` installed otherwise it will fallback to internal.

#### `style` (when method = "bat")

- **Type**: `string`
- **Default**: `"plain"`
- **Options**: `"plain"`, `"numbers"`, `"full"`

Change the style of the `bat` preview method.

#### `theme` (when method = "bat")

- **Type**: `string`
- **Default**: `"default"`

Options to set the bat theme of the bat preview method. All available bat themes are supported. See `bat --list-themes`

#### `wrap` (when method = "bat")

- **Type**: `boolean`
- **Default**: `false`

Toggle wrapping the bat output to the pane width.
- If false, long lines stay on one line and go off-screen
- If true, all the lines are wrapped to the pane width

#### `tab_width` (when method = "bat")

- **Type**: `integer`
- **Default**: `4`

Configure the tab indentation of `bat`. This does not overwrite every file preview indentation.

## Layout Configuration

### `[display.layout]`

Display ratios for panes (will be scaled to 100%).

#### `parent`

- **Type**: `integer`
- **Default**: `20`

Parent pane width ratio.

#### `main`

- **Type**: `integer`
- **Default**: `40`

Main pane width ratio.

#### `preview`

- **Type**: `integer`
- **Default**: `40`

Preview pane width ratio.

## File Information Display

### `[display.info]`

Display the file info attributes.

#### `name`

- **Type**: `boolean`
- **Default**: `true`

Show file/directory name.

#### `file_type`

- **Type**: `boolean`
- **Default**: `true`

Show file type.

#### `size`

- **Type**: `boolean`
- **Default**: `true`

Show file size.

#### `modified`

- **Type**: `boolean`
- **Default**: `true`

Show modified date.

#### `created`

- **Type**: `boolean`
- **Default**: `true`

Show created date.

#### `accessed`

- **Type**: `boolean`
- **Default**: `false`

Show accessed date.

#### `perms`

- **Type**: `boolean`
- **Default**: `true`

Show file permissions.

#### `owner`

- **Type**: `boolean`
- **Default**: `true`

Toggle the Owner and Group file information for Unix. If disabled, then its also off for the status bar.

#### `group`

- **Type**: `boolean`
- **Default**: `true`

Show file group information (Unix only).

#### `position`

- **Type**: `string`
- **Default**: `"bottom_left"`

Position of the File Info widget.

#### `status_bar`

- **Type**: `boolean`
- **Default**: `true`

Enable file information on the status bar on the bottom left.

#### `format`

- **Type**: `string`
- **Default**: `"{perms} | {size}"`

Configure the format string of the file information on the status bar if enabled.

**Available keys**: `perms`, `size`, `mtime`/`modified`, `btime`/`created`, `atime`/`accessed`, `type`, and on unix additionally: `owner`, `group`.

#### `date_format`

- **Type**: `string`
- **Default**: `"%Y-%m-%d %H:%M"`

Configure the date format for file timestamps (Modified, Created, Accessed).
Uses the standard strftime-style format codes.

**Common specifiers:**
- `%Y` - 4 digit year (2026)
- `%y` - 2 digit year (26)
- `%m` - Month number (01-12)
- `%b` - Month name (Jan, Feb, etc.)
- `%d` - Day of the month
- `%H` - Hour
- `%M` - Minute
- `%S` - Second
- `%p` - AM/PM

**Examples:**
- `"%Y-%m-%d"` → 2026-04-04
- `"%d.%m.%Y %H:%M"` → 04.04.2026 14:30
- `"%b %d, %Y"` → Apr 04, 2026

## Status Line Configuration

### `[display.status]`

Display the status line options on the status line (Header or Footer).

**Available options**: `"footer"`, `"header"`, or `"none"` to disable

#### `entry_count`

- **Type**: `string`
- **Default**: `"footer"`

Position to display entry count.

#### `filter`

- **Type**: `string`
- **Default**: `"header"`

Position to display active filter status.

#### `markers`

- **Type**: `string`
- **Default**: `"footer"`

Position to display marker count.

#### `clipboard`

- **Type**: `string`
- **Default**: `"footer"`

Position to display clipboard status.

#### `tasks`

- **Type**: `string`
- **Default**: `"footer"`

Position to display background tasks status.

#### `tabs`

- **Type**: `string`
- **Default**: `"header"`

Position to display tab information.

