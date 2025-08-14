// Library for resolving LLM output
//
// Notes:
// - This is a single-file self-contained version including minimal `data` and
//   `tokenizer` submodules to be runnable out-of-the-box.
// - WordAligner implements exact-token-subsequence matching and a sliding-window
//   fuzzy overlap heuristic (ratio of matched normalized tokens).
// - Replace tokenizer/tokenization with your production tokenizer for better results.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use thiserror::Error;

/// -----------------------
/// Minimal supporting types
/// -----------------------
pub mod schema {
    pub const EXTRACTIONS_KEY: &str = "extractions";
}

pub mod exceptions {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error("{0}")]
    pub struct LangExtractError(pub String);
}

pub mod data {
    use serde_json::Value as JsonValue;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CharInterval {
        pub start_pos: usize,
        pub end_pos: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TokenInterval {
        pub start_index: usize,
        pub end_index: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum AlignmentStatus {
        MatchExact,
        MatchLesser,
        MatchFuzzy,
    }

    #[derive(Debug, Clone)]
    pub struct Extraction {
        pub extraction_class: String,
        pub extraction_text: String,
        pub extraction_index: usize,
        pub group_index: usize,
        pub attributes: Option<JsonValue>,
        pub token_interval: Option<TokenInterval>,
        pub char_interval: Option<CharInterval>,
        pub alignment_status: Option<AlignmentStatus>,
    }

    impl Extraction {
        pub fn new(
            extraction_class: String,
            extraction_text: String,
            extraction_index: usize,
            group_index: usize,
            attributes: Option<JsonValue>,
        ) -> Self {
            Self {
                extraction_class,
                extraction_text,
                extraction_index,
                group_index,
                attributes,
                token_interval: None,
                char_interval: None,
                alignment_status: None,
            }
        }
    }
}

/// Very small whitespace tokenizer that also returns character spans for tokens.
/// Replace this with your real tokenizer for production use.
pub mod tokenizer {
    use super::data::CharInterval;

    #[derive(Debug, Clone)]
    pub struct Token {
        pub text: String,
        pub char_interval: CharInterval,
    }

    #[derive(Debug, Clone)]
    pub struct TokenizedText {
        pub text: String,
        pub tokens: Vec<Token>,
    }

    /// Naive whitespace tokenizer that yields tokens and their char spans.
    pub fn tokenize(text: &str) -> TokenizedText {
        let mut tokens = Vec::new();
        let mut chars = text.char_indices().peekable();

        while let Some((start_idx, ch)) = chars.next() {
            if ch.is_whitespace() {
                continue;
            }

            let mut end_idx = start_idx + ch.len_utf8();
            let mut token_text = String::new();
            token_text.push(ch);

            // Collect non-whitespace characters
            while let Some(&(next_idx, next_ch)) = chars.peek() {
                if next_ch.is_whitespace() {
                    break;
                }
                token_text.push(next_ch);
                end_idx = next_idx + next_ch.len_utf8();
                chars.next();
            }

            tokens.push(Token {
                text: token_text,
                char_interval: CharInterval {
                    start_pos: start_idx,
                    end_pos: end_idx,
                },
            });
        }

        TokenizedText {
            text: text.to_string(),
            tokens,
        }
    }
}

/// ----------------------------
/// Resolver implementation
/// ----------------------------
const FUZZY_ALIGNMENT_MIN_THRESHOLD: f64 = 0.75;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Other: {0}")]
    Other(String),
}

pub type ResolverResult<T> = Result<T, ResolverError>;

/// AbstractResolver trait (mirrors abstract base class behavior).
pub trait AbstractResolver {
    fn resolve(&self, input_text: &str, suppress_parse_errors: bool) -> ResolverResult<Vec<data::Extraction>>;

