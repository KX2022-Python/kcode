//! Cross-platform media file downloader.
//! Downloads files from Telegram, WhatsApp, and Feishu to local storage.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for generating unique filenames.
static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Resolve the media storage directory: ~/.kcode/bridge-media/
pub fn media_storage_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".kcode").join("bridge-media")
}

/// Ensure the media directory exists.
pub fn ensure_media_dir() -> std::io::Result<()> {
    let dir = media_storage_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// Generate a unique local filename based extension.
fn generate_filename(extension: &str) -> String {
    let id = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("media_{:010}.{}", id, extension)
}

/// Download a file from a URL to the local media directory.
pub async fn download_file(
    url: &str,
    token: Option<&str>,
    extension: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    ensure_media_dir()?;

    let client = reqwest::Client::new();
    let mut request = client.get(url);

    if let Some(t) = token {
        request = request.header("Authorization", t);
    }

    let resp = request.send().await?;
    if !resp.status().is_success() {
        return Err(format!("Download failed: {}", resp.status()).into());
    }

    let bytes = resp.bytes().await?;
    let filename = generate_filename(extension);
    let path = media_storage_dir().join(&filename);

    tokio::fs::write(&path, &bytes).await?;

    Ok(path)
}

/// Download a file from Telegram.
pub async fn download_telegram_file(
    bot_token: &str,
    file_id: &str,
    extension: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Step 1: Get file path from Telegram API
    let api_url = format!("https://api.telegram.org/bot{}/getFile?file_id={}", bot_token, file_id);
    let client = reqwest::Client::new();
    let resp = client.get(&api_url).send().await?;
    let json: serde_json::Value = resp.json().await?;

    if !json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(format!("Telegram getFile failed: {}", json).into());
    }

    let file_path = json["result"]["file_path"]
        .as_str()
        .ok_or("Missing file_path in response")?;

    let download_url = format!("https://api.telegram.org/file/bot{}/{}", bot_token, file_path);

    // Step 2: Download the actual file
    download_file(&download_url, None, extension).await
}

/// Download a file from WhatsApp Cloud API.
pub async fn download_whatsapp_file(
    media_id: &str,
    access_token: &str,
    extension: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Step 1: Get media URL
    let api_url = format!("https://graph.facebook.com/v18.0/{}", media_id);
    let client = reqwest::Client::new();
    let resp = client
        .get(&api_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;
    let json: serde_json::Value = resp.json().await?;

    let url = json["url"]
        .as_str()
        .ok_or("Missing url in WhatsApp media response")?;

    // Step 2: Download the actual file
    download_file(url, Some(access_token), extension).await
}

/// Download a file from Feishu.
pub async fn download_feishu_file(
    image_key: &str,
    tenant_token: &str,
    extension: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://open.feishu.cn/open-apis/im/v1/images/{}/download",
        image_key
    );

    download_file(&url, Some(&format!("Bearer {}", tenant_token)), extension).await
}
