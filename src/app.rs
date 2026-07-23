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
        test,
    };
    use dotenvy::from_filename;
    use serde_json::Value;
    use uuid::Uuid;

    use crate::database::migrate_database;

    async fn test_get_ok(path: &str) {
        match from_filename(".env.test") {
            Ok(_) => println!("Successfully loaded .env.test"),
            Err(err) => println!("Error loading .env.test: {}", err),
        }

        // Run migrations before tests to ensure database schema is ready
        migrate_database().unwrap();

        let app = test::init_service(app()).await;
        let req = test::TestRequest::get()
            .insert_header(ContentType::plaintext())
            .insert_header(("Origin", "http://localhost:8080"))
            .uri(path)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_ping() {
        test_get_ok("/ping").await;
    }

    #[actix_web::test]
    async fn test_ready() {
        test_get_ok("/ready").await;
    }

    #[actix_web::test]
    async fn test_404() {
        match from_filename(".env.test") {
            Ok(_) => println!("Successfully loaded .env.test"),
            Err(err) => println!("Error loading .env.test: {}", err),
        }

        let uuid = Uuid::parse_str("02f09a3f-1624-3b1d-1337-44eff7708208").unwrap();
        let path = format!("/api/assessments/{}", uuid);

        let app = test::init_service(app()).await;

        let req = test::TestRequest::get()
            .insert_header(ContentType::plaintext())
            .insert_header(("Origin", "http://localhost:8080"))
            .uri(&path)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_cors_allowed_origins() {
        match from_filename(".env.test") {
            Ok(_) => println!("Successfully loaded .env.test"),
            Err(err) => println!("Error loading .env.test: {}", err),
        }

        let app = test::init_service(app()).await;

        let origins = [
            "https://example.com",
            "https://api.example.com",
            "http://localhost:8080",
            "http://localhost:8081",
        ];

        for origin in &origins {
            let req = test::TestRequest::get()
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
    async fn test_assessments_not_implemented() {
        match from_filename(".env.test") {
            Ok(_) => println!("Successfully loaded .env.test"),
            Err(err) => println!("Error loading .env.test: {}", err),
        }

        let app = test::init_service(app()).await;

        let req = test::TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("Origin", "http://localhost:8080"))
            .set_json(
                serde_json::from_str::<Value>(
                    r#"{
                    "datasets": [
                        "https://dataset.foo"
                    ]
                }"#,
                )
                .unwrap(),
            )
            .uri("/api/assessments")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[actix_web::test]
    async fn test_cors_disallowed_origins() {
        match from_filename(".env.test") {
            Ok(_) => println!("Successfully loaded .env.test"),
            Err(err) => println!("Error loading .env.test: {}", err),
        }

        let app = test::init_service(app()).await;

        let origins = ["https://exxxample.com"];

        for origin in &origins {
            let req = test::TestRequest::get()
                .insert_header(("Origin", *origin))
                .uri("/ready")
                .to_request();

            let resp = test::call_service(&app, req).await;

            assert_eq!(resp.status(), StatusCode::OK);
            assert!(!resp.headers().contains_key("access-control-allow-origin"),);
        }
    }

    #[actix_web::test]
    async fn test_post_and_get_scores() {
        match from_filename(".env.test") {
            Ok(_) => println!("Successfully loaded .env.test"),
            Err(err) => println!("Error loading .env.test: {}", err),
        }

        let uuid = Uuid::parse_str("02f09a3f-1624-3b1d-8409-44eff7708208").unwrap();
        let path = format!("/api/assessments/{}", uuid);

        let app = test::init_service(app()).await;

        let req = test::TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("Origin", "http://localhost:8080"))
            .set_json(include_str!("../tests/post.json"))
            .uri(&path)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let req = test::TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("X-API-KEY", "bar"))
            .insert_header(("Origin", "http://localhost:8080"))
            .set_json(include_str!("../tests/post.json"))
            .uri(&path)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let req = test::TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("X-API-KEY", "foo"))
            .insert_header(("Origin", "http://localhost:8080"))
            .set_json(serde_json::from_str::<Value>(include_str!("../tests/post.json")).unwrap())
            .uri(&path)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let req = test::TestRequest::get()
            .insert_header(("Origin", "http://localhost:8080"))
            .uri(&path)
            .to_request();
        let bytes = test::call_and_read_body(&app, req).await;
        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            include_str!("../tests/assessment.ttl")
        );

        let req = test::TestRequest::post()
            .insert_header(ContentType::json())
            .insert_header(("Origin", "http://localhost:8080"))
            .set_json(
                serde_json::from_str::<Value>(
                    r#"{
                    "datasets": [
                        "https://dataset.foo"
                    ]
                }"#,
                )
                .unwrap(),
            )
            .uri("/api/scores")
            .to_request();
        let body: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(
            body,
            serde_json::from_str::<Value>(include_str!("../tests/score.json")).unwrap()
        );
    }
}
