//! kcode-bridge v1.0 — Unified channel abstraction layer.
//!
//! The bridge normalizes inbound messages from channels, maps them to sessions,
//! filters commands by bridge-safe policy, and produces normalized outbound events.
//! It does NOT own session state — SessionEngine remains the single source of truth.

pub mod attachment;
pub mod events;
pub mod loopback;
pub mod policy;
pub mod session;

pub use attachment::{AttachmentEnvelope, AttachmentKind};
pub use events::{BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode};
pub use loopback::{LoopbackAdapter, LoopbackConfig, LoopbackMessage};
pub use policy::{BridgeCommandPolicy, CommandPolicyProfile};
pub use session::{ChannelSessionKey, SessionMapping, SessionMappingMode};
