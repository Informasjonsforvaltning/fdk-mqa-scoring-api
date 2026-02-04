use std::collections::HashMap;

use diesel::{
    expression_methods::ExpressionMethods,
    r2d2::{ConnectionManager, Pool, PooledConnection},
    result, Connection, PgConnection, QueryDsl, RunQueryDsl,
};
use uuid::Uuid;

use crate::{
    db_models::{DatasetAssessment, Dimension, DimensionAggregate},
    models, schema,
};

/// Embedded database migrations for automatic schema management.
pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!("./migrations");
type DB = diesel::pg::Pg;

/// Runs all pending database migrations on the given connection.
fn run_migration(conn: &mut impl diesel_migrations::MigrationHarness<DB>) {
    conn.run_pending_migrations(MIGRATIONS).unwrap();
}

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("{0}: {1}")]
    ConfigError(&'static str, String),
    #[error(transparent)]
    R2d2Error(#[from] r2d2::Error),
    #[error(transparent)]
    DieselError(#[from] result::Error),
    #[error(transparent)]
    DieselConnectionError(#[from] diesel::ConnectionError),
    #[error(transparent)]
    DieselMigrationError(#[from] diesel_migrations::MigrationError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

/// Retrieves an environment variable, returning a `DatabaseError` if not found.
fn var(key: &'static str) -> Result<String, DatabaseError> {
    std::env::var(key).map_err(|e| DatabaseError::ConfigError(key, e.to_string()))
}

/// Constructs a PostgreSQL connection URL from environment variables.
/// 
/// Required environment variables:
/// - `POSTGRES_HOST`: Database hostname
/// - `POSTGRES_PORT`: Database port number
/// - `POSTGRES_USERNAME`: Database username
/// - `POSTGRES_PASSWORD`: Database password
/// - `POSTGRES_DB_NAME`: Database name
fn database_url() -> Result<String, DatabaseError> {
    let host = var("POSTGRES_HOST")?;
    let port = var("POSTGRES_PORT")?
        .parse::<u16>()
        .map_err(|e| DatabaseError::ConfigError("POSTGRES_PORT", e.to_string()))?;
    let user = var("POSTGRES_USERNAME")?;
    let password = var("POSTGRES_PASSWORD")?;
    let dbname = var("POSTGRES_DB_NAME")?;
    let url = format!("postgres://{user}:{password}@{host}:{port}/{dbname}");

    Ok(url)
}

/// Runs all pending database migrations.
/// 
/// This function establishes a connection to the database and applies any
/// pending migrations defined in the `migrations/` directory.
pub fn migrate_database() -> Result<(), DatabaseError> {
    let url = database_url()?;
    let mut conn = PgConnection::establish(&url)?;
    run_migration(&mut conn);

    Ok(())
}

/// A connection pool for PostgreSQL database connections.
/// 
/// Uses r2d2 for connection pooling with a maximum of 2 connections.
/// Connections are tested on checkout to ensure they're still valid.
#[derive(Clone)]
pub struct PgPool(Pool<ConnectionManager<PgConnection>>);

impl PgPool {
    /// Creates a new connection pool.
    /// 
    /// The pool is configured with:
    /// - Maximum size: 2 connections
    /// - Test on checkout: enabled (validates connections before use)
    pub fn new() -> Result<Self, DatabaseError> {
        let url = database_url()?;
        let manager = ConnectionManager::new(url);
        let pool = Pool::builder()
            .max_size(2)
            .test_on_check_out(true)
            .build(manager)
            .expect("Could not create a connection pool");
        Ok(PgPool(pool))
    }

    /// Gets a connection from the pool.
    /// 
    /// Returns an error if no connections are available or if the pool is closed.
    pub fn get(&self) -> Result<PgConn, DatabaseError> {
        Ok(PgConn(self.0.get()?))
    }
}

/// A pooled PostgreSQL database connection.
/// 
/// This connection is automatically returned to the pool when dropped.
pub struct PgConn(PooledConnection<ConnectionManager<PgConnection>>);

impl PgConn {
    /// Tests the database connection by performing a lightweight query.
    /// 
    /// This is used by the `/ready` endpoint to verify database connectivity.
    /// Uses a simple `SELECT 1` query which is the most efficient way to test
    /// a connection without touching any tables.
    pub fn test_connection(&mut self) -> Result<(), DatabaseError> {
        use diesel::sql_query;
        use diesel::sql_types::Integer;
        use diesel::deserialize::QueryableByName;
        
        #[derive(QueryableByName)]
        struct TestResult {
            #[diesel(sql_type = Integer)]
            #[diesel(column_name = "one")]
            _value: i32,
        }
        
        // Use a simple SELECT 1 query with an alias - this is the lightest possible query
        // and doesn't require any table access or counting
        let _: TestResult = sql_query("SELECT 1 AS one").get_result(&mut self.0)?;
        Ok(())
    }

    /// Stores or updates a dataset assessment in the database.
    /// 
    /// If an assessment with the same ID already exists, it will be updated.
    /// Otherwise, a new assessment will be inserted.
    pub fn store_dataset_assessment(&mut self, assessment: DatasetAssessment) -> Result<(), DatabaseError> {
        use schema::dataset_assessments::dsl;

        diesel::insert_into(dsl::dataset_assessments)
            .values(&assessment)
            .on_conflict(dsl::id)
            .do_update()
            .set(&assessment)
            .execute(&mut self.0)?;

        Ok(())
    }

    /// Stores or updates a dimension score for a dataset.
    /// 
    /// If a dimension with the same dataset URI and ID already exists, it will be updated.
    /// Otherwise, a new dimension will be inserted.
    pub fn store_dimension(&mut self, dimension: Dimension) -> Result<(), DatabaseError> {
        use schema::dimensions::dsl;

        diesel::insert_into(dsl::dimensions)
            .values(&dimension)
            .on_conflict((dsl::dataset_uri, dsl::id))
            .do_update()
            .set(&dimension)
            .execute(&mut self.0)?;

        Ok(())
    }

    /// Deletes all dimension records for a given dataset URI.
    /// 
    /// This is typically called when updating a dataset assessment to remove
    /// old dimension data before inserting new values.
    pub fn drop_dataset_dimensions(&mut self, dataset_uri: &str) -> Result<(), DatabaseError> {
        use schema::dimensions::dsl;

        diesel::delete(dsl::dimensions)
            .filter(dsl::dataset_uri.eq(dataset_uri))
            .execute(&mut self.0)?;

        Ok(())
    }

    /// Retrieves the Turtle (RDF) format assessment for a given dataset assessment ID.
    /// 
    /// Returns `None` if the assessment doesn't exist, or `Some(turtle_string)` if found.
    pub fn turtle_assessment(
        &mut self,
        dataset_assessment: Uuid,
    ) -> Result<Option<String>, DatabaseError> {
        use schema::dataset_assessments::dsl;

        match dsl::dataset_assessments
            .filter(dsl::id.eq(dataset_assessment.to_string()))
            .select(dsl::turtle_assessment)
            .first(&mut self.0)
        {
            Ok(assessment) => Ok(Some(assessment)),
            Err(result::Error::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Retrieves the JSON-LD format assessment for a given dataset assessment ID.
    /// 
    /// Returns `None` if the assessment doesn't exist, or `Some(jsonld_string)` if found.
    pub fn jsonld_assessment(
        &mut self,
        dataset_assessment: Uuid,
    ) -> Result<Option<String>, DatabaseError> {
        use schema::dataset_assessments::dsl;

        match dsl::dataset_assessments
            .filter(dsl::id.eq(dataset_assessment.to_string()))
            .select(dsl::jsonld_assessment)
            .first(&mut self.0)
        {
            Ok(assessment) => Ok(Some(assessment)),
            Err(result::Error::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Retrieves JSON score data for multiple datasets.
    /// 
    /// # Arguments
    /// 
    /// * `dataset_uris` - A vector of dataset URIs to fetch scores for
    /// 
    /// # Returns
    /// 
    /// A HashMap mapping dataset URIs to their `DatasetScore` objects.
    /// 
    /// # Note
    /// 
    /// Ensure that URIs are valid before calling this function, as they are
    /// used directly in the SQL query.
    pub fn json_scores(
        &mut self,
        dataset_uris: &Vec<String>,
    ) -> Result<HashMap<String, models::DatasetScore>, DatabaseError> {
        use schema::dataset_assessments::dsl;

        let uris = dataset_uris
            .iter()
            .map(|uri| uri.to_string())
            .collect::<Vec<String>>();

        let rows: Vec<(String, String)> = dsl::dataset_assessments
            .filter(dsl::dataset_uri.eq_any(uris))
            .select((dsl::dataset_uri, dsl::json_score))
            .get_results(&mut self.0)?;

        let dataset_scores = rows
            .into_iter()
            .map(|(dataset_uri, json)| {
                if json.is_empty() {
                    tracing::error!(
                        dataset_uri = dataset_uri.as_str(),
                        json_length = json.len(),
                        "Empty JSON string found in database for dataset"
                    );
                    // Create an EOF error by attempting to parse empty string
                    let e = serde_json::from_str::<models::DatasetScore>("")
                        .map_err(|e| {
                            tracing::error!(
                                dataset_uri = dataset_uri.as_str(),
                                error = format!("{:?}", e).as_str(),
                                "Empty JSON string in database caused EOF error"
                            );
                            DatabaseError::SerdeError(e)
                        })
                        .unwrap_err();
                    return Err(e);
                }
                serde_json::from_str(&json).map_err(|e| {
                    tracing::error!(
                        dataset_uri = dataset_uri.as_str(),
                        json_length = json.len(),
                        json_preview = if json.len() > 500 { 
                            &json[..500] 
                        } else { 
                            &json 
                        },
                        error = format!("{:?}", e).as_str(),
                        "Failed to parse JSON from database in json_scores"
                    );
                    DatabaseError::SerdeError(e)
                })
                .map(|score| (dataset_uri, score))
            })
            .collect::<Result<HashMap<String, models::DatasetScore>, DatabaseError>>()?;

        Ok(dataset_scores)
    }

    /// Calculates average dimension scores across multiple datasets.
    /// 
    /// This function aggregates dimension scores (accessibility, findability, etc.)
    /// by computing the average score and max_score for each dimension type
    /// across all specified datasets.
    /// 
    /// # Arguments
    /// 
    /// * `dataset_uris` - A vector of dataset URIs to aggregate dimensions for
    /// 
    /// # Returns
    /// 
    /// A vector of `DimensionAggregate` objects, sorted in a predefined order:
    /// 1. Interoperability
    /// 2. Findability
    /// 3. Accessibility
    /// 4. Contextuality
    /// 5. Reusability
    /// 
    /// # Note
    /// 
    /// Ensure that URIs are valid before calling this function, as they are
    /// used directly in the SQL query. The results are sorted to ensure
    /// consistent ordering in API responses.
    pub fn dimension_aggregates(
        &mut self,
        dataset_uris: &Vec<String>,
    ) -> Result<Vec<models::DimensionAggregate>, DatabaseError> {
        let q = format!(
            "SELECT id, AVG(score)::float8 AS score, AVG(max_score)::float8 AS max_score
             FROM dimensions WHERE dataset_uri in ({}) GROUP BY id ORDER BY id",
            dataset_uris
                .iter()
                .map(|uri| format!("'{uri}'"))
                .collect::<Vec<String>>()
                .join(",")
        );
        let aggregates: Vec<DimensionAggregate> =
            diesel::dsl::sql_query(q).get_results(&mut self.0)?;

        // Define the expected order for consistent API responses
        let order = [
            "https://data.norge.no/vocabulary/dcatno-mqa#interoperability",
            "https://data.norge.no/vocabulary/dcatno-mqa#findability",
            "https://data.norge.no/vocabulary/dcatno-mqa#accessibility",
            "https://data.norge.no/vocabulary/dcatno-mqa#contextuality",
            "https://data.norge.no/vocabulary/dcatno-mqa#reusability",
        ];

        let mut result: Vec<models::DimensionAggregate> = aggregates
            .into_iter()
            .map(
                |DimensionAggregate {
                     id,
                     score,
                     max_score,
                 }| models::DimensionAggregate {
                    id,
                    score,
                    max_score,
                },
            )
            .collect();

        // Sort by the predefined order to ensure consistent API responses
        result.sort_by(|a, b| {
            let a_pos = order.iter().position(|&x| x == a.id).unwrap_or(usize::MAX);
            let b_pos = order.iter().position(|&x| x == b.id).unwrap_or(usize::MAX);
            a_pos.cmp(&b_pos)
        });

        Ok(result)
    }
}
