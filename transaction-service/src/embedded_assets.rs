// Minimal Swagger UI assets embedded in binary
pub const SWAGGER_CSS: &[u8] = b"
.swagger-ui { font-family: sans-serif; }
.swagger-ui .topbar { display: none; }
";

pub const SWAGGER_JS: &[u8] = b"
// Minimal Swagger UI implementation
window.SwaggerUIBundle = {
    presets: {
        apis: [],
        standalone: []
    }
};

// Simple fallback if CDN fails
if (!window.SwaggerUIBundle) {
    document.body.innerHTML = '<h1>Swagger UI</h1><p>Loading from CDN...</p><script src=\"https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-bundle.js\"></script>';
}
";