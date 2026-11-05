use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// REPL 全屏布局
pub struct ReplLayout {
    pub header: Rect,
    pub messages: Rect,
    pub prompt: Rect,
    pub footer: Rect,
}

pub fn build_layout(area: Rect, prompt_height: u16) -> ReplLayout {
    let footer_height = if area.height > prompt_height + 4 {
        1
    } else {
        0
    };
    let header_height = if area.height > prompt_height + footer_height + 2 {
        1
    } else {
        0
    };
    let prompt_height = prompt_height
        .min(
            area.height
                .saturating_sub(header_height + footer_height + 1),
        )
        .max(3);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(3),
            Constraint::Length(prompt_height),
            Constraint::Length(footer_height),
        ])
        .split(area);

    ReplLayout {
        header: chunks[0],
        messages: chunks[1],
        prompt: chunks[2],
        footer: chunks[3],
    }
}

#[cfg(test)]
mod tests {
    use super::build_layout;
    use ratatui::layout::Rect;

    #[test]
    fn prompt_height_scales_without_collapsing_message_area() {
        let layout = build_layout(
            Rect {
                x: 0,
                y: 0,
                width: 120,
                height: 32,
            },
            6,
        );

        assert_eq!(layout.header.height, 1);
        assert_eq!(layout.prompt.height, 6);
        assert!(layout.messages.height >= 3);
    }
}
