use std::time::Instant;

/// 通知优先级 — 对齐 CC-Haha Notification 优先级体系
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationPriority {
    /// 低优先级：提示性信息，12s 超时
    Low,
    /// 中优先级：状态变更，8s 超时
    Medium,
    /// 高优先级：重要确认，5s 超时
    High,
    /// 紧急：错误/警告，3s 超时
    Immediate,
}

impl NotificationPriority {
    pub fn timeout_secs(&self) -> u64 {
        match self {
            NotificationPriority::Low => 12,
            NotificationPriority::Medium => 8,
            NotificationPriority::High => 5,
            NotificationPriority::Immediate => 3,
        }
    }
}

/// 通知条目 — 对齐 CC-Haha Notifications.tsx
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u64,
    pub message: String,
    pub priority: NotificationPriority,
    pub created_at: Instant,
    pub dismissed: bool,
}

impl Notification {
    pub fn new(id: u64, message: String, priority: NotificationPriority) -> Self {
        Self {
            id,
            message,
            priority,
            created_at: Instant::now(),
            dismissed: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= self.priority.timeout_secs()
    }
}

/// 通知队列管理器 — 对齐 CC-Haha notification queue
pub struct NotificationQueue {
    notifications: Vec<Notification>,
    next_id: u64,
}

impl NotificationQueue {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            next_id: 1,
        }
    }

    pub fn push(&mut self, message: String, priority: NotificationPriority) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.notifications
            .push(Notification::new(id, message, priority));
        id
    }

    pub fn push_info(&mut self, message: String) -> u64 {
        self.push(message, NotificationPriority::Medium)
    }

    pub fn push_success(&mut self, message: String) -> u64 {
        self.push(message, NotificationPriority::High)
    }

    pub fn push_warning(&mut self, message: String) -> u64 {
        self.push(message, NotificationPriority::Immediate)
    }

    pub fn push_error(&mut self, message: String) -> u64 {
        self.push(message, NotificationPriority::Immediate)
    }

    pub fn dismiss(&mut self, id: u64) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) {
            n.dismissed = true;
        }
    }

    pub fn dismiss_all(&mut self) {
        self.notifications.iter_mut().for_each(|n| n.dismissed = true);
    }

    /// 获取当前应显示的通知（未过期 + 未解除）
    pub fn active(&self) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|n| !n.dismissed && !n.is_expired())
            .collect()
    }

    /// 清理过期通知
    pub fn cleanup(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    /// 获取队列长度
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}
