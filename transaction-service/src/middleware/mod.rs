use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use sqlx::PgPool;
use sqlx::Row;
use crate::auth::{compute_fingerprint, verify_key};
use crate::rate_limit;
use std::env;
// argon2 imports not needed here (verification uses helper)

use crate::models::ErrorResponse;

pub async fn auth_middleware(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Extract API key from x-api-key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "unauthorized",
                    "Missing x-api-key header",
                )),
            )
        })?;

    // Validate API key against database
    // Compute fingerprint to find candidate
    let fingerprint = compute_fingerprint(api_key);

    let key_row = sqlx::query(
        "SELECT id, account_id, key_hash FROM api_keys WHERE key_fingerprint = $1",
    )
    .bind(&fingerprint)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "database_error",
                "Failed to validate API key",
            )),
        )
    })?;
    let row = match key_row {
        Some(r) => r,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("unauthorized", "Invalid API key")),
            ));
        }
    };

    let key_hash: String = row.get("key_hash");

    // Verify Argon2 hash with helper
    let verified = verify_key(api_key, &key_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("crypto_error", "Invalid key hash format")),
        )
    })?;

    if !verified {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("unauthorized", "Invalid API key")),
        ));
    }

    // Update last_used timestamp (use id from row)
    let key_id: uuid::Uuid = row.get("id");
    let _ = sqlx::query(
        "UPDATE api_keys SET last_used = NOW() WHERE id = $1",
    )
    .bind(key_id)
    .execute(&pool)
    .await;

    // Rate limiting: enforce per-key limit (fingerprint used as key)
    let limit_per_min: u64 = env::var("RATE_LIMIT_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
    if !rate_limit::allow(&fingerprint, limit_per_min) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new("rate_limited", "Too many requests")),
        ));
    }

    // Continue to the next handler
    Ok(next.run(request).await)
}
