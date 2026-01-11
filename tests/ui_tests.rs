//! UI-related tests for runa
//!
//! These tests focus on the user interface components of the runa TUI application,
//! including formatting and layout rendering.
//! They ensure that the UI behaves correctly under various conditions.
//!
//! These tests may create temporary directories and files to simulate different UI scenarios.
//! These temporary resources are automatically cleaned up after the tests complete.

use ratatui::layout::Rect;
use runa_tui::app::AppState;
use runa_tui::config::{Config, load::RawConfig};
use runa_tui::core;
use runa_tui::core::Formatter;
use runa_tui::ui::render::layout_chunks;
use std::collections::HashSet;
use std::error;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_formatter_truncation_and_padding() -> Result<(), Box<dyn error::Error>> {
    let width = 10;
    let formatter = Formatter::new(true, true, true, false, Arc::new(HashSet::new()), width);

    let path = Path::new(".");
    let mut entries = core::browse_dir(path)?;

    formatter.format(&mut entries);

    assert!(
        !entries.is_empty(),
        "Should have found files in project root"
    );

    for entry in entries {
        let disp = entry.display_name();
        let visual_width = unicode_width::UnicodeWidthStr::width(disp);

        // every single line must be exactly the pane width
        assert_eq!(
            visual_width,
            width,
            "Visual width mismatch for '{}'. Got {}, expected {}",
            entry.name_str(),
            visual_width,
            width
        );

        let suffix_len = if entry.is_dir() { 1 } else { 0 };
        if entry.name_str().chars().count() + suffix_len > width {
            assert!(
                disp.contains('…'),
                "Long file '{}' (len {}) should contain ellipsis '…' at width {}",
                entry.name_str(),
                entry.name_str().chars().count(),
                width
            );
        }

        // Special check for crab file if it exists in the directory
        if entry.name_str().contains('🦀') {
            // If the crab file is short, ensure it was padded correctly
            if visual_width == width && !disp.contains('…') {
                assert!(
                    disp.ends_with(' '),
                    "Short crab file should be space-padded"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn test_formatter_empty_dir() -> Result<(), Box<dyn error::Error>> {
    let width = 15;
    let formatter = Formatter::new(true, true, true, false, Arc::new(HashSet::new()), width);

    let temp_dir = tempdir()?;

    let empty_path = temp_dir.path();
    let mut entries = core::browse_dir(empty_path)?;
    formatter.format(&mut entries);

    for entry in entries {
        let disp = entry.display_name();
        assert!(
            disp.chars().count() <= width,
            "Entry '{}' exceeds width",
            entry.name_str()
        );
    }
    Ok(())
}

#[test]
fn test_layout_chunks_with_config() -> Result<(), Box<dyn error::Error>> {
    let size = Rect::new(0, 0, 100, 10);

    // define a config string where ratios = 150%
    let toml_content = r#"
            [display]
            parent = true
            preview = true
            separators = false

            [display.layout]
            parent = 50
            main = 50
            preview = 50
        "#;

    let raw: RawConfig = toml::from_str(toml_content)?;
    let config = Config::from(raw);

    let app = AppState::new(&config).expect("Failed to create AppState");

    let chunks = layout_chunks(size, &app);

    assert_eq!(chunks.len(), 3);
    let total_width: u16 = chunks.iter().map(|c| c.width).sum();

    assert!(total_width <= 100);
    assert!(chunks[0].width >= 33 && chunks[0].width <= 34);
    Ok(())
}
