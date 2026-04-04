/// 虚拟滚动窗口 — 对齐 CC-Haha VirtualMessageList
/// 只渲染可见区域的消息，避免大对话时的性能问题
pub struct VirtualWindow {
    /// 起始索引（消息数组中的位置）
    pub start_index: usize,
    /// 结束索引（不包含）
    pub end_index: usize,
    /// 可见区域高度（行数）
    pub viewport_height: usize,
    /// 总行数（所有消息的总行数）
    pub total_lines: usize,
    /// 垂直滚动偏移
    pub scroll_offset: usize,
}

impl VirtualWindow {
    pub fn new(viewport_height: usize) -> Self {
        Self {
            start_index: 0,
            end_index: 0,
            viewport_height,
            total_lines: 0,
            scroll_offset: 0,
        }
    }

    /// 更新窗口 — 计算当前应该渲染哪些消息
    pub fn update(&mut self, message_line_counts: &[usize], total_messages: usize) {
        if message_line_counts.is_empty() {
            self.start_index = 0;
            self.end_index = 0;
            self.total_lines = 0;
            return;
        }

        self.total_lines = message_line_counts.iter().sum();

        // 确保 scroll_offset 在有效范围内
        let max_scroll = self.total_lines.saturating_sub(self.viewport_height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        // 找到 scroll_offset 对应的消息索引
        let mut accumulated = 0;
        self.start_index = 0;
        for (i, &lines) in message_line_counts.iter().enumerate() {
            if accumulated + lines > self.scroll_offset {
                self.start_index = i;
                break;
            }
            accumulated += lines;
        }

        // 计算结束索引
        let mut visible_lines = 0;
        self.end_index = self.start_index;
        for &lines in message_line_counts[self.start_index..].iter() {
            if visible_lines + lines > self.viewport_height + 10 {
                // +10 为预渲染缓冲
                break;
            }
            visible_lines += lines;
            self.end_index += 1;
        }

        self.end_index = self.end_index.min(total_messages);
    }

    /// 滚动到顶部
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// 滚动到底部
    pub fn scroll_to_bottom(&mut self, message_line_counts: &[usize]) {
        self.total_lines = message_line_counts.iter().sum();
        let max_scroll = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = max_scroll;
    }

    /// 向上滚动
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// 向下滚动
    pub fn scroll_down(&mut self, lines: usize, max_scroll: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    /// 检查是否已滚动到底部
    pub fn is_at_bottom(&self) -> bool {
        let max_scroll = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset >= max_scroll.saturating_sub(2) // 2行容忍度
    }
}
