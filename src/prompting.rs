/// Library for building prompts.
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Minimal representation of FormatType (mirrors langextract.data.FormatType)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    YAML,
    JSON,
}


impl TryFrom<&str> for FormatType {
    type Error = String;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "yaml" => Ok(FormatType::YAML),
            "json" => Ok(FormatType::JSON),
            _ => Err(format!("Invalid format type: {}", value)),
        }
    }
}

/// Custom errors for prompt builder
#[derive(thiserror::Error, Debug)]
pub enum PromptBuilderError {
    #[error("I/O error reading file: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, PromptBuilderError>;

/// ExampleData and related types (a minimal mirror of langextract.data.ExampleData)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Extraction {
    pub extraction_class: String,
    pub extraction_text: String,
    #[serde(default)]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExampleData {
    pub text: String,
    #[serde(default)]
    pub extractions: Vec<Extraction>,
}

/// Structured prompt template for few-shot examples.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptTemplateStructured {
    pub description: String,
    #[serde(default)]
    pub examples: Vec<ExampleData>,
}

/// Read a structured prompt template from a file (YAML or JSON).
///
/// Returns PromptTemplateStructured or ParseError.
pub fn read_prompt_template_structured_from_file<P: AsRef<Path>>(
    prompt_path: P,
    format_type: FormatType,
) -> Result<PromptTemplateStructured> {
    let content = fs::read_to_string(&prompt_path)?;
    match format_type {
        FormatType::YAML => {
            let tpl: PromptTemplateStructured = serde_yaml::from_str(&content)?;
            Ok(tpl)
        }
        FormatType::JSON => {
            let tpl: PromptTemplateStructured = serde_json::from_str(&content)?;
            Ok(tpl)
        }
    }
}

/// QAPromptGenerator: generates question-answer prompts from the provided template.
#[derive(Debug, Clone)]
pub struct QAPromptGenerator {
    pub template: PromptTemplateStructured,
    pub format_type: FormatType,
    pub attribute_suffix: String,
    pub examples_heading: String,
    pub question_prefix: String,
    pub answer_prefix: String,
    pub fence_output: bool,
}

impl Default for QAPromptGenerator {
    fn default() -> Self {
        QAPromptGenerator {
            template: PromptTemplateStructured {
                description: String::new(),
                examples: Vec::new(),
            },
            format_type: FormatType::YAML,
            attribute_suffix: "_attributes".to_string(),
            examples_heading: "Examples".to_string(),
            question_prefix: "Q: ".to_string(),
            answer_prefix: "A: ".to_string(),
            fence_output: true,
        }
    }
}

impl fmt::Display for QAPromptGenerator {
    /// Returns a string representation of the prompt with an empty question.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(""))
    }
}

impl QAPromptGenerator {
    /// Create a new QAPromptGenerator from a template.
    pub fn new(template: PromptTemplateStructured) -> Self {
        QAPromptGenerator {
            template,
            ..Default::default()
        }
    }

    /// Formats a single example as text according to the generator settings.
    ///
    /// This mirrors the Python `format_example_as_text`.
    pub fn format_example_as_text(&self, example: &ExampleData) -> String {
        let question = &example.text;

        // Build a dictionary (serde_json::Value) for serialization
        let mut extractions_vec = Vec::with_capacity(example.extractions.len());
        for extraction in &example.extractions {
            // each entry is a map with class->text and class+suffix -> attributes
            let mut entry = serde_json::Map::new();
            entry.insert(
                extraction.extraction_class.clone(),
                serde_json::Value::String(extraction.extraction_text.clone()),
            );

            let attrs_key = format!("{}{}", extraction.extraction_class, self.attribute_suffix);
            let attrs_value = extraction
                .attributes
                .clone()
                .map(|m| serde_json::Value::Object(
                    m.into_iter().collect()
                ))
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
            entry.insert(attrs_key, attrs_value);
            extractions_vec.push(serde_json::Value::Object(entry));
        }

        let mut data_obj = serde_json::Map::new();
        // using "extractions" as the key (mirrors schema.EXTRACTIONS_KEY)
        data_obj.insert(
            "extractions".to_string(),
            serde_json::Value::Array(extractions_vec),
        );
        let data_value = serde_json::Value::Object(data_obj);

        // Format as YAML or JSON
        let answer = match self.format_type {
            FormatType::YAML => {
                let formatted = serde_yaml::to_string(&data_value)
                    .unwrap_or_else(|_| String::from("{}"));
                if self.fence_output {
                    format!("```yaml\n{}```", formatted.trim())
                } else {
                    formatted.trim().to_string()
                }
            }
            FormatType::JSON => {
                let formatted = serde_json::to_string_pretty(&data_value)
                    .unwrap_or_else(|_| String::from("{}"));
                if self.fence_output {
                    format!("```json\n{}```", formatted.trim())
                } else {
                    formatted.trim().to_string()
                }
            }
        };

        format!("{}{}\n{}{}\n", self.question_prefix, question, self.answer_prefix, answer)
    }

