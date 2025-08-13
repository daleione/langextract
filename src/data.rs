use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

use crate::tokenizer::{TokenInterval, TokenizedText, tokenize};

#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentStatus {
    MatchExact,
    MatchGreater,
    MatchLesser,
    MatchFuzzy,
}

impl fmt::Display for AlignmentStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AlignmentStatus::MatchExact => write!(f, "match_exact"),
            AlignmentStatus::MatchGreater => write!(f, "match_greater"),
            AlignmentStatus::MatchLesser => write!(f, "match_lesser"),
            AlignmentStatus::MatchFuzzy => write!(f, "match_fuzzy"),
        }
    }
}

impl TryFrom<&str> for AlignmentStatus {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "match_exact" => Ok(AlignmentStatus::MatchExact),
            "match_greater" => Ok(AlignmentStatus::MatchGreater),
            "match_lesser" => Ok(AlignmentStatus::MatchLesser),
            "match_fuzzy" => Ok(AlignmentStatus::MatchFuzzy),
            _ => Err(format!("Unknown alignment status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CharInterval {
    pub start_pos: Option<usize>,
    pub end_pos: Option<usize>,
}

impl CharInterval {
    pub fn new(start_pos: Option<usize>, end_pos: Option<usize>) -> Self {
        Self { start_pos, end_pos }
    }
}

#[derive(Debug, Clone)]
pub struct Extraction {
    pub extraction_class: String,
    pub extraction_text: String,
    pub char_interval: Option<CharInterval>,
    pub alignment_status: Option<AlignmentStatus>,
    pub extraction_index: Option<usize>,
    pub group_index: Option<usize>,
    pub description: Option<String>,
    pub attributes: Option<HashMap<String, AttributeValue>>,
    token_interval: Option<TokenInterval>,
}

#[derive(Debug, Clone)]
pub enum AttributeValue {
    Single(String),
    Multiple(Vec<String>),
}

impl Extraction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extraction_class: String,
        extraction_text: String,
        token_interval: Option<TokenInterval>,
        char_interval: Option<CharInterval>,
        alignment_status: Option<AlignmentStatus>,
        extraction_index: Option<usize>,
        group_index: Option<usize>,
        description: Option<String>,
        attributes: Option<HashMap<String, AttributeValue>>,
    ) -> Self {
        Self {
            extraction_class,
            extraction_text,
            char_interval,
            token_interval,
            alignment_status,
            extraction_index,
            group_index,
            description,
            attributes,
        }
    }

    pub fn token_interval(&self) -> Option<&TokenInterval> {
        self.token_interval.as_ref()
    }

    pub fn set_token_interval(&mut self, value: Option<TokenInterval>) {
        self.token_interval = value;
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub text: String,
    pub additional_context: Option<String>,
    document_id: Option<String>,
    tokenized_text: Option<TokenizedText>,
}

impl Document {
    pub fn new(text: String, document_id: Option<String>, additional_context: Option<String>) -> Self {
        Self {
            text,
            additional_context,
            document_id,
            tokenized_text: None,
        }
    }

    pub fn document_id(&mut self) -> String {
        if self.document_id.is_none() {
            self.document_id = Some(format!("doc_{}", &Uuid::new_v4().simple().to_string()[..8]));
        }
        self.document_id.clone().unwrap()
    }

    pub fn set_document_id(&mut self, value: Option<String>) {
        self.document_id = value;
    }

    pub fn tokenized_text(&mut self) -> &TokenizedText {
        if self.tokenized_text.is_none() {
            self.tokenized_text = Some(tokenize(&self.text));
        }
        self.tokenized_text.as_ref().unwrap()
    }

    pub fn set_tokenized_text(&mut self, value: TokenizedText) {
        self.tokenized_text = Some(value);
    }
}

/// AnnotatedDocument 结构体
#[derive(Debug, Clone)]
pub struct AnnotatedDocument {
    pub extractions: Option<Vec<Extraction>>,
    pub text: Option<String>,
    document_id: Option<String>,
    tokenized_text: Option<TokenizedText>,
}

impl AnnotatedDocument {
    pub fn new(document_id: Option<String>, extractions: Option<Vec<Extraction>>, text: Option<String>) -> Self {
        Self {
            extractions,
            text,
            document_id,
            tokenized_text: None,
        }
    }

