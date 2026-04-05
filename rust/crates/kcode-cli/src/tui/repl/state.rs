use std::time::Instant;

/// 会话状态机 — 对齐 CC-Haha QueryGuard + streamMode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// 空闲，等待用户输入
    Idle,
    /// 已发送请求，等待首字节
    Requesting { start: Instant },
    /// 模型在思考（thinking blocks）
    Thinking { text: String },
    /// 模型在输出文本
    Responding { text: String },
    /// 工具调用等待执行
    ToolUse { tool_name: String, input: String },
    /// 工具执行中
    ToolRunning { tool_name: String },
    /// 等待用户授权
    WaitingPermission {
        tool_name: String,
        input_summary: String,
    },
    /// 错误
    Error { message: String },
    /// 完成一轮
    Completed { summary: String },
}

impl SessionState {
    pub fn is_active(&self) -> bool {
        !matches!(
            self,
            SessionState::Idle | SessionState::Error { .. } | SessionState::Completed { .. }
        )
    }

    pub fn is_waiting_permission(&self) -> bool {
        matches!(self, SessionState::WaitingPermission { .. })
    }

    pub fn label(&self) -> &str {
        match self {
            SessionState::Idle => "idle",
            SessionState::Requesting { .. } => "requesting",
            SessionState::Thinking { .. } => "thinking",
            SessionState::Responding { .. } => "responding",
            SessionState::ToolUse { .. } => "tool_use",
            SessionState::ToolRunning { .. } => "tool_running",
            SessionState::WaitingPermission { .. } => "waiting_permission",
            SessionState::Error { .. } => "error",
            SessionState::Completed { .. } => "completed",
        }
    }
}

/// 可渲染的消息类型 — 对齐 CC-Haha MessageRow 类型系统
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderableMessage {
    /// 用户消息
    User { text: String },
    /// 助手文本
    AssistantText { text: String, streaming: bool },
    /// 助手思考中
    AssistantThinking { text: String },
    /// 工具调用
    ToolCall {
        name: String,
        input: String,
        status: ToolStatus,
    },
    /// 工具结果
    ToolResult {
        name: String,
        output: String,
        is_error: bool,
    },
    /// 系统消息
    System { message: String, level: SysLevel },
    /// 压缩边界
    CompactBoundary,
    /// 错误消息
    Error { message: String },
    /// 成本/用量
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        cost: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmittedCommand {
    Prompt(String),
    Slash(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackendResult {
    pub messages: Vec<RenderableMessage>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub ui_state: Option<RuntimeUiState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeUiState {
    pub model: String,
    pub profile: String,
    pub session_id: String,
    pub permission_mode_label: String,
    pub profile_supports_tools: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Pending,
    Running,
    Completed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// 权限请求 — 对齐 CC-Haha ToolUseConfirm
pub struct PermissionRequest {
    pub tool_name: String,
    pub input_summary: String,
    pub decision: Option<PermissionDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    AllowAlways,
    Deny,
    DenyAlways,
}

impl PermissionRequest {
    pub fn new(tool_name: String, input_summary: String) -> Self {
        Self {
            tool_name,
            input_summary,
            decision: None,
        }
    }
}
