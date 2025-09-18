use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::handlers::*;
use crate::middleware::auth_middleware;

pub fn routes(pool: PgPool) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/accounts", post(create_account))
        .route("/api-keys", post(create_api_key));

    // Protected routes (require API key authentication)
    let protected_routes = Router::new()
        .route("/accounts", get(list_accounts))
        .route("/accounts/{id}", get(get_account))
        .route("/accounts/{id}/balance", get(get_account_balance))
        .route("/transactions", post(create_transaction).get(list_transactions))
        .route("/transactions/{id}", get(get_transaction))
        .route("/webhooks", post(create_webhook).get(list_webhooks))
        .route_layer(middleware::from_fn_with_state(pool.clone(), auth_middleware));

    // Combine routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(pool)
}
