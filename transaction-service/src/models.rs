use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};
use sqlx::FromRow;
use uuid::Uuid;

// ============================
// Account Models
// ============================

#[derive(Debug, Serialize, FromRow)]
pub struct Account {
    pub id: Uuid,
    pub business_name: String,
    pub balance: rust_decimal::Decimal,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub business_name: String,
    pub initial_balance: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Serialize)]
pub struct AccountBalance {
    pub account_id: Uuid,
    pub balance: rust_decimal::Decimal,
}

// ============================
// Transaction Models
// ============================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub from_account: Option<Uuid>,
    pub to_account: Option<Uuid>,
    pub amount: rust_decimal::Decimal,
    pub txn_type: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount: rust_decimal::Decimal,
    pub txn_type: String, // "credit", "debit", "transfer"
}

// ============================
// API Key Models
// ============================

#[derive(Debug, Serialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub account_id: Uuid,
    pub key_fingerprint: String,
    pub key_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub account_id: Uuid,
}

// ============================
// Webhook Models
// ============================

#[derive(Debug, Serialize, FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub account_id: Uuid,
    pub url: String,
    pub secret: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub account_id: Uuid,
    pub url: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub txn_id: Uuid,
    pub delivered: bool,
    pub retry_count: i32,
    pub last_attempt: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ============================
// Webhook Payload
// ============================

#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub transaction: Transaction,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ============================
// Error Types
// ============================

#[derive(Debug)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

// Custom Serialize implementation to keep backward compatibility with clients
// that expect the old `error` field while producing the new `code` field.
impl Serialize for ErrorResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // We will emit three fields: code, message, and legacy error (duplicate of code)
        let mut s = serializer.serialize_struct("ErrorResponse", 3)?;
        s.serialize_field("code", &self.code)?;
        s.serialize_field("message", &self.message)?;
        // legacy key for compatibility
        s.serialize_field("error", &self.code)?;
        s.end()
    }
}
