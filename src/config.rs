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

pub fn http_workers() -> usize {
    env::var("HTTP_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(4)
        })
}
