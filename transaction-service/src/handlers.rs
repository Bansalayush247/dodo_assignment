use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use sqlx::Row;
use rust_decimal::Decimal;
use hmac::Hmac;
use hmac::Mac;
use sha2::Sha256;
use crate::auth::{compute_fingerprint, hash_key};

use crate::models::*;

// ============================
// Account Handlers
// ============================

pub async fn create_account(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<Json<Account>, (StatusCode, Json<ErrorResponse>)> {
    let initial_balance = payload.initial_balance.unwrap_or_default();

    let account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO accounts (business_name, balance)
        VALUES ($1, $2)
        RETURNING id, business_name, balance, created_at, updated_at
        "#,
    )
    .bind(payload.business_name)
    .bind(initial_balance)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create account: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to create account")),
        )
    })?;

    Ok(Json(account))
}

pub async fn list_accounts(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Account>>, (StatusCode, Json<ErrorResponse>)> {
    let accounts = sqlx::query_as::<_, Account>(
        "SELECT id, business_name, balance, created_at, updated_at FROM accounts ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch accounts: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to fetch accounts")),
        )
    })?;

    Ok(Json(accounts))
}

pub async fn get_account(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Account>, (StatusCode, Json<ErrorResponse>)> {
    let account = sqlx::query_as::<_, Account>(
        "SELECT id, business_name, balance, created_at, updated_at FROM accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch account: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to fetch account")),
        )
    })?
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("not_found", "Account not found")),
    ))?;

    Ok(Json(account))
}

pub async fn get_account_balance(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<AccountBalance>, (StatusCode, Json<ErrorResponse>)> {
    let row = sqlx::query(
        "SELECT balance FROM accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch account balance: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to fetch balance")),
        )
    })?
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("not_found", "Account not found")),
    ))?;

    let balance_val: Decimal = row.get("balance");

    Ok(Json(AccountBalance {
        account_id: id,
        balance: balance_val,
    }))
}

// ============================
// Transaction Handlers
// ============================

pub async fn create_transaction(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<Json<Transaction>, (StatusCode, Json<ErrorResponse>)> {
    // Validate transaction type
    if !["credit", "debit", "transfer"].contains(&payload.txn_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "invalid_transaction_type",
                "Transaction type must be 'credit', 'debit', or 'transfer'",
            )),
        ));
    }

    // Validate business logic
    match payload.txn_type.as_str() {
        "credit" => {
            if payload.from_account_id.is_some() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "invalid_credit",
                        "Credit transactions should not have a from_account_id",
                    )),
                ));
            }
            if payload.to_account_id.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "invalid_credit",
                        "Credit transactions must have a to_account_id",
                    )),
                ));
            }
        }
        "debit" => {
            if payload.from_account_id.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "invalid_debit",
                        "Debit transactions must have a from_account_id",
                    )),
                ));
            }
            if payload.to_account_id.is_some() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "invalid_debit",
                        "Debit transactions should not have a to_account_id",
                    )),
                ));
            }
        }
        "transfer" => {
            if payload.from_account_id.is_none() || payload.to_account_id.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "invalid_transfer",
                        "Transfer transactions must have both from_account_id and to_account_id",
                    )),
                ));
            }
        }
        _ => unreachable!(),
    }

    // Start database transaction for atomic balance updates
    let mut tx = pool.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to begin transaction")),
        )
    })?;

    // Update balances based on transaction type
    match payload.txn_type.as_str() {
        "credit" => {
            // Add to target account
            let to_account_id = payload.to_account_id.unwrap();
            sqlx::query(
                "UPDATE accounts SET balance = balance + $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(payload.amount)
            .bind(to_account_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update account balance: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("database_error", "Failed to update balance")),
                )
            })?;
        }
        "debit" => {
            // Subtract from source account
            let from_account_id = payload.from_account_id.unwrap();
            let result = sqlx::query(
                "UPDATE accounts SET balance = balance - $1, updated_at = NOW() WHERE id = $2 AND balance >= $1",
            )
            .bind(payload.amount)
            .bind(from_account_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update account balance: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("database_error", "Failed to update balance")),
                )
            })?;

            if result.rows_affected() == 0 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("insufficient_funds", "Insufficient funds")),
                ));
            }
        }
        "transfer" => {
            // Subtract from source account
            let from_account_id = payload.from_account_id.unwrap();
            let to_account_id = payload.to_account_id.unwrap();

            let result = sqlx::query(
                "UPDATE accounts SET balance = balance - $1, updated_at = NOW() WHERE id = $2 AND balance >= $1",
            )
            .bind(payload.amount)
            .bind(from_account_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update source account balance: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("database_error", "Failed to update balance")),
                )
            })?;

            if result.rows_affected() == 0 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("insufficient_funds", "Insufficient funds")),
                ));
            }

            // Add to target account
            sqlx::query(
                "UPDATE accounts SET balance = balance + $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(payload.amount)
            .bind(to_account_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update target account balance: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("database_error", "Failed to update balance")),
                )
            })?;
        }
        _ => unreachable!(),
    }

    // Create the transaction record
    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions (from_account, to_account, amount, txn_type, status)
        VALUES ($1, $2, $3, $4, 'completed')
        RETURNING id, from_account, to_account, amount, txn_type, status, created_at
        "#,
    )
    .bind(payload.from_account_id)
    .bind(payload.to_account_id)
    .bind(payload.amount)
    .bind(payload.txn_type)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to create transaction")),
        )
    })?;

    // Commit the transaction
    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to commit transaction")),
        )
    })?;

    // TODO: Trigger webhook delivery
    tokio::spawn(deliver_webhooks(pool.clone(), transaction.clone()));

    Ok(Json(transaction))
}

