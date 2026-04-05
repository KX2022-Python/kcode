use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::notifications::{Notification, NotificationPriority, NotificationQueue};
use super::theme::ThemePalette;

/// 渲染通知栏 — 对齐 CC-Haha Notifications.tsx 单行绝对定位
pub fn render_notifications(
    frame: &mut Frame<'_>,
    queue: &mut NotificationQueue,
    area: Rect,
    palette: ThemePalette,
) {
    // 清理过期通知
    queue.cleanup();

    let active = queue.active();
    if active.is_empty() {
        return;
    }

    // 只显示最高优先级的通知
    if let Some(notification) = active.first() {
        let n = *notification;
        let notif_area = render_single_notification(frame, n, area, palette);
        // 3s 后自动dismiss
        if n.is_expired() {
            queue.dismiss(n.id);
        }
    }
}

fn render_single_notification(
    frame: &mut Frame<'_>,
    notification: &Notification,
    area: Rect,
    palette: ThemePalette,
) -> Rect {
    let (icon, text_color, border_color) = match notification.priority {
        NotificationPriority::Low => (
            "ℹ",
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::DIM),
            palette.text_muted,
        ),
        NotificationPriority::Medium => ("●", Style::default().fg(palette.info), palette.info),
        NotificationPriority::High => (
            "★",
            Style::default()
                .fg(palette.success)
                .add_modifier(Modifier::BOLD),
            palette.success,
        ),
        NotificationPriority::Immediate => (
            "⚠",
            Style::default()
                .fg(palette.warning)
                .add_modifier(Modifier::BOLD),
            palette.warning,
        ),
    };

    let lines = vec![Line::from(vec![
        Span::styled(icon, text_color),
        Span::raw(" "),
        Span::styled(&notification.message, text_color),
    ])];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(palette.dialog_bg));

    let paragraph = Paragraph::new(lines).block(block);

    // 顶部居中显示，宽度 60，高度 3
    let width = 60.min(area.width.saturating_sub(4));
    let height = 3;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + 1; // 在 header 下方

    let notif_rect = Rect {
        x,
        y,
        width,
        height,
    };

    frame.render_widget(Clear, notif_rect);
    frame.render_widget(paragraph, notif_rect);

    notif_rect
}
