//! Attachment normalization — unified attachment envelope for bridge messages.

/// Kinds of attachments the bridge understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// Plain text attachment.
    Text,
    /// Image (png, jpg, gif, webp).
    Image,
    /// Document (pdf, txt, md, etc.).
    Document,
    /// Quoted/replied message reference.
    QuotedMessage,
    /// Platform-specific metadata attachment.
    PlatformMetadata,
}

impl AttachmentKind {
    pub fn from_mime(mime: &str) -> Self {
        if mime.starts_with("text/") {
            Self::Text
        } else if mime.starts_with("image/") {
            Self::Image
        } else if mime.starts_with("application/") {
            Self::Document
        } else {
            Self::PlatformMetadata
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Document => "document",
            Self::QuotedMessage => "quoted_message",
            Self::PlatformMetadata => "platform_metadata",
        }
    }
}

/// Normalized attachment that channels produce and the bridge consumes.
#[derive(Debug, Clone)]
pub struct AttachmentEnvelope {
    pub kind: AttachmentKind,
    /// Human-readable name or description.
    pub name: String,
    /// MIME type, if known.
    pub mime_type: Option<String>,
    /// Text content (for text attachments).
    pub text_content: Option<String>,
    /// URL or local path for media/document attachments.
    pub url_or_path: Option<String>,
    /// Size in bytes, if known.
    pub size_bytes: Option<u64>,
}

impl AttachmentEnvelope {
    /// Create a text attachment.
    pub fn text(name: String, content: String) -> Self {
        let len = content.len() as u64;
        Self {
            kind: AttachmentKind::Text,
            name,
            mime_type: Some("text/plain".into()),
            text_content: Some(content),
            url_or_path: None,
            size_bytes: Some(len),
        }
    }

    /// Create an image attachment.
    pub fn image(name: String, url_or_path: String) -> Self {
        Self {
            kind: AttachmentKind::Image,
            name,
            mime_type: None,
            text_content: None,
            url_or_path: Some(url_or_path),
            size_bytes: None,
        }
    }

    /// Create a quoted message reference.
    pub fn quoted_message(text: String, original_sender: String) -> Self {
        Self {
            kind: AttachmentKind::QuotedMessage,
            name: format!("reply from {original_sender}"),
            mime_type: None,
            text_content: Some(text),
            url_or_path: None,
            size_bytes: None,
        }
    }

    /// Check if this attachment type is allowed by a basic policy.
    pub fn is_allowed(&self, allowed_kinds: &[AttachmentKind]) -> bool {
        allowed_kinds.contains(&self.kind)
    }
}

/// Default attachment policy — what kinds are accepted.
pub fn default_attachment_allowlist() -> Vec<AttachmentKind> {
    vec![
        AttachmentKind::Text,
        AttachmentKind::Image,
        AttachmentKind::Document,
        AttachmentKind::QuotedMessage,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_attachment_has_expected_fields() {
        let att = AttachmentEnvelope::text("notes.txt".into(), "hello".into());
        assert_eq!(att.kind, AttachmentKind::Text);
        assert_eq!(att.text_content.as_deref(), Some("hello"));
    }

    #[test]
    fn attachment_kind_from_mime_maps_correctly() {
        assert_eq!(AttachmentKind::from_mime("text/plain"), AttachmentKind::Text);
        assert_eq!(
            AttachmentKind::from_mime("image/png"),
            AttachmentKind::Image
        );
        assert_eq!(
            AttachmentKind::from_mime("application/pdf"),
            AttachmentKind::Document
        );
    }

    #[test]
    fn default_allowlist_accepts_text() {
        let allowed = default_attachment_allowlist();
        let att = AttachmentEnvelope::text("x".into(), "y".into());
        assert!(att.is_allowed(&allowed));
    }
}
