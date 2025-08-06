use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::data::{ AnnotatedDocument, AttributeValue, CharInterval, Extraction};
use crate::tokenizer::TokenInterval;

pub fn annotated_document_to_dict(adoc: &AnnotatedDocument) -> Value {
    if adoc.text.is_none() && adoc.extractions.is_none() {
        return Value::Object(Map::new());
    }

    let mut map = Map::new();

    // text
    if let Some(ref text) = adoc.text {
        map.insert("text".to_string(), Value::String(text.clone()));
    }

    // document_id
    map.insert(
        "document_id".to_string(),
        Value::String(adoc.clone().document_id()),
    );

    // extractions
    if let Some(ref extractions) = adoc.extractions {
        let mut ext_array = Vec::new();
        for ext in extractions {
            let mut ext_map = Map::new();
            ext_map.insert(
                "extraction_class".to_string(),
                Value::String(ext.extraction_class.clone()),
            );
            ext_map.insert(
                "extraction_text".to_string(),
                Value::String(ext.extraction_text.clone()),
            );

            // alignment_status
            if let Some(status) = &ext.alignment_status {
                ext_map.insert(
                    "alignment_status".to_string(),
                    Value::String(status.to_string()),
                );
            }

            // char_interval
            if let Some(ref char_interval) = ext.char_interval {
                let mut ci = Map::new();
                if let Some(start) = char_interval.start_pos {
                    ci.insert("start_pos".to_string(), Value::Number(start.into()));
                }
                if let Some(end) = char_interval.end_pos {
                    ci.insert("end_pos".to_string(), Value::Number(end.into()));
                }
                ext_map.insert("char_interval".to_string(), Value::Object(ci));
            }

            // token_interval
            if let Some(ref token_interval) = ext.token_interval() {
                let mut ti = Map::new();
                ti.insert(
                    "start".to_string(),
                    Value::Number(token_interval.start_index.into()),
                );
                ti.insert(
                    "end".to_string(),
                    Value::Number(token_interval.end_index.into()),
                );
                ext_map.insert("token_interval".to_string(), Value::Object(ti));
            }

            // attributes
            if let Some(ref attrs) = ext.attributes {
                let mut attr_map = Map::new();
                for (k, v) in attrs {
                    match v {
                        AttributeValue::Single(s) => {
                            attr_map.insert(k.clone(), Value::String(s.clone()));
                        }
                        AttributeValue::Multiple(list) => {
                            attr_map.insert(
                                k.clone(),
                                Value::Array(
                                    list.iter().map(|s| Value::String(s.clone())).collect(),
                                ),
                            );
                        }
                    }
                }
                ext_map.insert("attributes".to_string(), Value::Object(attr_map));
            }

            ext_array.push(Value::Object(ext_map));
        }
        map.insert("extractions".to_string(), Value::Array(ext_array));
    }

    Value::Object(map)
}

pub fn dict_to_annotated_document(value: &Value) -> AnnotatedDocument {
    if !value.is_object() {
        return AnnotatedDocument::new(None, None, None);
    }

    let map = value.as_object().unwrap();

    let document_id = map
        .get("document_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let text = map
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let extractions = map
        .get("extractions")
        .and_then(|v| v.as_array())
        .map(|extractions| {
            extractions
                .iter()
                .filter_map(|ext_val| {
                    let ext_obj = ext_val.as_object()?;

                    let extraction_class = ext_obj
                        .get("extraction_class")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let extraction_text = ext_obj
                        .get("extraction_text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // alignment_status
                    let alignment_status = ext_obj
                        .get("alignment_status")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.try_into().ok());

                    // char_interval
                    let char_interval = ext_obj.get("char_interval").and_then(|ci| {
                        let start = ci
                            .get("start_pos")
                            .and_then(|v| v.as_u64())
                            .map(|x| x as usize);
                        let end = ci
                            .get("end_pos")
                            .and_then(|v| v.as_u64())
                            .map(|x| x as usize);
                        Some(CharInterval::new(start, end))
                    });

                    // token_interval
                    let token_interval = ext_obj.get("token_interval").and_then(|ti| {
                        let start = ti.get("start").and_then(|v| v.as_u64()).map(|x| x as usize);
                        let end = ti.get("end").and_then(|v| v.as_u64()).map(|x| x as usize);
                        Some(TokenInterval {
                            start_index: start.unwrap(),
                            end_index: end.unwrap(),
                        })
                    });

                    // attributes
                    let attributes = ext_obj.get("attributes").and_then(|attrs| {
                        let mut map = HashMap::new();
                        if let Some(obj) = attrs.as_object() {
                            for (k, v) in obj {
                                if v.is_string() {
                                    map.insert(
                                        k.clone(),
                                        AttributeValue::Single(v.as_str().unwrap().to_string()),
                                    );
                                } else if v.is_array() {
                                    let arr = v.as_array().unwrap();
                                    let vec_str: Vec<String> = arr
                                        .iter()
                                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                        .collect();
                                    map.insert(k.clone(), AttributeValue::Multiple(vec_str));
                                }
                            }
                        }
                        Some(map)
                    });

                    Some(Extraction::new(
                        extraction_class,
                        extraction_text,
                        token_interval,
                        char_interval,
                        alignment_status,
                        None,
                        None,
                        None,
                        attributes,
                    ))
                })
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());

    AnnotatedDocument::new(document_id, extractions, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        AlignmentStatus, AnnotatedDocument, AttributeValue, CharInterval, Extraction,
    };
    use crate::tokenizer::TokenInterval;
    use std::collections::HashMap;

    #[test]
    fn test_annotated_document_to_dict_and_back() {
        let char_interval = CharInterval::new(Some(0), Some(5));
        let token_interval = TokenInterval {
            start_index: 0,
            end_index: 2,
        };
        let mut attributes = HashMap::new();
        attributes.insert(
            "attr1".to_string(),
            AttributeValue::Single("value1".to_string()),
        );

        let extraction = Extraction::new(
            "class1".to_string(),
            "text1".to_string(),
            Some(token_interval.clone()),
            Some(char_interval.clone()),
            Some(AlignmentStatus::MatchExact),
            None,
            None,
            None,
            Some(attributes.clone()),
        );

        let adoc = AnnotatedDocument::new(
            Some("doc_1234".to_string()),
            Some(vec![extraction]),
            Some("hello".to_string()),
        );

        let dict = annotated_document_to_dict(&adoc);
        assert!(
            dict.get("document_id")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("doc_")
        );

        let adoc_back = dict_to_annotated_document(&dict);

        assert_eq!(adoc_back.text.unwrap(), "hello".to_string());
        assert_eq!(adoc_back.extractions.unwrap().len(), 1);
    }

    #[test]
    fn test_empty_annotated_document() {
        let adoc = AnnotatedDocument::new(None, None, None);
        let dict = annotated_document_to_dict(&adoc);
        assert!(dict.as_object().unwrap().is_empty());

        dbg!(&adoc);
        dbg!(&dict);
        let adoc_back = dict_to_annotated_document(&dict);
        dbg!(&adoc_back);
        assert!(adoc_back.text.is_none());
        assert!(adoc_back.extractions.is_none());
    }
}
