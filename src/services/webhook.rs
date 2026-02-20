use crate::models::{Webhook, WebhookDelivery};
use crate::Database;
use anyhow::Result;

/// Create a new webhook.
pub fn create_webhook(
    db: &Database,
    name: &str,
    url: &str,
    secret: Option<&str>,
    events: &str,
) -> Result<i64> {
    let conn = db.get()?;
    conn.execute(
        "INSERT INTO webhooks (name, url, secret, events) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, url, secret, events],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Update an existing webhook.
pub fn update_webhook(
    db: &Database,
    id: i64,
    name: &str,
    url: &str,
    secret: Option<&str>,
    events: &str,
    active: bool,
) -> Result<()> {
    let conn = db.get()?;
    conn.execute(
        "UPDATE webhooks SET name = ?1, url = ?2, secret = ?3, events = ?4, active = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
        rusqlite::params![name, url, secret, events, active, id],
    )?;
    Ok(())
}

/// Delete a webhook by ID.
pub fn delete_webhook(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM webhooks WHERE id = ?", [id])?;
    Ok(())
}

/// List all webhooks.
pub fn list_webhooks(db: &Database) -> Result<Vec<Webhook>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, url, secret, events, active, created_at, updated_at FROM webhooks ORDER BY created_at DESC",
    )?;
    let webhooks = stmt
        .query_map([], |row| {
            Ok(Webhook {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                secret: row.get(3)?,
                events: row.get(4)?,
                active: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(webhooks)
}

/// Get a single webhook by ID.
pub fn get_webhook(db: &Database, id: i64) -> Result<Option<Webhook>> {
    let conn = db.get()?;
    let webhook = conn
        .query_row(
            "SELECT id, name, url, secret, events, active, created_at, updated_at FROM webhooks WHERE id = ?",
            [id],
            |row| {
                Ok(Webhook {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    secret: row.get(3)?,
                    events: row.get(4)?,
                    active: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .ok();
    Ok(webhook)
}

/// List recent deliveries for a webhook.
pub fn list_deliveries(db: &Database, webhook_id: i64, limit: i64) -> Result<Vec<WebhookDelivery>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, webhook_id, event, payload, response_status, response_body, success, attempts, delivered_at
         FROM webhook_deliveries WHERE webhook_id = ? ORDER BY delivered_at DESC LIMIT ?",
    )?;
    let deliveries = stmt
        .query_map(rusqlite::params![webhook_id, limit], |row| {
            Ok(WebhookDelivery {
                id: row.get(0)?,
                webhook_id: row.get(1)?,
                event: row.get(2)?,
                payload: row.get(3)?,
                response_status: row.get(4)?,
                response_body: row.get(5)?,
                success: row.get(6)?,
                attempts: row.get(7)?,
                delivered_at: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(deliveries)
}

/// Fire webhooks for a given event. Sends HTTP POST to each matching active webhook.
/// Uses HMAC-SHA256 to sign the payload if the webhook has a secret.
/// Runs asynchronously via tokio::spawn — does not block the caller.
#[cfg(feature = "webhooks")]
pub fn fire_webhooks(db: &Database, event: &str, payload: serde_json::Value) {
    let webhooks = match list_webhooks(db) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to list webhooks: {}", e);
            return;
        }
    };

    let active_webhooks: Vec<Webhook> = webhooks
        .into_iter()
        .filter(|w| w.active && w.handles_event(event))
        .collect();

    if active_webhooks.is_empty() {
        return;
    }

    let event = event.to_string();
    let db = db.clone();

    tokio::spawn(async move {
        for webhook in active_webhooks {
            let payload_str = payload.to_string();
            let delivery_id = uuid::Uuid::new_v4().to_string();

            // Compute HMAC signature if secret is set
            let signature = webhook.secret.as_ref().map(|secret| {
                use hmac::{Hmac, Mac};
                use sha2::Sha256;

                type HmacSha256 = Hmac<Sha256>;
                let mut mac =
                    HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key length");
                mac.update(payload_str.as_bytes());
                let result = mac.finalize();
                format!("sha256={}", hex::encode(result.into_bytes()))
            });

            let mut attempts = 0;
            let max_attempts = 3;
            let mut success = false;
            let mut response_status = None;
            let mut response_body = None;

            while attempts < max_attempts && !success {
                attempts += 1;

                let client = reqwest::Client::new();
                let mut request = client
                    .post(&webhook.url)
                    .header("Content-Type", "application/json")
                    .header("X-Pebble-Event", &event)
                    .header("X-Pebble-Delivery", &delivery_id)
                    .header("User-Agent", "Pebble-CMS-Webhook/1.0");

                if let Some(ref sig) = signature {
                    request = request.header("X-Pebble-Signature", sig);
                }

                match request.body(payload_str.clone()).send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16() as i32;
                        response_status = Some(status);
                        response_body = resp.text().await.ok();
                        success = (200..300).contains(&status);
                    }
                    Err(e) => {
                        response_body = Some(e.to_string());
                    }
                }

                if !success && attempts < max_attempts {
                    let delay = std::time::Duration::from_secs(1 << (2 * (attempts - 1)));
                    tokio::time::sleep(delay).await;
                }
            }

            // Log delivery
            if let Ok(conn) = db.get() {
                let _ = conn.execute(
                    "INSERT INTO webhook_deliveries (webhook_id, event, payload, response_status, response_body, success, attempts)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        webhook.id,
                        event,
                        payload_str,
                        response_status,
                        response_body,
                        success,
                        attempts,
                    ],
                );
            }

            if success {
                tracing::info!(
                    "Webhook delivered: {} -> {} ({})",
                    event,
                    webhook.url,
                    response_status.unwrap_or(0)
                );
            } else {
                tracing::warn!(
                    "Webhook failed after {} attempts: {} -> {}",
                    attempts,
                    event,
                    webhook.url
                );
            }
        }
    });
}

/// No-op version when webhooks feature is disabled.
#[cfg(not(feature = "webhooks"))]
pub fn fire_webhooks(_db: &Database, _event: &str, _payload: serde_json::Value) {
    // Webhooks feature not enabled
}
