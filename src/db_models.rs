use super::schema::*;
use diesel::sql_types::Double;

#[derive(Insertable, Queryable, AsChangeset)]
#[diesel(table_name = dataset_assessments)]
pub struct DatasetAssessment {
    pub id: String,
    pub dataset_uri: String,
    pub turtle_assessment: String,
    pub jsonld_assessment: String,
    pub json_score: String,
}

#[derive(Insertable, Queryable, AsChangeset)]
#[diesel(table_name = dimensions)]
pub struct Dimension {
    pub dataset_uri: String,
    pub id: String,
    pub score: i32,
    pub max_score: i32,
}

#[derive(QueryableByName)]
#[diesel(table_name = dimensions)]
pub struct DimensionAggregate {
    pub id: String,
    #[diesel(sql_type = Double)]
    pub score: f64,
    #[diesel(sql_type = Double)]
    pub max_score: f64,
}
