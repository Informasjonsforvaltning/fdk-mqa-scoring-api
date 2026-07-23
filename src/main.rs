#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate serde;

use actix_web::{middleware::Logger, HttpServer};

use crate::{app::app, config::API_KEY, database::migrate_database};

mod app;
mod auth;
mod config;
mod cors;
mod database;
mod db_models;
mod error;
mod handlers;
mod http_utils;
mod models;
mod mqa_dimensions;
mod schema;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .with_current_span(false)
        .init();

    tracing::debug!("Tracing initialized");

    migrate_database().unwrap();

    // Fail if API_KEY missing
    let _ = API_KEY.clone();

    HttpServer::new(move || app().wrap(Logger::default()))
        .bind(("0.0.0.0", 8082))?
        .run()
        .await
}