pub async fn list_transactions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Transaction>>, (StatusCode, Json<ErrorResponse>)> {
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT id, from_account, to_account, amount, txn_type, status, created_at FROM transactions ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch transactions: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to fetch transactions")),
        )
    })?;

    Ok(Json(transactions))
}

pub async fn get_transaction(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Transaction>, (StatusCode, Json<ErrorResponse>)> {
    let transaction = sqlx::query_as::<_, Transaction>(
        "SELECT id, from_account, to_account, amount, txn_type, status, created_at FROM transactions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to fetch transaction")),
        )
    })?
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("not_found", "Transaction not found")),
    ))?;

    Ok(Json(transaction))
}

// ============================
// API Key Handlers
// ============================

pub async fn create_api_key(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Generate a random API key
    use rand::Rng;
    let api_key: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    // Compute fingerprint and Argon2 hash using helpers
    let fingerprint = compute_fingerprint(&api_key);
    let password_hash = hash_key(&api_key).map_err(|e| {
        tracing::error!("Failed to hash API key: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("crypto_error", "Failed to hash API key")),
        )
    })?;

    // Insert fingerprint + hash into DB
    let row = sqlx::query(
        r#"
        INSERT INTO api_keys (account_id, key_fingerprint, key_hash)
        VALUES ($1, $2, $3)
        RETURNING id, account_id, key_fingerprint, key_hash, created_at, last_used
        "#,
    )
    .bind(payload.account_id)
    .bind(&fingerprint)
    .bind(&password_hash)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert API key: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to create API key")),
        )
    })?;

    // Build response containing the raw key once
    #[derive(serde::Serialize)]
    struct ApiKeyCreationResponse {
        id: uuid::Uuid,
        account_id: uuid::Uuid,
        key: String,
        created_at: chrono::DateTime<chrono::Utc>,
        last_used: Option<chrono::DateTime<chrono::Utc>>,
    }

    let created = ApiKeyCreationResponse {
        id: row.get("id"),
        account_id: row.get("account_id"),
        key: api_key,
        created_at: row.get("created_at"),
        last_used: row.get("last_used"),
    };

    Ok(Json(serde_json::to_value(created).unwrap()))
}

// ============================
// Webhook Handlers
// ============================

pub async fn create_webhook(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<Json<Webhook>, (StatusCode, Json<ErrorResponse>)> {
    // Generate a random secret for HMAC signing
    use rand::Rng;
    let secret: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let webhook = sqlx::query_as::<_, Webhook>(
        r#"
        INSERT INTO webhooks (account_id, url, secret)
        VALUES ($1, $2, $3)
        RETURNING id, account_id, url, secret, created_at
        "#,
    )
    .bind(payload.account_id)
    .bind(payload.url)
    .bind(secret)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create webhook: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to create webhook")),
        )
    })?;
    Ok(Json(webhook))
}