    fn align(
        &self,
        extractions: &[data::Extraction],
        source_text: &str,
        token_offset: usize,
        char_offset: Option<usize>,
        enable_fuzzy_alignment: bool,
        fuzzy_alignment_threshold: f64,
        accept_match_lesser: bool,
    ) -> Vec<data::Extraction>;
}

/// Concrete Resolver
#[derive(Debug, Clone)]
pub struct Resolver {
    pub fence_output: bool,
    pub extraction_index_suffix: Option<String>,
    pub extraction_attributes_suffix: Option<String>,
    pub format_is_yaml: bool,
}

impl Default for Resolver {
    fn default() -> Self {
        Self {
            fence_output: true,
            extraction_index_suffix: Some("_index".to_string()),
            extraction_attributes_suffix: Some("_attributes".to_string()),
            format_is_yaml: false,
        }
    }
}

impl Resolver {
    pub fn new(
        fence_output: bool,
        extraction_index_suffix: Option<String>,
        extraction_attributes_suffix: Option<String>,
        format_is_yaml: bool,
    ) -> Self {
        Self {
            fence_output,
            extraction_index_suffix,
            extraction_attributes_suffix,
            format_is_yaml,
        }
    }

    /// Extract fenced content if fence_output==true, else full string.
    fn extract_and_parse_content(&self, input_string: &str) -> ResolverResult<JsonValue> {
        if input_string.trim().is_empty() {
            return Err(ResolverError::Parse(
                "Input string must be a non-empty string.".to_string(),
            ));
        }

        let content = if self.fence_output {
            self.extract_fenced_content(input_string)?
        } else {
            input_string.to_string()
        };

        // parse
        if self.format_is_yaml {
            let parsed: JsonValue = serde_yaml::from_str(&content)?;
            Ok(parsed)
        } else {
            let parsed: JsonValue = serde_json::from_str(&content)?;
            Ok(parsed)
        }
    }

    fn extract_fenced_content(&self, input_string: &str) -> ResolverResult<String> {
        let left_key = if self.format_is_yaml { "```yaml" } else { "```json" };

        if let Some(start) = input_string.find(left_key)
            && let Some(end) = input_string[start + left_key.len()..].find("```")
        {
            let content_start = start + left_key.len();
            let content_end = content_start + end;
            return Ok(input_string[content_start..content_end].trim().to_string());
        }

        Err(ResolverError::Parse(
            "Input string does not contain valid markers.".to_string(),
        ))
    }

