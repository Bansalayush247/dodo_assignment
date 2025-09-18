// src/main.rs - clean implementation
use axum::{Router, routing::get, response::Html, response::IntoResponse};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use tokio::fs;
use std::net::SocketAddr;
use tracing_subscriber;
use dotenvy::dotenv;
use std::env;

mod routes;
mod db;
mod middleware;
mod models;
mod handlers;
mod auth;
mod embedded_assets;
mod rate_limit;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter("transaction_service=debug,tower_http=debug")
        .init();

    let pool = db::init_pool().await.expect("DB connection failed");

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/openapi.yaml", get(serve_openapi))
        .route("/docs", get(serve_swagger))
        .route("/static/swagger-ui.css", get(serve_swagger_css))
        .route("/static/swagger-ui-bundle.js", get(serve_swagger_bundle_js))
        .nest("/api", routes::routes(pool.clone()));

    let port: u16 = env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server at {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_openapi() -> impl IntoResponse {
    match fs::read("openapi.yaml").await {
        Ok(bytes) => (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/yaml")],
            bytes,
        ),
        Err(e) => {
            tracing::error!("Failed to read openapi.yaml: {}", e);
            (
                StatusCode::NOT_FOUND,
                [(CONTENT_TYPE, "text/plain")],
                "openapi.yaml not found".to_string().into_bytes(),
            )
        }
    }
}

async fn serve_swagger() -> impl IntoResponse {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>API Documentation</title>
    <link rel="stylesheet" type="text/css" href="/swagger-ui.css" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="/swagger-ui-bundle.js"></script>
    <script>
        SwaggerUIBundle({
            url: '/openapi.yaml',
            dom_id: '#swagger-ui',
            presets: [
                SwaggerUIBundle.presets.apis,
                SwaggerUIBundle.presets.standalone
            ]
        });
    </script>
</body>
</html>
"#;
    Html(html)
}

async fn serve_swagger_css() -> impl IntoResponse {
    let bytes: &'static [u8] = embedded_assets::SWAGGER_CSS;
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/css")],
        bytes.to_vec(),
    )
}

async fn serve_swagger_bundle_js() -> impl IntoResponse {
    let bytes: &'static [u8] = embedded_assets::SWAGGER_JS;
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/javascript")],
        bytes.to_vec(),
    )
}