    pub fn document_id(&mut self) -> String {
        if self.document_id.is_none() {
            self.document_id = Some(format!("doc_{}", &Uuid::new_v4().simple().to_string()[..8]));
        }
        self.document_id.clone().unwrap()
    }

    pub fn set_document_id(&mut self, value: Option<String>) {
        self.document_id = value;
    }

    pub fn tokenized_text(&mut self) -> Option<&TokenizedText> {
        if self.tokenized_text.is_none()
            && let Some(ref text) = self.text
        {
            self.tokenized_text = Some(tokenize(text));
        }
        self.tokenized_text.as_ref()
    }

    pub fn set_tokenized_text(&mut self, value: TokenizedText) {
        self.tokenized_text = Some(value);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatType {
    Yaml,
    Json,
}

#[derive(Debug, Clone)]
pub struct ExampleData {
    pub text: String,
    pub extractions: Vec<Extraction>,
}

impl ExampleData {
    pub fn new(text: String, extractions: Vec<Extraction>) -> Self {
        Self { text, extractions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_char_interval_creation() {
        let interval = CharInterval::new(Some(0), Some(5));
        assert_eq!(interval.start_pos, Some(0));
        assert_eq!(interval.end_pos, Some(5));
    }

    #[test]
    fn test_document_id_generation() {
        let mut doc = Document::new("Hello World".to_string(), None, None);
        let id1 = doc.document_id();
        assert!(id1.starts_with("doc_"));
        let id2 = doc.document_id();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_document_id_setter() {
        let mut doc = Document::new("Hello".to_string(), None, None);
        doc.set_document_id(Some("custom_id".to_string()));
        assert_eq!(doc.document_id(), "custom_id".to_string());
    }

    #[test]
    fn test_tokenized_text_lazy_init() {
        let mut doc = Document::new("Hello World".to_string(), None, None);
        assert!(doc.tokenized_text.is_none());

        let tokens = doc.tokenized_text().tokens.clone();
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].index, 0);
        assert_eq!(tokens[0].token_type, crate::tokenizer::TokenType::Word);
        assert_eq!(tokens[0].char_interval.start_pos, 0);
        assert_eq!(tokens[0].char_interval.end_pos, 5);
    }

    #[test]
    fn test_annotated_document_generation() {
        let mut ann_doc = AnnotatedDocument::new(None, None, Some("Test Text".to_string()));
        let id = ann_doc.document_id();
        assert!(id.starts_with("doc_"));
    }

    #[test]
    fn test_extraction_with_attributes() {
        let mut attributes = HashMap::new();
        attributes.insert("key1".to_string(), AttributeValue::Single("value1".to_string()));
        attributes.insert(
            "key2".to_string(),
            AttributeValue::Multiple(vec!["v1".to_string(), "v2".to_string()]),
        );

        let extraction = Extraction::new(
            "class1".to_string(),
            "text1".to_string(),
            None,
            None,
            Some(AlignmentStatus::MatchExact),
            Some(1),
            Some(2),
            Some("description".to_string()),
            Some(attributes),
        );

        assert_eq!(extraction.extraction_class, "class1");
        assert_eq!(extraction.extraction_text, "text1");
        assert_eq!(extraction.alignment_status, Some(AlignmentStatus::MatchExact));
    }

    #[test]
    fn test_example_data_creation() {
        let extraction = Extraction::new(
            "class".to_string(),
            "text".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let example = ExampleData::new("Example text".to_string(), vec![extraction]);
        assert_eq!(example.text, "Example text");
        assert_eq!(example.extractions.len(), 1);
    }

    #[test]
    fn test_alignment_status_conversion() {
        let status_str = AlignmentStatus::MatchExact.to_string();
        assert_eq!(&status_str, "match_exact");

        let status = AlignmentStatus::try_from("match_fuzzy").unwrap();
        assert_eq!(status, AlignmentStatus::MatchFuzzy);
    }
}
