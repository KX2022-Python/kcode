//! Session Router for multi-channel bridge.
//! Routes messages by chat_id to independent Kcode sessions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};

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

    /// Get or create a session for the given chat_id.
    pub fn get_or_create_session(&self, chat_id: &str, channel: &str) -> ChannelSession {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(sess) = sessions.get(chat_id) {
            return sess.clone();
        }

        let session_id = format!("bridge-{}-{}", channel, chat_id);
        let session = ChannelSession {
            session_id: session_id.clone(),
            channel: channel.to_string(),
            chat_id: chat_id.to_string(),
        };
        sessions.insert(chat_id.to_string(), session.clone());
        session
    }

    /// Get the session path for a given chat_id.
    pub fn session_path(&self, chat_id: &str) -> PathBuf {
        let sess = self.get_or_create_session(chat_id, "unknown");
        self.session_dir.join(format!("{}.jsonl", sess.session_id))
    }

    /// List all active sessions.
    pub fn list_sessions(&self) -> Vec<ChannelSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.values().cloned().collect()
    }

    /// Remove a session by chat_id.
    pub fn remove_session(&self, chat_id: &str) {
        self.sessions.lock().unwrap().remove(chat_id);
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
