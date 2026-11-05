use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};

use super::messages::auto_scroll_to_bottom;
use super::state::UiScale;
use super::ReplApp;

const SCROLL_STEP: usize = 3;

impl ReplApp {
    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.dialog.is_active() || self.history_search.active {
            return;
        }

        self.message_area_hovered = self.is_in_message_area(mouse.column, mouse.row);
        self.scrollbar_hovered = self.is_in_scrollbar_area(mouse.column, mouse.row);

        if self.picker.visible {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.picker.select_previous(),
                MouseEventKind::ScrollDown => self.picker.select_next(),
                _ => {}
            }
            return;
        }

        if mouse.modifiers.contains(KeyModifiers::CONTROL) {
            self.handle_zoom_mouse(mouse.kind);
            return;
        }

        if !(self.message_area_hovered || self.scrollbar_hovered) {
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_messages_up(SCROLL_STEP),
            MouseEventKind::ScrollDown => self.scroll_messages_down(SCROLL_STEP),
            MouseEventKind::Moved => {}
            _ => {}
        }
    }

    pub fn scroll_messages_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.stick_to_bottom = false;
    }

    pub fn scroll_messages_down(&mut self, lines: usize) {
        let max_offset = auto_scroll_to_bottom(
            &self.messages,
            self.message_area_height,
            self.message_area_width,
        );
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
        self.stick_to_bottom = self.scroll_offset >= max_offset;
    }

    fn handle_zoom_mouse(&mut self, kind: MouseEventKind) {
        let next_scale = match kind {
            MouseEventKind::ScrollUp => self.ui_scale.zoom_in(),
            MouseEventKind::ScrollDown => self.ui_scale.zoom_out(),
            _ => return,
        };
        self.set_ui_scale(next_scale);
    }

    pub fn set_ui_scale(&mut self, next_scale: UiScale) {
        if self.ui_scale == next_scale {
            return;
        }
        let bottom_gap = self.messages_max_scroll().saturating_sub(self.scroll_offset);
        let was_stuck = self.stick_to_bottom;
        self.ui_scale = next_scale;
        self.notify_info(format!("界面缩放: {}", self.ui_scale.label()));
        let inner_area = self.scaled_message_area(self.message_outer_area);
        self.set_message_viewport(inner_area);
        let next_max = self.messages_max_scroll();
        if was_stuck {
            self.scroll_to_bottom();
        } else {
            self.scroll_offset = next_max.saturating_sub(bottom_gap);
            self.stick_to_bottom = self.scroll_offset >= next_max;
        }
    }

    fn is_in_message_area(&self, column: u16, row: u16) -> bool {
        contains(self.message_area, column, row)
    }

    fn is_in_scrollbar_area(&self, column: u16, row: u16) -> bool {
        contains(self.scrollbar_area, column, row)
    }
}

fn contains(area: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    if area.width == 0 || area.height == 0 {
        return false;
    }
    let within_x = column >= area.x && column < area.x.saturating_add(area.width);
    let within_y = row >= area.y && row < area.y.saturating_add(area.height);
    within_x && within_y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::repl::RenderableMessage;

    fn mouse(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers,
        }
    }

    fn app_with_messages() -> ReplApp {
        let mut app = ReplApp::new(
            "gpt-5.4".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );
        app.message_outer_area = ratatui::layout::Rect::new(2, 1, 24, 6);
        let scaled = app.scaled_message_area(app.message_outer_area);
        app.set_message_viewport(scaled);
        for index in 0..20 {
            app.add_message(RenderableMessage::AssistantText {
                text: format!("message-{index} content content content"),
                streaming: false,
            });
        }
        app.scroll_messages_up(6);
        app.scrollbar_area = ratatui::layout::Rect::new(
            app.message_outer_area.x + app.message_outer_area.width - 1,
            app.message_outer_area.y,
            1,
            app.message_outer_area.height,
        );
        app
    }

    #[test]
    fn wheel_scrolls_only_inside_message_area() {
        let mut app = app_with_messages();
        let before = app.scroll_offset;

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 0, 0, KeyModifiers::NONE));
        assert_eq!(app.scroll_offset, before);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 3, 2, KeyModifiers::NONE));
        assert!(app.scroll_offset < before);
    }

    #[test]
    fn ctrl_wheel_changes_ui_scale_without_scrolling() {
        let mut app = app_with_messages();
        let before_scroll = app.scroll_offset;
        let before_gap = app.messages_max_scroll().saturating_sub(before_scroll);

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            3,
            2,
            KeyModifiers::CONTROL,
        ));

        assert_eq!(
            app.messages_max_scroll().saturating_sub(app.scroll_offset),
            before_gap
        );
        assert_eq!(app.ui_scale, UiScale::Comfortable);
    }

    #[test]
    fn wheel_scrolls_when_pointer_is_on_scrollbar() {
        let mut app = app_with_messages();
        let before = app.scroll_offset;
        let bar = app.scrollbar_area;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            bar.x,
            bar.y,
            KeyModifiers::NONE,
        ));

        assert!(app.scroll_offset > before);
    }
}
