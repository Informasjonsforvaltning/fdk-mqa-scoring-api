use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, App,
};
use utoipa::openapi::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cors::cors,
    database::PgPool,
    handlers::{
        assessments::{assessment_graph, assessments, update_assessment},
        health::{ping, ready},
        scores::scores,
    },
};

pub fn app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = actix_web::Error,
        Config = (),
        InitError = (),
    >,
> {
    let pool = PgPool::new().unwrap();

    let openapi = serde_yaml::from_str::<OpenApi>(include_str!("../openapi.yaml")).unwrap();

    App::new()
        .wrap(cors())
        .app_data(web::PayloadConfig::default().limit(50_000_000)) // 50 MB limit
        .app_data(web::Data::new(pool.clone()))
        .service(ping)
        .service(ready)
        .service(assessment_graph)
        .service(update_assessment)
        .service(assessments)
        .service(scores)
        .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/openapi.json", openapi.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        http::{header::ContentType, header::HeaderValue, StatusCode},
        test::{self, TestRequest},
    };
    use dotenvy::from_filename;
    use serde_json::Value;
    use uuid::Uuid;

    use crate::database::migrate_database;

    const ASSESSMENT_ID: &str = "02f09a3f-1624-3b1d-8409-44eff7708208";
    const MISSING_ASSESSMENT_ID: &str = "02f09a3f-1624-3b1d-1337-44eff7708208";
    const CONFLICT_ASSESSMENT_ID: &str = "02f09a3f-1624-3b1d-9999-44eff7708208";
    const ORIGIN: &str = "http://localhost:8080";
    const VALID_API_KEY: &str = "foo";

    fn load_test_env() {
        if let Err(err) = from_filename(".env.test") {
            println!("Error loading .env.test: {err}");
        }
    }

    fn setup() {
        load_test_env();
        migrate_database().unwrap();
    }

    fn assessment_path(id: &str) -> String {
        format!("/api/assessments/{id}")
    }

    fn post_fixture() -> Value {
        serde_json::from_str(include_str!("../tests/post.json")).unwrap()
    }

    fn score_fixture() -> Value {
        serde_json::from_str(include_str!("../tests/score.json")).unwrap()
    }

    fn datasets_request(uri: &str) -> Value {
        serde_json::json!({ "datasets": [uri] })
    }

    fn store_assessment_request(id: &str) -> TestRequest {
        TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("X-API-KEY", VALID_API_KEY))
            .insert_header(("Origin", ORIGIN))
            .set_json(post_fixture())
            .uri(&assessment_path(id))
    }

    #[actix_web::test]
    async fn test_ping() {
        setup();
        let app = test::init_service(app()).await;
        let req = TestRequest::get()
            .insert_header(ContentType::plaintext())
            .insert_header(("Origin", ORIGIN))
            .uri("/ping")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_ready() {
        setup();
        let app = test::init_service(app()).await;
        let req = TestRequest::get()
            .insert_header(ContentType::plaintext())
            .insert_header(("Origin", ORIGIN))
            .uri("/ready")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_404() {
        setup();
        let app = test::init_service(app()).await;

        let req = TestRequest::get()
            .insert_header(ContentType::plaintext())
            .insert_header(("Origin", ORIGIN))
            .uri(&assessment_path(MISSING_ASSESSMENT_ID))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_cors_allowed_origins() {
        setup();
        let app = test::init_service(app()).await;

        let origins = [
            "https://example.com",
            "https://api.example.com",
            "http://localhost:8080",
            "http://localhost:8081",
        ];

        for origin in &origins {
            let req = TestRequest::get()
                .insert_header(("Origin", *origin))
                .uri("/ready")
                .to_request();

            let resp = test::call_service(&app, req).await;

            assert_eq!(resp.status(), StatusCode::OK);
            assert!(resp.headers().contains_key("access-control-allow-origin"));

            let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
            assert_eq!(cors_header, HeaderValue::from_str(origin).unwrap());
        }
    }

    #[actix_web::test]
    async fn test_cors_disallowed_origins() {
        setup();
        let app = test::init_service(app()).await;

        let req = TestRequest::get()
            .insert_header(("Origin", "https://exxxample.com"))
            .uri("/ready")
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!resp.headers().contains_key("access-control-allow-origin"));
    }

    #[actix_web::test]
    async fn test_assessments_not_implemented() {
        setup();
        let app = test::init_service(app()).await;

        let req = TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("Origin", ORIGIN))
            .set_json(datasets_request("https://dataset.foo"))
            .uri("/api/assessments")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[actix_web::test]
    async fn rejects_missing_api_key() {
        setup();
        let app = test::init_service(app()).await;

        let req = TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("Origin", ORIGIN))
            .set_json(post_fixture())
            .uri(&assessment_path(ASSESSMENT_ID))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn rejects_wrong_api_key() {
        setup();
        let app = test::init_service(app()).await;

        let req = TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("X-API-KEY", "bar"))
            .insert_header(("Origin", ORIGIN))
            .set_json(post_fixture())
            .uri(&assessment_path(ASSESSMENT_ID))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn stores_assessment_and_returns_turtle() {
        setup();
        let app = test::init_service(app()).await;

        let resp = test::call_service(&app, store_assessment_request(ASSESSMENT_ID).to_request()).await;
        assert!(resp.status().is_success());

        let req = TestRequest::get()
            .insert_header(("Origin", ORIGIN))
            .uri(&assessment_path(ASSESSMENT_ID))
            .to_request();
        let bytes = test::call_and_read_body(&app, req).await;
        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            include_str!("../tests/assessment.ttl")
        );
    }

    #[actix_web::test]
    async fn stores_assessment_and_returns_json_ld() {
        setup();
        let app = test::init_service(app()).await;

        let resp = test::call_service(&app, store_assessment_request(ASSESSMENT_ID).to_request()).await;
        assert!(resp.status().is_success());

        let post = post_fixture();
        let expected = post["jsonld_assessment"].as_str().unwrap();

        let req = TestRequest::get()
            .insert_header(("Origin", ORIGIN))
            .insert_header(("Accept", "application/ld+json"))
            .uri(&assessment_path(ASSESSMENT_ID))
            .to_request();
        let bytes = test::call_and_read_body(&app, req).await;
        assert_eq!(String::from_utf8(bytes.to_vec()).unwrap(), expected);
    }

    #[actix_web::test]
    async fn returns_scores_for_dataset_uri() {
        setup();
        let app = test::init_service(app()).await;

        let resp = test::call_service(&app, store_assessment_request(ASSESSMENT_ID).to_request()).await;
        assert!(resp.status().is_success());

        let req = TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("Origin", ORIGIN))
            .set_json(datasets_request("https://dataset.foo"))
            .uri("/api/scores")
            .to_request();
        let body: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body, score_fixture());
    }

    #[actix_web::test]
    async fn rejects_duplicate_dataset_uri_with_different_id() {
        setup();
        let app = test::init_service(app()).await;

        // Keep fixture IDs valid so typos fail loudly.
        Uuid::parse_str(ASSESSMENT_ID).unwrap();
        Uuid::parse_str(CONFLICT_ASSESSMENT_ID).unwrap();

        let resp = test::call_service(&app, store_assessment_request(ASSESSMENT_ID).to_request()).await;
        assert!(resp.status().is_success());

        let resp = test::call_service(
            &app,
            store_assessment_request(CONFLICT_ASSESSMENT_ID).to_request(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }
}
