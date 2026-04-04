//! Environment configuration validation.
//! Ensures all required variables are set and correctly formatted before startup.

use std::collections::HashMap;
use std::env;

/// Represents a validation error for an environment variable.
pub struct EnvError {
    pub var_name: String,
    pub message: String,
}

/// Validate the bridge environment configuration.
/// Returns a list of errors (empty if all valid).
pub fn validate_bridge_config() -> Vec<EnvError> {
    let mut errors = Vec::new();

    // Check if at least one channel is configured
    let telegram_set = env::var("KCODE_TELEGRAM_BOT_TOKEN").ok().map(|v| !v.is_empty()).unwrap_or(false);
    let whatsapp_set = env::var("KCODE_WHATSAPP_PHONE_ID").ok().map(|v| !v.is_empty()).unwrap_or(false);
    let feishu_set = env::var("KCODE_FEISHU_APP_ID").ok().map(|v| !v.is_empty()).unwrap_or(false);

    if !telegram_set && !whatsapp_set && !feishu_set {
        errors.push(EnvError {
            var_name: "CHANNEL_CONFIG".to_string(),
            message: "At least one channel must be configured (Telegram, WhatsApp, or Feishu)".to_string(),
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

    // Validate WhatsApp config
    if whatsapp_set {
        if env::var("KCODE_WHATSAPP_TOKEN").ok().map(|v| v.is_empty()).unwrap_or(true) {
            errors.push(EnvError {
                var_name: "KCODE_WHATSAPP_TOKEN".to_string(),
                message: "Required when KCODE_WHATSAPP_PHONE_ID is set".to_string(),
            });
        }
    }

    // Validate Feishu config
    if feishu_set {
        if env::var("KCODE_FEISHU_APP_SECRET").ok().map(|v| v.is_empty()).unwrap_or(true) {
            errors.push(EnvError {
                var_name: "KCODE_FEISHU_APP_SECRET".to_string(),
                message: "Required when KCODE_FEISHU_APP_ID is set".to_string(),
            });
        }
    }

    // Validate core API config
    if env::var("KCODE_API_KEY").ok().map(|v| v.is_empty()).unwrap_or(true) {
        errors.push(EnvError {
            var_name: "KCODE_API_KEY".to_string(),
            message: "Kcode API key is required".to_string(),
        });
    }

    if env::var("KCODE_MODEL").ok().map(|v| v.is_empty()).unwrap_or(true) {
        errors.push(EnvError {
            var_name: "KCODE_MODEL".to_string(),
            message: "Model name is required".to_string(),
        });
    }

    errors
}

/// Print a formatted summary of the current configuration.
pub fn print_config_summary() {
    println!("📋 Configuration Summary:");
    
    let channels = [
        ("Telegram", env::var("KCODE_TELEGRAM_BOT_TOKEN").ok()),
        ("WhatsApp", env::var("KCODE_WHATSAPP_PHONE_ID").ok()),
        ("Feishu", env::var("KCODE_FEISHU_APP_ID").ok()),
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
        println!("  Webhook: {}", webhook);
    } else {
        println!("  Webhook: Using Long Polling");
    }
}