    /// string_to_extraction_data: ensure mapping with "extractions": [...]
    fn string_to_extraction_data(&self, input_string: &str) -> ResolverResult<Vec<HashMap<String, JsonValue>>> {
        let parsed = self.extract_and_parse_content(input_string)?;

        // Handle simple array format
        if let Some(array) = parsed.as_array() {
            // Simple array format: ["item1", "item2", ...]
            // Convert to single group with multiple extractions using "text" as key
            let mut single_group = HashMap::new();
            for (index, item) in array.iter().enumerate() {
                let key = if array.len() == 1 {
                    "text".to_string()
                } else {
                    format!("text_{}", index)
                };

                if let Some(text) = item.as_str() {
                    single_group.insert(key, JsonValue::String(text.to_string()));
                } else {
                    single_group.insert(key, item.clone());
                }
            }
            return Ok(vec![single_group]);
        }

        if let Some(obj) = parsed.as_object() {
            // Check for structured format first: {"extractions": [...]}
            if let Some(extractions) = obj.get(schema::EXTRACTIONS_KEY) {
                let arr = extractions.as_array().ok_or_else(|| {
                    ResolverError::Parse("The 'extractions' value must be a sequence (list).".to_string())
                })?;

                // Check if this is DeepSeek format: [{"characters": "text", "characters_attributes": {}}, ...]
                if let Some(first_item) = arr.first() {
                    if let Some(first_obj) = first_item.as_object() {
                        let mut has_category_fields = false;
                        for key in first_obj.keys() {
                            if !key.ends_with("_attributes") && key != "extraction_class" && key != "extraction_text" {
                                has_category_fields = true;
                                break;
                            }
                        }

                        if has_category_fields {
                            // Process DeepSeek format
                            let mut result = Vec::new();
                            for item in arr {
                                if let Some(item_obj) = item.as_object() {
                                    for (key, value) in item_obj {
                                        // Skip index and attributes keys
                                        let should_skip = key.ends_with("_attributes")
                                            || (self.extraction_index_suffix.is_some()
                                                && key.ends_with(self.extraction_index_suffix.as_ref().unwrap()));

                                        if !should_skip {
                                            let mut extraction_map = HashMap::new();
                                            extraction_map
                                                .insert("extraction_class".to_string(), JsonValue::String(key.clone()));
                                            extraction_map.insert("extraction_text".to_string(), value.clone());

                                            // Copy over related index and attributes fields
                                            if let Some(index_suffix) = &self.extraction_index_suffix {
                                                let index_key = format!("{}{}", key, index_suffix);
                                                if let Some(index_value) = item_obj.get(&index_key) {
                                                    extraction_map.insert(
                                                        format!("extraction_text{}", index_suffix),
                                                        index_value.clone(),
                                                    );
                                                }
                                            }

                                            if let Some(attr_suffix) = &self.extraction_attributes_suffix {
                                                let attr_key = format!("{}{}", key, attr_suffix);
                                                if let Some(attr_value) = item_obj.get(&attr_key) {
                                                    extraction_map.insert(
                                                        format!("extraction_text{}", attr_suffix),
                                                        attr_value.clone(),
                                                    );
                                                }
                                            }

                                            result.push(extraction_map);
                                        }
                                    }
                                }
                            }
                            return Ok(result);
                        }
                    }
                }

                let mut result = Vec::with_capacity(arr.len());
                for item in arr {
                    if let Some(map) = item.as_object() {
                        // Item is already a mapping
                        let mut hm = HashMap::with_capacity(map.len());
                        for (k, v) in map {
                            hm.insert(k.clone(), v.clone());
                        }
                        result.push(hm);
                    } else if let Some(text) = item.as_str() {
                        // Item is a simple string, convert to extraction format
                        let mut hm = HashMap::new();
                        hm.insert("text".to_string(), JsonValue::String(text.to_string()));
                        result.push(hm);
                    } else {
                        // Item is some other type, convert to string
                        let mut hm = HashMap::new();
                        hm.insert("text".to_string(), item.clone());
                        result.push(hm);
                    }
                }
                return Ok(result);
            }

            // Handle nested category format: {"characters": ["name1", "name2"], "locations": [...]}
            let mut result = Vec::new();
            for (category, value) in obj {
                if let Some(array) = value.as_array() {
                    // Category contains an array of items - create separate extraction for each
                    for item in array.iter() {
                        let mut extraction_map = HashMap::new();
                        extraction_map.insert("extraction_class".to_string(), JsonValue::String(category.clone()));

                        if let Some(text) = item.as_str() {
                            extraction_map.insert("extraction_text".to_string(), JsonValue::String(text.to_string()));
                        } else {
                            extraction_map.insert("extraction_text".to_string(), item.clone());
                        }

                        result.push(extraction_map);
                    }
                } else {
                    // Category contains a single item
                    let mut extraction_map = HashMap::new();
                    extraction_map.insert("extraction_class".to_string(), JsonValue::String(category.clone()));
                    extraction_map.insert("extraction_text".to_string(), value.clone());
                    result.push(extraction_map);
                }
            }
            return Ok(result);
        }

        Err(ResolverError::Parse(
            "Content must be an array, a mapping with an 'extractions' key, or a category-based mapping.".to_string(),
        ))
    }

