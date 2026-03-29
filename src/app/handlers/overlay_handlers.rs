//! Overlay handlers for AppState.
//!
//! This module defines the overlay-related functions for the central
//! [app::state::handle_keypress] function.

use crate::app::state::{AppState, KeypressResult};
use crate::core::file_info::CachedFileInfo;
use crate::ui::overlays::{Overlay, OverlayKind};

use crossterm::event::{KeyCode::*, KeyEvent};
use std::sync::Arc;
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

        if self.overlays().is_open(OverlayKind::KeybindHelp) {
            self.overlays_mut().remove_kind(OverlayKind::KeybindHelp);
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
        if let (Some(new_info), Some(info)) = (
            self.selected_info_clone(),
            self.overlays_mut().find_show_info_mut(),
        ) {
            *info = new_info;
        }
    }

    /// Shows the file info overlay for the currently selected entry.
    fn show_file_info(&mut self) {
        if let Some(info) = self.selected_info_clone() {
            self.overlays_mut().push(Overlay::ShowInfo { info });
        }
    }

    /// Toggles the file info overlay.
    pub(super) fn toggle_file_info(&mut self) {
        let is_open = self.overlays().is_open(OverlayKind::ShowInfo);

        if is_open {
            self.overlays_mut().remove_kind(OverlayKind::ShowInfo);
        } else {
            self.show_file_info();
        }
    }

    pub(super) fn show_prefix_help(&mut self) {
        if !self.overlays().is_top(OverlayKind::PrefixHelp) {
            self.overlays_mut().push(Overlay::PrefixHelp);
        }
    }

    pub(crate) fn hide_prefix_help(&mut self) {
        if self.overlays().is_top(OverlayKind::PrefixHelp) {
            self.overlays_mut().pop();
        }
    }

    pub(super) fn toggle_keybind_help(&mut self) {
        let is_open = self.overlays().is_open(OverlayKind::KeybindHelp);

        if is_open {
            self.overlays_mut().remove_kind(OverlayKind::KeybindHelp);
        } else {
            self.overlays_mut().push(Overlay::KeybindHelp);
        }
    }

    /// Pushes a message overlay that lasts for the specified duration.
    pub(crate) fn push_overlay_message(&mut self, text: String, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);

        if self.overlays().is_top(OverlayKind::Message) {
            self.overlays_mut().pop();
        }

        self.overlays_mut().push(Overlay::Message { text });
    }

    fn selected_info_clone(&self) -> Option<Arc<CachedFileInfo>> {
        self.info.selected_info_arc().map(Arc::clone)
    }
}
