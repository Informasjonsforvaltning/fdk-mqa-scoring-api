use std::env;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref API_KEY: String = env::var("API_KEY").unwrap_or_else(|e| {
        tracing::error!(error = e.to_string().as_str(), "API_KEY not found");
        std::process::exit(1)
    });
    pub static ref ALLOWED_ORIGINS: String = env::var("CORS_ORIGIN_PATTERNS").unwrap_or_else(|e| {
        tracing::error!(
            error = e.to_string().as_str(),
            "CORS_ORIGIN_PATTERNS not found"
        );
        std::process::exit(1)
    });
}
