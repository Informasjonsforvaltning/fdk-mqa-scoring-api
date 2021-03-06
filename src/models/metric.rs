/*
 * Metadata Quality
 *
 * Metadata quality of datasets
 *
 * The version of the OpenAPI document: 0.1.0
 * 
 * Generated by: https://openapi-generator.tech
 */




#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Metric {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "score")]
    pub score: i32,
    #[serde(rename = "is_scored", skip_serializing_if = "Option::is_none")]
    pub is_scored: Option<bool>,
    #[serde(rename = "max_score")]
    pub max_score: i32,
}

impl Metric {
    pub fn new(id: String, score: i32, max_score: i32) -> Metric {
        Metric {
            id,
            score,
            is_scored: None,
            max_score,
        }
    }
}


