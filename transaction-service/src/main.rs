use axum::{Router, routing::get, response::Html, response::IntoResponse};
// Serve static files from the `static/` directory if present; fall back to embedded bytes
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
    <link rel="stylesheet" type="text/css" href="/static/swagger-ui.css" />
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>body { margin:0; padding:0; }</style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="/static/swagger-ui-bundle.js"></script>
    <script>
        // Initialize Swagger UI
        if (typeof SwaggerUIBundle === 'undefined') {
            // fallback: try loading CDN bundle
            var s = document.createElement('script');
            s.src = 'https://unpkg.com/swagger-ui-dist@4/swagger-ui-bundle.js';
            s.onload = function() { initSwagger(); };
            document.head.appendChild(s);
        } else {
            initSwagger();
        }

        function initSwagger() {
            try {
                SwaggerUIBundle({
                    url: '/openapi.yaml',
                    dom_id: '#swagger-ui',
                    presets: [
                        SwaggerUIBundle.presets.apis,
                        SwaggerUIBundle.presets.standalone
                    ]
                });
            } catch (e) {
                document.getElementById('swagger-ui').innerText = 'Failed to initialize Swagger UI: ' + e;
            }
        }
    </script>
</body>
</html>
"#;
    Html(html)
}

async fn serve_swagger_css() -> impl IntoResponse {
    // Prefer files from ./static/ if present (copied into the runtime image); otherwise use embedded bytes
    match fs::read("./static/swagger-ui.css").await {
        Ok(bytes) => (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/css")],
            bytes,
        ),
        Err(_) => {
            if let Some(bytes) = embedded_assets::swagger_css() {
                (
                    StatusCode::OK,
                    [(CONTENT_TYPE, "text/css")],
                    bytes.to_vec(),
                )
            } else {
                tracing::warn!("No embedded swagger CSS available and ./static/swagger-ui.css not found");
                (
                    StatusCode::NOT_FOUND,
                    [(CONTENT_TYPE, "text/plain")],
                    "swagger css not found".to_string().into_bytes(),
                )
            }
        }
    }
}

async fn serve_swagger_bundle_js() -> impl IntoResponse {
    // Prefer files from ./static/ if present (copied into the runtime image); otherwise use embedded bytes
    match fs::read("./static/swagger-ui-bundle.js").await {
        Ok(bytes) => (
            StatusCode::OK,
            [(CONTENT_TYPE, "application/javascript")],
            bytes,
        ),
        Err(_) => {
            if let Some(bytes) = embedded_assets::swagger_js() {
                (
                    StatusCode::OK,
                    [(CONTENT_TYPE, "application/javascript")],
                    bytes.to_vec(),
                )
            } else {
                tracing::warn!("No embedded swagger JS available and ./static/swagger-ui-bundle.js not found");
                (
                    StatusCode::NOT_FOUND,
                    [(CONTENT_TYPE, "text/plain")],
                    "swagger js not found".to_string().into_bytes(),
                )
            }
        }
    }
}
