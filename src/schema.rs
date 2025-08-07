use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
}

impl Default for Constraint {
    fn default() -> Self {
        Self {
            constraint_type: ConstraintType::None,
        }
    }
}

pub const EXTRACTIONS_KEY: &str = "extractions";

pub trait Schema {
    fn from_examples(examples: &[ExampleData], attribute_suffix: &str) -> Self;
}

#[derive(Debug, Clone)]
pub struct GeminiSchema {
    schema_dict: serde_json::Value,
}

impl GeminiSchema {
    pub fn schema_dict(&self) -> &serde_json::Value {
        &self.schema_dict
    }
}

impl Schema for GeminiSchema {
    fn from_examples(examples: &[ExampleData], attribute_suffix: &str) -> Self {
        let mut extraction_categories: HashMap<String, HashMap<String, HashSet<ValueType>>> = HashMap::new();

        for example in examples {
            for ext in &example.extractions {
                let category = &ext.extraction_class;
                let attrs = extraction_categories.entry(category.clone()).or_default();

                if let Some(attr_map) = &ext.attributes {
                    for (k, v) in attr_map {
                        attrs.entry(k.clone()).or_default().insert(ValueType::from_json(v));
                    }
                }
            }
        }

        let mut extraction_properties = serde_json::Map::new();

        for (category, attrs) in extraction_categories {
            extraction_properties.insert(category.clone(), json!({"type": "string"}));

            let mut attr_props = serde_json::Map::new();

            if attrs.is_empty() {
                attr_props.insert("_unused".to_string(), json!({"type": "string"}));
            } else {
                for (attr_name, attr_types) in attrs {
                    let prop = if attr_types.contains(&ValueType::Array) {
                        json!({"type": "array", "items": {"type": "string"}})
                    } else {
                        json!({"type": "string"})
                    };
                    attr_props.insert(attr_name, prop);
                }
            }

            extraction_properties.insert(
                format!("{}{}", category, attribute_suffix),
                json!({
                    "type": "object",
                    "properties": attr_props,
                    "nullable": true
                }),
            );
        }

        let extraction_schema = json!({
            "type": "object",
            "properties": extraction_properties,
        });

        let schema_dict = json!({
            "type": "object",
            "properties": {
                EXTRACTIONS_KEY: {
                    "type": "array",
                    "items": extraction_schema
                }
            },
            "required": [EXTRACTIONS_KEY]
        });

        Self { schema_dict }
    }
}

// --- Supporting structures ---

use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct Extraction {
    pub extraction_class: String,
    pub attributes: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
pub struct ExampleData {
    pub extractions: Vec<Extraction>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ValueType {
    String,
    Array,
    Other,
}

impl ValueType {
    fn from_json(val: &Value) -> Self {
        match val {
            Value::Array(_) => ValueType::Array,
            Value::String(_) => ValueType::String,
            _ => ValueType::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_gemini_schema_generation() {
        let examples = vec![
            ExampleData {
                extractions: vec![Extraction {
                    extraction_class: "Book".to_string(),
                    attributes: Some(HashMap::from([
                        ("title".to_string(), json!("Rust Book")),
                        ("authors".to_string(), json!(["Alice", "Bob"])),
                    ])),
                }],
            },
            ExampleData {
                extractions: vec![
                    Extraction {
                        extraction_class: "Book".to_string(),
                        attributes: Some(HashMap::from([("title".to_string(), json!("Another Book"))])),
                    },
                    Extraction {
                        extraction_class: "Article".to_string(),
                        attributes: Some(HashMap::from([("name".to_string(), json!("Deep Learning"))])),
                    },
                ],
            },
        ];

        let schema = GeminiSchema::from_examples(&examples, "_attributes");
        let dict = schema.schema_dict();

        assert!(dict.get("properties").is_some());
        assert_eq!(dict["properties"][EXTRACTIONS_KEY]["type"], json!("array"));
    }
}