pub async fn list_webhooks(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Webhook>>, (StatusCode, Json<ErrorResponse>)> {
    let webhooks = sqlx::query_as::<_, Webhook>(
        "SELECT id, account_id, url, secret, created_at FROM webhooks ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch webhooks: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("database_error", "Failed to fetch webhooks")),
        )
    })?;
    Ok(Json(webhooks))
}

// ============================
// Webhook Delivery
// ============================

async fn deliver_webhooks(pool: PgPool, transaction: Transaction) {
    tracing::info!("Starting webhook delivery for transaction {}", transaction.id);

    // Find all webhooks for accounts involved in the transaction
    let mut account_ids = Vec::new();
    if let Some(from_account) = transaction.from_account {
        account_ids.push(from_account);
    }
    if let Some(to_account) = transaction.to_account {
        account_ids.push(to_account);
    }

    for account_id in account_ids {
        let rows = sqlx::query_as::<_, Webhook>(
            "SELECT id, account_id, url, secret, created_at FROM webhooks WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_all(&pool)
        .await;
        if let Ok(rows) = rows {
            let webhooks: Vec<Webhook> = rows;

            for webhook in webhooks {
                // Create webhook event record
                let event = sqlx::query_as::<_, WebhookEvent>(
                    r#"
                    INSERT INTO webhook_events (webhook_id, txn_id, delivered, retry_count)
                    VALUES ($1, $2, false, 0)
                    RETURNING id, webhook_id, txn_id, delivered, retry_count, last_attempt, created_at
                    "#,
                )
                .bind(webhook.id)
                .bind(transaction.id)
                .fetch_one(&pool)
                .await;

                if let Ok(event) = event {
                    // Attempt delivery
                    tokio::spawn(attempt_webhook_delivery(pool.clone(), webhook, transaction.clone(), event));
                }
            }
        }
    }
}

async fn attempt_webhook_delivery(
    pool: PgPool,
    webhook: Webhook,
    transaction: Transaction,
    mut event: WebhookEvent,
) {
    const MAX_RETRIES: i32 = 3;
    
    let payload = WebhookPayload {
        event_type: "transaction.created".to_string(),
        transaction: transaction.clone(),
        timestamp: chrono::Utc::now(),
    };

    for attempt in 0..=MAX_RETRIES {
        tracing::info!("Attempting webhook delivery {} for event {}", attempt + 1, event.id);

        let client = reqwest::Client::new();
        // Compute HMAC-SHA256 signature over JSON body
        let body = match serde_json::to_vec(&payload) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to serialize webhook payload: {}", e);
                // Mark event as failed and break
                let _ = sqlx::query(
                    "UPDATE webhook_events SET delivered = $1, retry_count = $2, last_attempt = NOW() WHERE id = $3",
                )
                .bind(false)
                .bind(attempt)
                .bind(event.id)
                .execute(&pool)
                .await;
                break;
            }
        };

        // NOTE: webhook.secret is stored as plain text; use it as HMAC key
        let mut mac = Hmac::<Sha256>::new_from_slice(webhook.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(&body);
        let signature = mac.finalize().into_bytes();
        let sig_hex = hex::encode(signature);
        let sig_header = format!("sha256={}", sig_hex);

        let result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Signature", sig_header)
            .body(body.clone())
            .send()
            .await;

        let success = match result {
            Ok(response) => response.status().is_success(),
            Err(e) => {
                tracing::error!("Webhook delivery failed: {}", e);
                false
            }
        };

        // Update webhook event
        event.retry_count = attempt;
        event.delivered = success;

        let _ = sqlx::query(
            "UPDATE webhook_events SET delivered = $1, retry_count = $2, last_attempt = NOW() WHERE id = $3",
        )
        .bind(event.delivered)
        .bind(event.retry_count)
        .bind(event.id)
        .execute(&pool)
        .await;

        if success {
            tracing::info!("Webhook delivered successfully for event {}", event.id);
            break;
        }

        if attempt < MAX_RETRIES {
            // Exponential backoff: 1s, 2s, 4s
            let delay = std::time::Duration::from_secs(2_u64.pow(attempt as u32));
            tokio::time::sleep(delay).await;
        } else {
            tracing::error!("Failed to deliver webhook after {} attempts for event {}", MAX_RETRIES + 1, event.id);
        }
    }
}