    /// Extracts and orders extractions similar to Python code logic.
    fn extract_ordered_extractions_impl(
        &self,
        extraction_data: &[HashMap<String, JsonValue>],
    ) -> ResolverResult<Vec<data::Extraction>> {
        let mut processed = Vec::new();
        let mut default_index_counter = 0usize;
        let index_suffix = self.extraction_index_suffix.as_deref();
        let attributes_suffix = self.extraction_attributes_suffix.as_deref();

        for (group_index, group) in extraction_data.iter().enumerate() {
            // Validate index values first
            if let Some(suf) = index_suffix {
                for (key, value) in group {
                    if key.ends_with(suf) && !value.is_number() {
                        return Err(ResolverError::Other("Index values must be integers.".to_string()));
                    }
                }
            }

            // Check if this group represents a structured extraction with extraction_class and extraction_text
            if group.contains_key("extraction_class") && group.contains_key("extraction_text") {
                // Handle structured extraction format
                let extraction_class = group
                    .get("extraction_class")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ResolverError::Parse("extraction_class must be a string".to_string()))?;

                let extraction_text = self.convert_to_text(
                    group
                        .get("extraction_text")
                        .ok_or_else(|| ResolverError::Parse("extraction_text is required".to_string()))?,
                )?;

                let extraction_index =
                    self.get_extraction_index(group, "extraction_text", index_suffix, &mut default_index_counter)?;
                let attributes = self.get_attributes(group, "extraction_text", attributes_suffix)?;

                let ext = data::Extraction::new(
                    extraction_class.to_string(),
                    extraction_text,
                    extraction_index,
                    group_index,
                    attributes,
                );
                processed.push(ext);
            } else {
                // Handle legacy format - treat each key as extraction_class and value as extraction_text
                for (key, value) in group {
                    // Skip index and attributes keys
                    if let Some(suf) = index_suffix
                        && key.ends_with(suf)
                    {
                        continue;
                    }
                    if let Some(suf) = attributes_suffix
                        && key.ends_with(suf)
                    {
                        continue;
                    }

                    let text_val = self.convert_to_text(value)?;
                    let extraction_index =
                        self.get_extraction_index(group, key, index_suffix, &mut default_index_counter)?;
                    let attributes = self.get_attributes(group, key, attributes_suffix)?;

                    let ext = data::Extraction::new(key.clone(), text_val, extraction_index, group_index, attributes);
                    processed.push(ext);
                }
            }
        }

        // Sort by extraction_index then by group_index
        processed.sort_by_key(|e| (e.extraction_index, e.group_index));
        Ok(processed)
    }

    fn convert_to_text(&self, value: &JsonValue) -> ResolverResult<String> {
        match value {
            JsonValue::String(s) => Ok(s.clone()),
            JsonValue::Number(n) => Ok(n.to_string()),
            JsonValue::Bool(b) => Ok(b.to_string()),
            JsonValue::Null => Ok(String::new()),
            JsonValue::Array(_) | JsonValue::Object(_) => Err(ResolverError::Other(
                "Extraction text must be string or number.".to_string(),
            )),
        }
    }

    fn get_extraction_index(
        &self,
        group: &HashMap<String, JsonValue>,
        key: &str,
        index_suffix: Option<&str>,
        default_counter: &mut usize,
    ) -> ResolverResult<usize> {
        if let Some(suf) = index_suffix {
            let index_key = format!("{}{}", key, suf);
            if let Some(idx_val) = group.get(&index_key) {
                return idx_val
                    .as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| ResolverError::Other("Index must be integer.".to_string()));
            }
        }

        *default_counter += 1;
        Ok(*default_counter)
    }

    fn get_attributes(
        &self,
        group: &HashMap<String, JsonValue>,
        key: &str,
        attributes_suffix: Option<&str>,
    ) -> ResolverResult<Option<JsonValue>> {
        if let Some(suf) = attributes_suffix {
            let attr_key = format!("{}{}", key, suf);
            if let Some(v) = group.get(&attr_key) {
                if v.is_object() || v.is_null() {
                    return Ok(Some(v.clone()));
                } else {
                    return Err(ResolverError::Other(
                        "Attributes must be a mapping or null.".to_string(),
                    ));
                }
            }
        }
        Ok(None)
    }

    /// Public entry: parse string -> ordered extractions
    pub fn parse_extractions_from_string(&self, input: &str) -> ResolverResult<Vec<data::Extraction>> {
        let parsed = self.string_to_extraction_data(input)?;
        let processed = self.extract_ordered_extractions_impl(&parsed)?;
        Ok(processed)
    }
}

