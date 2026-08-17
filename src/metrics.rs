use std::{
    future::{ready, Future, Ready},
    pin::Pin,
    sync::Once,
    time::Instant,
};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error as ActixError,
};
use lazy_static::lazy_static;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref HTTP_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "HTTP Requests"),
        &["method", "path", "status"]
    )
    .unwrap_or_else(|e| {
        tracing::error!(error = e.to_string(), "http_requests_total metric error");
        std::process::exit(1);
    });
    static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts {
            common_opts: Opts::new("http_request_duration_seconds", "HTTP Request Duration"),
            buckets: vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 100.0],
        },
        &["method", "path"]
    )
    .unwrap_or_else(|e| {
        tracing::error!(
            error = e.to_string(),
            "http_request_duration_seconds metric error"
        );
        std::process::exit(1);
    });
}

static REGISTER: Once = Once::new();

pub fn register_metrics() {
    REGISTER.call_once(|| {
        REGISTRY
            .register(Box::new(HTTP_REQUESTS.clone()))
            .unwrap_or_else(|e| {
                tracing::error!(error = e.to_string(), "http_requests_total collector error");
                std::process::exit(1);
            });

        REGISTRY
            .register(Box::new(HTTP_REQUEST_DURATION.clone()))
            .unwrap_or_else(|e| {
                tracing::error!(
                    error = e.to_string(),
                    "http_request_duration_seconds collector error"
                );
                std::process::exit(1);
            });
    });
}

pub fn get_metrics() -> Result<String, String> {
    let mut buffer = Vec::new();

    TextEncoder::new()
        .encode(&REGISTRY.gather(), &mut buffer)
        .map_err(|e| e.to_string())?;

    String::from_utf8(buffer).map_err(|e| e.to_string())
}

pub struct HttpMetrics;

impl<S, B> Transform<S, ServiceRequest> for HttpMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = HttpMetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(HttpMetricsMiddleware { service }))
    }
}

pub struct HttpMetricsMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for HttpMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let start = Instant::now();
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let status = res.status().as_u16().to_string();
            let path = res
                .request()
                .match_pattern()
                .unwrap_or_else(|| "unmatched".to_string());

            HTTP_REQUESTS
                .with_label_values(&[&method, &path, &status])
                .inc();
            HTTP_REQUEST_DURATION
                .with_label_values(&[&method, &path])
                .observe(start.elapsed().as_secs_f64());

            Ok(res)
        })
    }
}
