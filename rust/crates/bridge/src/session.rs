//! Channel session mapping — maps channel identities to Kcode sessions.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// Composite key for identifying a unique session on a channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChannelSessionKey {
    /// Channel: "telegram", "feishu", "whatsapp", "loopback".
    pub channel: String,
    /// Tenant or bot identifier.
    pub tenant_or_bot_id: String,
    /// Chat scope: "dm", "group", "thread".
    pub chat_scope: String,
    /// User scope within the chat.
    pub user_scope: String,
    /// Optional thread scope.
    pub thread_scope: Option<String>,
}

impl ChannelSessionKey {
    pub fn new(
        channel: String,
        tenant_or_bot_id: String,
        chat_scope: String,
        user_scope: String,
    ) -> Self {
        Self {
            channel,
            tenant_or_bot_id,
            chat_scope,
            user_scope,
            thread_scope: None,
        }
    }

    pub fn with_thread(mut self, thread: String) -> Self {
        self.thread_scope = Some(thread);
        self
    }

    /// Stable string representation for use as map keys or session IDs.
    pub fn as_session_id(&self) -> String {
        let thread = self
            .thread_scope
            .as_deref()
            .unwrap_or("");
        format!(
            "{}:{}:{}:{}:{}",
            self.channel, self.tenant_or_bot_id, self.chat_scope, self.user_scope, thread
        )
    }
}

/// How sessions are mapped from channel identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMappingMode {
    /// One user gets one session (DM mode).
    OneToOne,
    /// One group gets one shared session.
    OneGroupOneSession,
    /// Each thread gets its own session.
    ThreadPerSession,
}

/// Session mapping registry — maps ChannelSessionKey to Kcode session IDs.
pub struct SessionMapping {
    mode: SessionMappingMode,
    mappings: Mutex<BTreeMap<ChannelSessionKey, String>>,
}

impl SessionMapping {
    pub fn new(mode: SessionMappingMode) -> Self {
        Self {
            mode,
            mappings: Mutex::new(BTreeMap::new()),
        }
    }

    /// Resolve or create a session ID for the given channel key.
    pub fn resolve_or_create(&self, key: &ChannelSessionKey, creator: impl FnOnce() -> String) -> String {
        let mut map = self.mappings.lock().expect("session mapping poisoned");

        if let Some(session_id) = map.get(key) {
            return session_id.clone();
        }

        let session_id = creator();
        map.insert(key.clone(), session_id.clone());
        session_id
    }

    /// Remove a session mapping (for cleanup / testing).
    pub fn remove(&self, key: &ChannelSessionKey) -> Option<String> {
        let mut map = self.mappings.lock().expect("session mapping poisoned");
        map.remove(key)
    }

    pub fn mode(&self) -> SessionMappingMode {
        self.mode
    }

    pub fn count(&self) -> usize {
        self.mappings.lock().expect("session mapping poisoned").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_to_session_id_is_stable() {
        let key = ChannelSessionKey::new(
            "loopback".into(),
            "bot1".into(),
            "dm".into(),
            "user1".into(),
        );
        let id1 = key.as_session_id();
        let id2 = key.as_session_id();
        assert_eq!(id1, id2);
        assert!(id1.contains("loopback"));
        assert!(id1.contains("user1"));
    }

    #[test]
    fn session_mapping_resolves_same_key() {
        let mapping = SessionMapping::new(SessionMappingMode::OneToOne);
        let key = ChannelSessionKey::new("lb".into(), "b1".into(), "dm".into(), "u1".into());

        let s1 = mapping.resolve_or_create(&key, || "session-1".into());
        let s2 = mapping.resolve_or_create(&key, || "session-2".into());

        assert_eq!(s1, s2);
        assert_eq!(s1, "session-1");
    }

    #[test]
    fn session_mapping_creates_new_for_different_key() {
        let mapping = SessionMapping::new(SessionMappingMode::OneToOne);
        let key1 = ChannelSessionKey::new("lb".into(), "b1".into(), "dm".into(), "u1".into());
        let key2 = ChannelSessionKey::new("lb".into(), "b1".into(), "dm".into(), "u2".into());

        let s1 = mapping.resolve_or_create(&key1, || "session-1".into());
        let s2 = mapping.resolve_or_create(&key2, || "session-2".into());

        assert_ne!(s1, s2);
        assert_eq!(mapping.count(), 2);
    }
}
