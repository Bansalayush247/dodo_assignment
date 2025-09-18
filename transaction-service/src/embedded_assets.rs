// Embedded assets removed: static files are served from the `static/` directory inside the
// runtime image. These helper functions remain to keep the module API present for code
// that references `embedded_assets::swagger_css()` and `embedded_assets::swagger_js()`.

/// Return embedded swagger CSS bytes if present. Currently returns `None` to indicate
/// no embedded fallback is available.
pub fn swagger_css() -> Option<&'static [u8]> {
    None
}

/// Return embedded swagger JS bytes if present. Currently returns `None` to indicate
/// no embedded fallback is available.
pub fn swagger_js() -> Option<&'static [u8]> {
    None
}