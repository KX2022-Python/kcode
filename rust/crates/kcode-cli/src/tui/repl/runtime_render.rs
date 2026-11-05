use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use super::command_palette::render_slash_command_picker;
use super::dialog::render_dialog;
use super::diff_viewer::render_diff_viewer;
use super::header::header;
use super::layout::build_layout;
use super::messages::{message_line_count, render_messages};
use super::notification_render::render_notifications;
use super::permission::render_permission_dialog;
use super::prompt::{prompt_height, render_prompt_input};
use super::state::{PermissionRequest, UiScale};
use super::tool_group::group_tool_calls;
use super::ReplApp;

pub(super) fn draw_frame(frame: &mut ratatui::Frame<'_>, app: &mut ReplApp) {
    if !super::runtime_loop::terminal_area_usable(frame.area()) {
        return;
    }

    let prompt_h = prompt_height(&app.input, frame.area().width)
        .saturating_add(app.ui_scale.prompt_height_bonus());
    let layout = build_layout(frame.area(), prompt_h);
    let message_area = app.scaled_message_area(layout.messages);
    let prompt_area = app.scaled_message_area(layout.prompt);
    let content_margin = ratatui::layout::Margin {
        horizontal: message_area.x.saturating_sub(layout.messages.x),
        vertical: message_area.y.saturating_sub(layout.messages.y),
    };
    app.set_message_viewport(message_area);
    app.message_outer_area = layout.messages;

    let display_messages = if app.tools_collapsed {
        group_tool_calls(&app.messages)
    } else {
        app.messages.clone()
    };

    if layout.header.height > 0 {
        frame.render_widget(
            header(
                layout.header.width,
                &app.model,
                &app.profile,
                &app.session_id,
                &app.permission_mode_label,
                app.state.label(),
                app.palette,
            ),
            layout.header,
        );
    }

    render_notifications(frame, &mut app.notifications, frame.area(), app.palette);
    render_messages(
        frame,
        &display_messages,
        message_area,
        &mut app.scroll_offset,
        app.palette,
    );
    maybe_render_scrollbar(frame, app, &display_messages, layout.messages);

    let input_active = !app.permission_pending
        && !app.dialog.is_active()
        && !app.history_search.active
        && !app.diff_viewer.visible;
    render_prompt_input(frame, &app.input, prompt_area, input_active, app.palette);
    render_slash_command_picker(frame, &app.picker, prompt_area, frame.area(), app.palette);
    render_dialog(frame, &app.dialog, frame.area(), app.palette);
    render_diff_viewer(frame, &app.diff_viewer, frame.area(), app.palette);

    if app.permission_pending {
        let request = PermissionRequest::new(
            "example_tool".to_string(),
            "这是一个演示权限请求".to_string(),
        );
        render_permission_dialog(frame, &request, frame.area(), 0, app.palette);
    }

    app.footer_pills.has_active_query = app.state.is_active();
    app.footer_pills.has_pending_permission = app.permission_pending;
    app.footer_pills.has_notifications = !app.notifications.is_empty();
    if layout.footer.height > 0 {
        frame.render_widget(
            app.footer_pills.render(layout.footer.width, app.palette),
            layout.footer,
        );
    }
}

fn maybe_render_scrollbar(
    frame: &mut ratatui::Frame<'_>,
    app: &mut ReplApp,
    messages: &[super::RenderableMessage],
    viewport_outer: Rect,
) {
    let line_count = message_line_count(messages, app.message_area_width);
    let max_offset = line_count.saturating_sub(app.message_area_height as usize);
    let show_scrollbar = max_offset > 0 && (app.message_area_hovered || app.scrollbar_hovered);
    let scrollbar_area = Rect::new(
        viewport_outer.x.saturating_add(viewport_outer.width.saturating_sub(1)),
        viewport_outer.y,
        1,
        viewport_outer.height,
    );
    app.scrollbar_area = if show_scrollbar {
        scrollbar_area
    } else {
        Rect::default()
    };

    if !show_scrollbar {
        return;
    }

    let mut state = ScrollbarState::new(line_count)
        .position(app.scroll_offset)
        .viewport_content_length(app.message_area_height as usize);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("│"))
        .track_style(Style::default().fg(app.palette.border))
        .thumb_symbol("█")
        .thumb_style(Style::default().fg(app.palette.accent));
    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut state);
}
