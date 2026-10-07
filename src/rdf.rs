use oxrdfio::{JsonLdProfileSet, RdfFormat, RdfParser, RdfSerializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RdfError {
    #[error("invalid Turtle assessment: {0}")]
    Parse(String),
    #[error("failed to serialize assessment as JSON-LD: {0}")]
    Serialize(String),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Converts a Turtle RDF graph to JSON-LD for storage alongside the Turtle form.
pub fn turtle_to_jsonld(turtle: &str) -> Result<String, RdfError> {
    let mut serializer = RdfSerializer::from_format(RdfFormat::JsonLd {
        profile: JsonLdProfileSet::empty(),
    })
    .for_writer(Vec::new());

    for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(turtle.as_bytes()) {
        let quad = quad.map_err(|e| RdfError::Parse(e.to_string()))?;
        serializer
            .serialize_quad(&quad)
            .map_err(|e| RdfError::Serialize(e.to_string()))?;
    }

    let bytes = serializer
        .finish()
        .map_err(|e| RdfError::Serialize(e.to_string()))?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_turtle_to_jsonld() {
        let turtle = r#"<https://dataset.assessment.foo> <https://data.norge.no/vocabulary/dcatno-mqa#assessmentOf> <https://dataset.foo> .
_:b1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/dqv#QualityMeasurement> .
"#;
        let jsonld = turtle_to_jsonld(turtle).unwrap();
        assert!(jsonld.contains("https://dataset.assessment.foo"));
        assert!(jsonld.contains("https://dataset.foo"));
        serde_json::from_str::<serde_json::Value>(&jsonld).unwrap();
    }

    #[test]
    fn rejects_invalid_turtle() {
        let err = turtle_to_jsonld("not valid turtle").unwrap_err();
        assert!(matches!(err, RdfError::Parse(_)));
    }
}
