//! Session Router for multi-channel bridge.
//! Routes messages by chat_id to independent Kcode sessions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// A session entry holding the channel-specific state.
pub struct ChannelSession {
    /// Unique session identifier (derived from chat_id).
    pub session_id: String,
    /// Channel type (telegram, whatsapp, feishu).
    pub channel: String,
    /// The chat/user identifier for reply routing.
    pub chat_id: String,
}

/// Session Router manages multiple independent Kcode sessions.
/// Each unique chat_id gets its own session context.
pub struct SessionRouter {
    sessions: Mutex<HashMap<String, ChannelSession>>,
    session_dir: PathBuf,
}

impl SessionRouter {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            session_dir,
        }
    }

    fn session_map_key(channel: &str, chat_id: &str) -> String {
        format!("{channel}:{chat_id}")
    }

    pub fn session_id_for(channel: &str, chat_id: &str) -> String {
        format!("bridge-{channel}-{chat_id}")
    }

    fn sessions_guard(&self) -> MutexGuard<'_, HashMap<String, ChannelSession>> {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Get or create a session for the given channel/chat pair.
    pub fn get_or_create_session(&self, channel: &str, chat_id: &str) -> ChannelSession {
        let mut sessions = self.sessions_guard();
        let map_key = Self::session_map_key(channel, chat_id);
        if let Some(sess) = sessions.get(&map_key) {
            return sess.clone();
        }

        let session_id = Self::session_id_for(channel, chat_id);
        let session = ChannelSession {
            session_id: session_id.clone(),
            channel: channel.to_string(),
            chat_id: chat_id.to_string(),
        };
        sessions.insert(map_key, session.clone());
        session
    }

    /// Get the session path for a given channel/chat pair.
    pub fn session_path(&self, channel: &str, chat_id: &str) -> PathBuf {
        self.session_dir
            .join(format!("{}.jsonl", Self::session_id_for(channel, chat_id)))
    }

    /// List all active sessions.
    pub fn list_sessions(&self) -> Vec<ChannelSession> {
        let sessions = self.sessions_guard();
        sessions.values().cloned().collect()
    }

    /// Remove a session by channel/chat pair.
    pub fn remove_session(&self, channel: &str, chat_id: &str) {
        self.sessions_guard()
            .remove(&Self::session_map_key(channel, chat_id));
    }
}

impl Clone for ChannelSession {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id.clone(),
            channel: self.channel.clone(),
            chat_id: self.chat_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SessionRouter;
    use std::path::PathBuf;

    #[test]
    fn session_router_scopes_same_chat_id_by_channel() {
        let router = SessionRouter::new(PathBuf::from("/tmp/bridge-sessions"));

        let telegram = router.get_or_create_session("telegram", "42");
        let feishu = router.get_or_create_session("feishu", "42");

        assert_ne!(telegram.session_id, feishu.session_id);
        assert_eq!(router.list_sessions().len(), 2);
    }

    #[test]
    fn session_path_matches_channel_scoped_session_id() {
        let router = SessionRouter::new(PathBuf::from("/tmp/bridge-sessions"));
        let path = router.session_path("telegram", "42");

        assert_eq!(
            path,
            PathBuf::from("/tmp/bridge-sessions/bridge-telegram-42.jsonl")
        );
    }

    #[test]
    fn session_router_recovers_after_mutex_poison() {
        let router = SessionRouter::new(PathBuf::from("/tmp/bridge-sessions"));

        let _ = std::panic::catch_unwind(|| {
            let _guard = router.sessions.lock().expect("lock should be acquired");
            panic!("poison router mutex");
        });

        let session = router.get_or_create_session("telegram", "42");
        assert_eq!(session.session_id, "bridge-telegram-42");
        assert_eq!(router.list_sessions().len(), 1);
    }
}
