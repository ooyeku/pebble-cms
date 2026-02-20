use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Webhook {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Webhook {
    /// Returns the list of events this webhook subscribes to.
    pub fn event_list(&self) -> Vec<&str> {
        self.events.split(',').map(|s| s.trim()).collect()
    }

    /// Check whether this webhook is subscribed to a given event.
    pub fn handles_event(&self, event: &str) -> bool {
        self.event_list().iter().any(|e| *e == event)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookDelivery {
    pub id: i64,
    pub webhook_id: i64,
    pub event: String,
    pub payload: String,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub success: bool,
    pub attempts: i32,
    pub delivered_at: String,
}
