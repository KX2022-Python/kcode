use super::state::{RenderableMessage, ToolStatus};

/// 工具调用组 — 对齐 CC-Haha GroupedToolUseContent
/// 将同一轮次的多个工具调用折叠为一个可展开的行
#[derive(Debug, Clone)]
pub struct ToolUseGroup {
    pub tool_calls: Vec<RenderableMessage>,
    pub collapsed: bool,
    pub total_count: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub pending_count: usize,
}

impl ToolUseGroup {
    pub fn new() -> Self {
        Self {
            tool_calls: Vec::new(),
            collapsed: true,
            total_count: 0,
            success_count: 0,
            error_count: 0,
            pending_count: 0,
        }
    }

    pub fn add_tool(&mut self, msg: RenderableMessage) {
        match &msg {
            RenderableMessage::ToolCall { status, .. } => match status {
                ToolStatus::Completed => self.success_count += 1,
                ToolStatus::Denied => self.error_count += 1,
                ToolStatus::Pending | ToolStatus::Running => self.pending_count += 1,
            },
            RenderableMessage::ToolResult { is_error, .. } => {
                if *is_error {
                    self.error_count += 1;
                } else {
                    self.success_count += 1;
                }
            }
            _ => {}
        }
        self.tool_calls.push(msg);
        self.total_count += 1;
    }

    pub fn toggle(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// 渲染折叠/展开状态
    pub fn summary_line(&self) -> String {
        let icon = if self.collapsed { "▶" } else { "▼" };
        let status = if self.pending_count > 0 {
            format!(
                "{} 个工具调用 ({} 完成, {} 错误, {} 等待)",
                self.total_count, self.success_count, self.error_count, self.pending_count
            )
        } else {
            format!(
                "{} 个工具调用 ({} 完成, {} 错误)",
                self.total_count, self.success_count, self.error_count
            )
        };
        format!(" {} {}", icon, status)
    }
}

/// 将消息列表中的连续工具调用分组 — 对齐 CC-Haha applyGrouping()
pub fn group_tool_calls(messages: &[RenderableMessage]) -> Vec<RenderableMessage> {
    let mut result = Vec::new();
    let mut current_group: Option<ToolUseGroup> = None;

    for msg in messages {
        match msg {
            RenderableMessage::ToolCall { .. } | RenderableMessage::ToolResult { .. } => {
                if current_group.is_none() {
                    current_group = Some(ToolUseGroup::new());
                }
                current_group.as_mut().unwrap().add_tool(msg.clone());
            }
            _ => {
                // 刷新当前组
                if let Some(group) = current_group.take() {
                    if group.collapsed {
                        result.push(RenderableMessage::System {
                            message: group.summary_line(),
                            level: super::state::SysLevel::Info,
                        });
                    } else {
                        for tool_msg in group.tool_calls {
                            result.push(tool_msg);
                        }
                    }
                }
                result.push(msg.clone());
            }
        }
    }

    // 处理末尾的组
    if let Some(group) = current_group {
        if group.collapsed {
            result.push(RenderableMessage::System {
                message: group.summary_line(),
                level: super::state::SysLevel::Info,
            });
        } else {
            for tool_msg in group.tool_calls {
                result.push(tool_msg);
            }
        }
    }

    result
}
