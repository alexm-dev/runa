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
use runa_tui::ui::render::layout_chunks;
use std::error;
use tempfile::tempdir;

#[test]
fn test_ui_sanitization_and_exact_width() {
    let pane_width = 10;

    let cases = vec![
        ("short.txt", 10),
        ("very_long_filename.txt", 10),
        ("🦀_crab.rs", 10),
        ("\t_tab", 10),
    ];

    for (input, expected_width) in cases {
        let result = core::sanitize_to_exact_width(input, pane_width);

        let actual_width = unicode_width::UnicodeWidthStr::width(result.as_str());

        assert_eq!(
            actual_width, expected_width,
            "Failed to produce exact width for input: '{}'. Result was: '{}' (width: {})",
            input, result, actual_width
        );

        assert!(
            !result.chars().any(|c| c.is_control() && c != ' '),
            "Result contains control characters: {:?}",
            result
        );
    }
}

#[test]
fn test_core_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let entries = core::browse_dir(temp_dir.path())?;

    assert!(entries.is_empty(), "Directory should be empty");
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
