//! Environment configuration validation.
//! Ensures all required variables are set and correctly formatted before startup.

use std::env;

use crate::apply_bridge_env_defaults_to_process;

/// Represents a validation error for an environment variable.
pub struct EnvError {
    pub var_name: String,
    pub message: String,
}

/// Validate the bridge environment configuration.
/// Returns a list of errors (empty if all valid).
pub fn validate_bridge_config() -> Vec<EnvError> {
    let mut errors = Vec::new();
    let _ = apply_bridge_env_defaults_to_process();

    // Check if at least one channel is configured
    let telegram_set = env::var("KCODE_TELEGRAM_BOT_TOKEN")
        .ok()
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let whatsapp_set = env::var("KCODE_WHATSAPP_PHONE_ID")
        .ok()
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let feishu_set = env::var("KCODE_FEISHU_APP_ID")
        .ok()
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let webhook_url = env::var("KCODE_WEBHOOK_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if !telegram_set && !whatsapp_set && !feishu_set {
        errors.push(EnvError {
            var_name: "CHANNEL_CONFIG".to_string(),
            message: "At least one channel must be configured (Telegram, WhatsApp, or Feishu)"
                .to_string(),
        });
    }

    // Validate Telegram config
    if telegram_set {
        let token = env::var("KCODE_TELEGRAM_BOT_TOKEN").unwrap_or_default();
        if !token.contains(':') {
            errors.push(EnvError {
                var_name: "KCODE_TELEGRAM_BOT_TOKEN".to_string(),
                message: "Invalid format. Expected '<bot_id>:<hash>'".to_string(),
            });
        }
    }

    if let Some(webhook) = webhook_url.as_deref() {
        if !telegram_set {
            errors.push(EnvError {
                var_name: "KCODE_WEBHOOK_URL".to_string(),
                message:
                    "Webhook URL only applies to Telegram bridge mode. Set KCODE_TELEGRAM_BOT_TOKEN or remove KCODE_WEBHOOK_URL."
                        .to_string(),
            });
        }
        if !webhook.starts_with("https://") {
            errors.push(EnvError {
                var_name: "KCODE_WEBHOOK_URL".to_string(),
                message:
                    "Telegram webhook URL must use a public HTTPS endpoint. Unset KCODE_WEBHOOK_URL to use long polling."
                        .to_string(),
            });
        }
        if webhook.contains("localhost") || webhook.contains("127.0.0.1") {
            errors.push(EnvError {
                var_name: "KCODE_WEBHOOK_URL".to_string(),
                message:
                    "Telegram webhook URL must be publicly reachable and cannot point at localhost or 127.0.0.1."
                        .to_string(),
            });
        }
        if !webhook.ends_with("/webhook/telegram") {
            errors.push(EnvError {
                var_name: "KCODE_WEBHOOK_URL".to_string(),
                message:
                    "Telegram webhook URL should end with /webhook/telegram to match the built-in local receiver."
                        .to_string(),
            });
        }
    }

    // Validate WhatsApp config
    if whatsapp_set {
        if env::var("KCODE_WHATSAPP_TOKEN")
            .ok()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            errors.push(EnvError {
                var_name: "KCODE_WHATSAPP_TOKEN".to_string(),
                message: "Required when KCODE_WHATSAPP_PHONE_ID is set".to_string(),
            });
        }
    }

    // Validate Feishu config
    if feishu_set {
        if env::var("KCODE_FEISHU_APP_SECRET")
            .ok()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            errors.push(EnvError {
                var_name: "KCODE_FEISHU_APP_SECRET".to_string(),
                message: "Required when KCODE_FEISHU_APP_ID is set".to_string(),
            });
        }
    }

    errors
}

/// Print a formatted summary of the current configuration.
pub fn print_config_summary() {
    let snapshot = apply_bridge_env_defaults_to_process().ok();
    println!("📋 Configuration Summary:");

    let channels = [
        (
            "Telegram",
            env::var("KCODE_TELEGRAM_BOT_TOKEN").ok().or_else(|| {
                snapshot
                    .as_ref()
                    .and_then(|env| env.resolve("KCODE_TELEGRAM_BOT_TOKEN"))
            }),
        ),
        (
            "WhatsApp",
            env::var("KCODE_WHATSAPP_PHONE_ID").ok().or_else(|| {
                snapshot
                    .as_ref()
                    .and_then(|env| env.resolve("KCODE_WHATSAPP_PHONE_ID"))
            }),
        ),
        (
            "Feishu",
            env::var("KCODE_FEISHU_APP_ID").ok().or_else(|| {
                snapshot
                    .as_ref()
                    .and_then(|env| env.resolve("KCODE_FEISHU_APP_ID"))
            }),
        ),
    ];

    for (name, value) in channels.iter() {
        let status = match value {
            Some(v) if !v.is_empty() => "✅ Active",
            _ => "⚪ Inactive",
        };
        println!("  {} {}", name, status);
    }

    if let Ok(model) = env::var("KCODE_MODEL") {
        println!("  Model: {}", model);
    }
    if let Ok(webhook) = env::var("KCODE_WEBHOOK_URL") {
        println!("  Telegram Delivery: Webhook via external HTTPS ingress");
        println!("  Webhook URL: {}", webhook);
        println!("  Local Receiver: http://0.0.0.0:3000/webhook/telegram");
    } else {
        println!("  Telegram Delivery: Long Polling (no public webhook URL configured)");
    }
}

#[cfg(test)]
mod tests {
    use super::validate_bridge_config;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn clear_bridge_env() {
        for key in [
            "KCODE_TELEGRAM_BOT_TOKEN",
            "KCODE_WHATSAPP_PHONE_ID",
            "KCODE_WHATSAPP_TOKEN",
            "KCODE_FEISHU_APP_ID",
            "KCODE_FEISHU_APP_SECRET",
            "KCODE_WEBHOOK_URL",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn rejects_non_public_telegram_webhook_urls() {
        let _guard = env_lock();
        clear_bridge_env();
        std::env::set_var("KCODE_TELEGRAM_BOT_TOKEN", "123456:test-token");
        std::env::set_var(
            "KCODE_WEBHOOK_URL",
            "http://localhost:3000/webhook/telegram",
        );

        let errors = validate_bridge_config();

        assert!(errors.iter().any(|error| {
            error.var_name == "KCODE_WEBHOOK_URL" && error.message.contains("public HTTPS")
        }));
        assert!(errors.iter().any(|error| {
            error.var_name == "KCODE_WEBHOOK_URL" && error.message.contains("localhost")
        }));

        clear_bridge_env();
    }

    #[test]
    fn rejects_webhook_url_without_telegram_channel() {
        let _guard = env_lock();
        clear_bridge_env();
        std::env::set_var("KCODE_WHATSAPP_PHONE_ID", "12345");
        std::env::set_var("KCODE_WHATSAPP_TOKEN", "wa-token");
        std::env::set_var("KCODE_WEBHOOK_URL", "https://example.com/webhook/telegram");

        let errors = validate_bridge_config();

        assert!(errors.iter().any(|error| {
            error.var_name == "KCODE_WEBHOOK_URL"
                && error.message.contains("only applies to Telegram")
        }));

        clear_bridge_env();
    }
}
