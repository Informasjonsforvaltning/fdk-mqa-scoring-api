use actix_cors::Cors;

use crate::config::ALLOWED_ORIGINS;

pub fn cors() -> Cors {
    let origins: Vec<String> = ALLOWED_ORIGINS
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    Cors::default()
        .allowed_origin_fn(move |origin, _req_head| {
            let origin_str = origin.to_str().unwrap_or_default();

            for allowed in &origins {
                if allowed == origin_str {
                    return true;
                }

                if allowed.starts_with("*.")
                    && origin_str.ends_with(allowed.trim_start_matches('*'))
                {
                    return true;
                }

                if allowed.ends_with(":*") && origin_str.starts_with(allowed.trim_end_matches(":*"))
                {
                    return true;
                }
            }

            false
        })
        .allowed_methods(["GET", "POST"])
        .allow_any_header()
        .max_age(3600)
}
