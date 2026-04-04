use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::message_row::render_message;
use super::state::RenderableMessage;

/// 消息列表渲染 — 对齐 CC-Haha VirtualMessageList
/// 使用简单的滚动列表（暂不实现虚拟滚动，后续优化）
pub fn render_messages(
    frame: &mut Frame<'_>,
    messages: &[RenderableMessage],
    area: Rect,
    scroll_offset: &mut usize,
) {
    if messages.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(vec![ratatui::text::Span::styled(
                "  输入消息开始对话，或输入 / 查看命令",
                Style::default().fg(Color::Gray),
            )]),
        ]);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = messages
        .iter()
        .flat_map(|msg| {
            let lines = render_message(msg);
            lines.into_iter().map(ListItem::new).collect::<Vec<_>>()
        })
        .collect();

    let message_count = items.len();
    let mut list_state = ListState::default();

    // 滚动控制：确保滚动偏移在有效范围内
    let visible_height = area.height as usize;
    let max_offset = message_count.saturating_sub(visible_height);
    if *scroll_offset > max_offset {
        *scroll_offset = max_offset;
    }

    list_state.select(Some(*scroll_offset));

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::Rgb(30, 30, 30))
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// 计算当前应该显示的滚动位置（自动滚动到底部）
pub fn auto_scroll_to_bottom(messages: &[RenderableMessage], area_height: u16) -> usize {
    let total_lines = messages
        .iter()
        .map(|m| render_message(m).len())
        .sum::<usize>()
        .max(1);
    total_lines.saturating_sub(area_height as usize)
}