    /// Render a full prompt with question and optional additional context.
    pub fn render(&self, question: &str) -> String {
        self.render_with_context(question, None)
    }

    /// Render with optional additional context.
    pub fn render_with_context(&self, question: &str, additional_context: Option<&str>) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("{}\n", self.template.description));

        if let Some(ctx) = additional_context
            && !ctx.is_empty() {
                lines.push(format!("{}\n", ctx));
            }

        if !self.template.examples.is_empty() {
            lines.push(self.examples_heading.clone());
            for ex in &self.template.examples {
                lines.push(self.format_example_as_text(ex));
            }
        }

        lines.push(format!("{}{}", self.question_prefix, question));
        lines.push(self.answer_prefix.clone());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_yaml_template() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("tpl.yaml");
        let mut f = File::create(&file_path).unwrap();

        let yaml = r#"
description: "Test description"
examples:
  - text: "Who is Alice?"
    extractions:
      - extraction_class: "person"
        extraction_text: "Alice"
"#;
        write!(f, "{}", yaml).unwrap();
        drop(f);

        let tpl = read_prompt_template_structured_from_file(&file_path, FormatType::YAML).unwrap();
        assert_eq!(tpl.description, "Test description");
        assert_eq!(tpl.examples.len(), 1);
        assert_eq!(tpl.examples[0].text, "Who is Alice?");
        assert_eq!(tpl.examples[0].extractions[0].extraction_class, "person");
    }

    #[test]
    fn test_format_example_as_text_yaml() {
        let ex = ExampleData {
            text: "Find Bob".to_string(),
            extractions: vec![
                Extraction {
                    extraction_class: "name".to_string(),
                    extraction_text: "Bob".to_string(),
                    attributes: None,
                }
            ],
        };

        let tpl = PromptTemplateStructured {
            description: "Desc".to_string(),
            examples: vec![ex.clone()],
        };

        let qa_gen = QAPromptGenerator {
            template: tpl,
            format_type: FormatType::YAML,
            ..Default::default()
        };

        let formatted = qa_gen.format_example_as_text(&ex);
        // Should contain fenced yaml and question/answer prefixes
        assert!(formatted.contains("```yaml"));
        assert!(formatted.contains("Q: Find Bob"));
        assert!(formatted.contains("A: "));
        assert!(formatted.contains("name:"));
    }

    #[test]
    fn test_render_composes_prompt() {
        let ex = ExampleData {
            text: "Who is Alice?".to_string(),
            extractions: vec![Extraction {
                extraction_class: "person".to_string(),
                extraction_text: "Alice".to_string(),
                attributes: None,
            }],
        };

        let tpl = PromptTemplateStructured {
            description: "This is a desc".to_string(),
            examples: vec![ex.clone()],
        };

        let qa_gen = QAPromptGenerator {
            template: tpl,
            format_type: FormatType::JSON,
            ..Default::default()
        };

        let out = qa_gen.render_with_context("What is being asked?", Some("Some extra context"));
        assert!(out.contains("This is a desc"));
        assert!(out.contains("Some extra context"));
        assert!(out.contains("Q: What is being asked?"));
        assert!(out.contains("A:"));
    }
}