impl AbstractResolver for Resolver {
    fn resolve(&self, input_text: &str, suppress_parse_errors: bool) -> ResolverResult<Vec<data::Extraction>> {
        match self.string_to_extraction_data(input_text) {
            Ok(parsed) => self.extract_ordered_extractions_impl(&parsed),
            Err(e) => {
                if suppress_parse_errors {
                    Ok(Vec::new())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn align(
        &self,
        extractions: &[data::Extraction],
        source_text: &str,
        token_offset: usize,
        char_offset: Option<usize>,
        enable_fuzzy_alignment: bool,
        fuzzy_alignment_threshold: f64,
        accept_match_lesser: bool,
    ) -> Vec<data::Extraction> {
        if extractions.is_empty() {
            return Vec::new();
        }

        let groups = vec![extractions.to_vec()];
        let mut aligner = WordAligner::new();
        let char_offset_val = char_offset.unwrap_or(0);

        let aligned = aligner.align_extractions(
            &groups,
            source_text,
            token_offset,
            char_offset_val,
            enable_fuzzy_alignment,
            fuzzy_alignment_threshold,
            accept_match_lesser,
        );

        aligned.into_iter().flatten().collect()
    }
}

/// ----------------------------
/// WordAligner (exact + fuzzy)
/// ----------------------------
pub struct WordAligner;

impl WordAligner {
    pub fn new() -> Self {
        Self
    }

    pub fn align_extractions(
        &mut self,
        extraction_groups: &[Vec<data::Extraction>],
        source_text: &str,
        token_offset: usize,
        char_offset: usize,
        enable_fuzzy_alignment: bool,
        fuzzy_alignment_threshold: f64,
        _accept_match_lesser: bool,
    ) -> Vec<Vec<data::Extraction>> {
        let source_tokenized = tokenizer::tokenize(source_text);
        let source_tokens: Vec<String> = source_tokenized.tokens.iter().map(|t| t.text.to_lowercase()).collect();

        let mut aligned_groups = vec![Vec::new(); extraction_groups.len()];

        for (g_idx, group) in extraction_groups.iter().enumerate() {
            for extraction in group {
                let aligned_extraction = self.align_single_extraction(
                    extraction,
                    &source_tokens,
                    &source_tokenized,
                    token_offset,
                    char_offset,
                    enable_fuzzy_alignment,
                    fuzzy_alignment_threshold,
                );
                aligned_groups[g_idx].push(aligned_extraction);
            }
        }

        aligned_groups
    }

    fn align_single_extraction(
        &self,
        extraction: &data::Extraction,
        source_tokens: &[String],
        source_tokenized: &tokenizer::TokenizedText,
        token_offset: usize,
        char_offset: usize,
        enable_fuzzy_alignment: bool,
        fuzzy_alignment_threshold: f64,
    ) -> data::Extraction {
        let ext_tokens: Vec<String> = extraction
            .extraction_text
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        if ext_tokens.is_empty() {
            return extraction.clone();
        }

        // Try exact match first
        if let Some(match_pos) = self.find_exact_match(&ext_tokens, source_tokens) {
            return self.create_aligned_extraction(
                extraction,
                match_pos,
                ext_tokens.len(),
                source_tokenized,
                token_offset,
                char_offset,
                data::AlignmentStatus::MatchExact,
            );
        }

        // Try fuzzy match if enabled
        if enable_fuzzy_alignment
            && let Some((start_idx, window_size)) =
                self.find_fuzzy_match(&ext_tokens, source_tokens, fuzzy_alignment_threshold)
        {
            return self.create_aligned_extraction(
                extraction,
                start_idx,
                window_size,
                source_tokenized,
                token_offset,
                char_offset,
                data::AlignmentStatus::MatchFuzzy,
            );
        }

        // No alignment found
        extraction.clone()
    }

    fn find_exact_match(&self, needle: &[String], haystack: &[String]) -> Option<usize> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return None;
        }

        (0..=(haystack.len() - needle.len())).find(|&start| &haystack[start..start + needle.len()] == needle)
    }

    fn find_fuzzy_match(
        &self,
        ext_tokens: &[String],
        source_tokens: &[String],
        threshold: f64,
    ) -> Option<(usize, usize)> {
        let ext_norm: Vec<String> = ext_tokens.iter().map(|t| normalize_token(t)).collect();
        let mut ext_counts = HashMap::new();
        for token in &ext_norm {
            *ext_counts.entry(token.clone()).or_insert(0usize) += 1;
        }

        let min_overlap = (ext_norm.len() as f64 * threshold).ceil() as usize;
        let mut best_ratio = 0.0f64;
        let mut best_span = None;

        // Try different window sizes
        for window_size in ext_norm.len()..=source_tokens.len() {
            if window_size > source_tokens.len() {
                break;
            }

            for start_idx in 0..=source_tokens.len() - window_size {
                let window: Vec<String> = source_tokens[start_idx..start_idx + window_size]
                    .iter()
                    .map(|t| normalize_token(t))
                    .collect();

                let overlap = self.calculate_overlap(&ext_counts, &window);
                if overlap >= min_overlap {
                    let ratio = overlap as f64 / ext_norm.len() as f64;
                    if ratio > best_ratio {
                        best_ratio = ratio;
                        best_span = Some((start_idx, window_size));
                    }
                }
            }
        }

        if best_ratio >= threshold { best_span } else { None }
    }

    fn calculate_overlap(&self, ext_counts: &HashMap<String, usize>, window_tokens: &[String]) -> usize {
        let mut window_counts = HashMap::new();
        for token in window_tokens {
            *window_counts.entry(token.clone()).or_insert(0usize) += 1;
        }

        ext_counts
            .iter()
            .map(|(token, &ext_count)| {
                let window_count = window_counts.get(token).copied().unwrap_or(0);
                std::cmp::min(ext_count, window_count)
            })
            .sum()
    }

    fn create_aligned_extraction(
        &self,
        extraction: &data::Extraction,
        start_idx: usize,
        length: usize,
        source_tokenized: &tokenizer::TokenizedText,
        token_offset: usize,
        char_offset: usize,
        status: data::AlignmentStatus,
    ) -> data::Extraction {
        let mut new_extraction = extraction.clone();

        new_extraction.token_interval = Some(data::TokenInterval {
            start_index: start_idx + token_offset,
            end_index: start_idx + length + token_offset,
        });

        if start_idx < source_tokenized.tokens.len() && start_idx + length <= source_tokenized.tokens.len() {
            let start_token = &source_tokenized.tokens[start_idx];
            let end_token = &source_tokenized.tokens[start_idx + length - 1];
            new_extraction.char_interval = Some(data::CharInterval {
                start_pos: char_offset + start_token.char_interval.start_pos,
                end_pos: char_offset + end_token.char_interval.end_pos,
            });
        }

        new_extraction.alignment_status = Some(status);
        new_extraction
    }
}

impl Default for WordAligner {
    fn default() -> Self {
        Self::new()
    }
}

/// Lowercase + light plural stemming (remove trailing 's' if >3 chars and not 'ss')
fn normalize_token(tok: &str) -> String {
    let mut s = tok.to_lowercase();
    if s.len() > 3 && s.ends_with('s') && !s.ends_with("ss") {
        s.pop();
    }
    s
}

/// ----------------------------
/// Tests
/// ----------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_string() {
        let resolver = Resolver::new(
            false,
            Some("_index".to_string()),
            Some("_attributes".to_string()),
            false,
        );
        let json = r#"{
            "extractions": [
                {"person": "Alice", "person_index": 1},
                {"location": "Paris", "location_index": 2}
            ]
        }"#;
        let res = resolver.parse_extractions_from_string(json).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].extraction_class, "person");
        assert_eq!(res[0].extraction_text, "Alice");
        assert_eq!(res[1].extraction_class, "location");
    }

    #[test]
    fn test_parse_yaml_fenced() {
        let resolver = Resolver::new(true, Some("_index".to_string()), Some("_attributes".to_string()), true);
        let yaml_fenced = "```yaml\nextractions:\n  - person: Bob\n    person_index: 1\n```";
        let res = resolver.parse_extractions_from_string(yaml_fenced).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].extraction_class, "person");
        assert_eq!(res[0].extraction_text, "Bob");
    }

    #[test]
    fn test_extract_ordering_and_attributes() {
        let resolver = Resolver::new(
            false,
            Some("_index".to_string()),
            Some("_attributes".to_string()),
            false,
        );
        let json = r#"{
            "extractions":[
                {"name":"X", "name_index":5, "name_attributes": {"role":"admin"}},
                {"name":"Y", "name_index":2}
            ]
        }"#;
        let res = resolver.parse_extractions_from_string(json).unwrap();
        assert_eq!(res.len(), 2);
        // sorted by index
        assert_eq!(res[0].extraction_text, "Y");
        assert_eq!(res[1].extraction_text, "X");
        assert!(res[1].attributes.is_some());
    }

    #[test]
    fn test_alignment_exact() {
        let resolver = Resolver::new(false, None, None, false);
        let ex = data::Extraction::new("person".to_string(), "Alice went".to_string(), 1, 0, None);
        let source = "Alice went to the market.";
        let aligned = resolver.align(&[ex], source, 0, Some(0), true, 0.75, true);

        assert_eq!(aligned.len(), 1);
        let a = &aligned[0];
        assert!(a.token_interval.is_some());
        assert!(a.char_interval.is_some());
        assert_eq!(a.alignment_status, Some(data::AlignmentStatus::MatchExact));
    }

    #[test]
    fn test_alignment_fuzzy() {
        let resolver = Resolver::new(false, None, None, false);
        let ex = data::Extraction::new("event".to_string(), "running races".to_string(), 1, 0, None);
        let source = "the race involved many runners and running race participants";
        let aligned = resolver.align(&[ex], source, 0, Some(0), true, 0.3, true);
        assert_eq!(aligned.len(), 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_tokenizer() {
        let tokenized = tokenizer::tokenize("Hello world! 测试");
        assert_eq!(tokenized.tokens.len(), 3);
        assert_eq!(tokenized.tokens[0].text, "Hello");
        assert_eq!(tokenized.tokens[1].text, "world!");
        assert_eq!(tokenized.tokens[2].text, "测试");
    }

    #[test]
    fn test_empty_input() {
        let resolver = Resolver::default();
        let result = resolver.parse_extractions_from_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json() {
        let resolver = Resolver::new(false, None, None, false);
        let result = resolver.parse_extractions_from_string("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_simple_yaml_array() {
        let resolver = Resolver::new(true, None, None, true);
        let yaml = r#"```yaml
- Alice
- Bob
- Charlie
```"#;
        let result = resolver.parse_extractions_from_string(yaml).unwrap();
        assert_eq!(result.len(), 3);

        // Check that we got the expected names (in any order)
        let texts: Vec<&str> = result.iter().map(|e| e.extraction_text.as_str()).collect();
        assert!(texts.contains(&"Alice"));
        assert!(texts.contains(&"Bob"));
        assert!(texts.contains(&"Charlie"));

        // Check that all have the expected class prefix
        for extraction in &result {
            assert!(extraction.extraction_class.starts_with("text"));
        }
    }

    #[test]
    fn test_parse_simple_json_array() {
        let resolver = Resolver::new(true, None, None, false);
        let json = r#"```json
["Alice", "Bob", "Charlie"]
```"#;
        let result = resolver.parse_extractions_from_string(json).unwrap();
        assert_eq!(result.len(), 3);

        // Check that we got the expected names (in any order)
        let texts: Vec<&str> = result.iter().map(|e| e.extraction_text.as_str()).collect();
        assert!(texts.contains(&"Alice"));
        assert!(texts.contains(&"Bob"));
        assert!(texts.contains(&"Charlie"));

        // Check that all have the expected class prefix
        for extraction in &result {
            assert!(extraction.extraction_class.starts_with("text"));
        }
    }

    #[test]
    fn test_parse_nested_category_format() {
        let resolver = Resolver::new(true, None, None, true);
        let yaml = r#"```yaml
characters:
  - 宝玉
  - 袭人
  - 林姑娘
  - 黛玉
locations:
  - 怡红院
  - 潇湘馆
objects:
  - 月白缎子袍子
  - 丝绦
  - 紫金冠
  - 云头履
```"#;
        let result = resolver.parse_extractions_from_string(yaml).unwrap();
        assert!(result.len() > 0);

        // Check that we got the expected names (in any order)
        let texts: Vec<&str> = result.iter().map(|e| e.extraction_text.as_str()).collect();
        assert!(texts.contains(&"宝玉"));
        assert!(texts.contains(&"袭人"));
        assert!(texts.contains(&"怡红院"));
        assert!(texts.contains(&"潇湘馆"));
        assert!(texts.contains(&"月白缎子袍子"));

        // Check that categories are used as class names
        let classes: Vec<&str> = result.iter().map(|e| e.extraction_class.as_str()).collect();
        assert!(classes.iter().any(|c| c.starts_with("characters")));
        assert!(classes.iter().any(|c| c.starts_with("locations")));
        assert!(classes.iter().any(|c| c.starts_with("objects")));
    }
}
