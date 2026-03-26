//! Overlay handlers for AppState.
//!
//! This module defines the overlay-related functions for the central
//! [app::state::handle_keypress] function.

use crate::app::state::{AppState, KeypressResult};
use crate::core::file_info::FileInfo;
use crate::ui::overlays::Overlay;

use crossterm::event::{KeyCode::*, KeyEvent};
use std::time::{Duration, Instant};

/// AppState input and action handlers
impl<'a> AppState<'a> {
    pub(in crate::app) fn handle_esc_close_overlays(
        &mut self,
        key: &KeyEvent,
    ) -> Option<KeypressResult> {
        if key.code != Esc {
            return None;
        }

        self.actions.scroll().reset();

        if self
            .overlays()
            .iter()
            .any(|o| matches!(o, Overlay::KeybindHelp))
        {
            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::KeybindHelp));
            return Some(KeypressResult::Consumed);
        }
        None
    }

    /// Handles displaying a timed message overlay.
    pub(super) fn handle_timed_message(&mut self, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);
    }

    // Input processes

    /// Refreshes the file info overlay if it is currently open.
    pub(crate) fn refresh_show_info_if_open(&mut self) {
        let maybe_idx = self
            .overlays()
            .find_index(|o| matches!(o, Overlay::ShowInfo { .. }));
        let Some(i) = maybe_idx else { return };

        if let Some(cached) = &self.selected_info {
            let file_info = cached.info().clone();

            if let Some(Overlay::ShowInfo { info }) = self.overlays_mut().get_mut(i) {
                *info = file_info;
            }
        }
    }

    /// Shows the file info overlay for the currently selected entry.
    fn show_file_info(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let path = self.nav.current_dir().join(entry.name());
            if let Ok(file_info) = FileInfo::get_file_info(&path) {
                self.overlays_mut()
                    .push(Overlay::ShowInfo { info: file_info });
            }
        }
    }

    /// Toggles the file info overlay.
    pub(super) fn toggle_file_info(&mut self) {
        let is_open = self
            .overlays()
            .iter()
            .any(|o| matches!(o, Overlay::ShowInfo { .. }));

        if is_open {
            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::ShowInfo { .. }));
        } else {
            self.show_file_info();
        }
    }

    pub(super) fn show_prefix_help(&mut self) {
        if !matches!(self.overlays().top(), Some(Overlay::PrefixHelp)) {
            self.overlays_mut().push(Overlay::PrefixHelp);
        }
    }

    pub(crate) fn hide_prefix_help(&mut self) {
        if matches!(self.overlays().top(), Some(Overlay::PrefixHelp)) {
            self.overlays_mut().pop();
        }
    }

    pub(super) fn toggle_keybind_help(&mut self) {
        let is_open = self
            .overlays()
            .iter()
            .any(|o| matches!(o, Overlay::KeybindHelp));

        if is_open {
            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::KeybindHelp));
        } else {
            self.overlays_mut().push(Overlay::KeybindHelp);
        }
    }

    /// Pushes a message overlay that lasts for the specified duration.
    pub(crate) fn push_overlay_message(&mut self, text: String, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);

        if matches!(self.overlays.top(), Some(Overlay::Message { .. })) {
            self.overlays_mut().pop();
        }

        self.overlays_mut().push(Overlay::Message { text });
    }
}
