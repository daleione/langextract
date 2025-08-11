//! Utility functions for visualizing LangExtract extractions in notebooks.
//!
//! # Example
//! ```rust
//! use langextract::visualization::{visualize, VisualizeOptions, DataSource};
//! use langextract::data::AnnotatedDocument;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let doc = AnnotatedDocument::new(
//!         Some("test_id".to_string()),
//!         Some(vec![]),
//!         Some("Hello world! This is a test document.".to_string())
//!     );
//!     let html = visualize(DataSource::Document(doc), VisualizeOptions::default())?;
//!     println!("{}", html);
//!     Ok(())
//! }
//! ```

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use html_escape::encode_text;
use serde_json;

use crate::data::{AnnotatedDocument, AttributeValue, Extraction};

/// Color palette for highlighting extractions
const PALETTE: &[&str] = &[
    "#D2E3FC", // Light Blue (Primary Container)
    "#C8E6C9", // Light Green (Tertiary Container)
    "#FEF0C3", // Light Yellow (Primary Color)
    "#F9DEDC", // Light Red (Error Container)
    "#FFDDBE", // Light Orange (Tertiary Container)
    "#EADDFF", // Light Purple (Secondary/Tertiary Container)
    "#C4E9E4", // Light Teal (Teal Container)
    "#FCE4EC", // Light Pink (Pink Container)
    "#E8EAED", // Very Light Grey (Neutral Highlight)
    "#DDE8E8", // Pale Cyan (Cyan Container)
];

/// CSS styles for visualization
const VISUALIZATION_CSS: &str = r#"
<style>
.lx-highlight { position: relative; border-radius:3px; padding:1px 2px;}
.lx-highlight .lx-tooltip {
  visibility: hidden;
  opacity: 0;
  transition: opacity 0.2s ease-in-out;
  background: #333;
  color: #fff;
  text-align: left;
  border-radius: 4px;
  padding: 6px 8px;
  position: absolute;
  z-index: 1000;
  bottom: 125%;
  left: 50%;
  transform: translateX(-50%);
  font-size: 12px;
  max-width: 240px;
  white-space: normal;
  box-shadow: 0 2px 6px rgba(0,0,0,0.3);
}
.lx-highlight:hover .lx-tooltip { visibility: visible; opacity:1; }
.lx-animated-wrapper { max-width: 100%; font-family: Arial, sans-serif; }
.lx-controls {
  background: #fafafa; border: 1px solid #90caf9; border-radius: 8px;
  padding: 12px; margin-bottom: 16px;
}
.lx-button-row {
  display: flex; justify-content: center; gap: 8px; margin-bottom: 12px;
}
.lx-control-btn {
  background: #4285f4; color: white; border: none; border-radius: 4px;
  padding: 8px 16px; cursor: pointer; font-size: 13px; font-weight: 500;
  transition: background-color 0.2s;
}
.lx-control-btn:hover { background: #3367d6; }
.lx-progress-container {
  margin-bottom: 8px;
}
.lx-progress-slider {
  width: 100%; margin: 0; appearance: none; height: 6px;
  background: #ddd; border-radius: 3px; outline: none;
}
.lx-progress-slider::-webkit-slider-thumb {
  appearance: none; width: 18px; height: 18px; background: #4285f4;
  border-radius: 50%; cursor: pointer;
}
.lx-progress-slider::-moz-range-thumb {
  width: 18px; height: 18px; background: #4285f4; border-radius: 50%;
  cursor: pointer; border: none;
}
.lx-status-text {
  text-align: center; font-size: 12px; color: #666; margin-top: 4px;
}
.lx-text-window {
  font-family: monospace; white-space: pre-wrap; border: 1px solid #90caf9;
  padding: 12px; max-height: 260px; overflow-y: auto; margin-bottom: 12px;
  line-height: 1.6;
}
.lx-attributes-panel {
  background: #fafafa; border: 1px solid #90caf9; border-radius: 6px;
  padding: 8px 10px; margin-top: 8px; font-size: 13px;
}
.lx-current-highlight {
  border-bottom: 4px solid #ff4444;
  font-weight: bold;
  animation: lx-pulse 1s ease-in-out;
}
@keyframes lx-pulse {
  0% { text-decoration-color: #ff4444; }
  50% { text-decoration-color: #ff0000; }
  100% { text-decoration-color: #ff4444; }
}
.lx-legend {
  font-size: 12px; margin-bottom: 8px;
  padding-bottom: 8px; border-bottom: 1px solid #e0e0e0;
}
.lx-label {
  display: inline-block;
  padding: 2px 4px;
  border-radius: 3px;
  margin-right: 4px;
  color: #000;
}
.lx-attr-key {
  font-weight: 600;
  color: #1565c0;
  letter-spacing: 0.3px;
}
.lx-attr-value {
  font-weight: 400;
  opacity: 0.85;
  letter-spacing: 0.2px;
}

/* Add optimizations with larger fonts and better readability for GIFs */
.lx-gif-optimized .lx-text-window { font-size: 16px; line-height: 1.8; }
.lx-gif-optimized .lx-attributes-panel { font-size: 15px; }
.lx-gif-optimized .lx-current-highlight { text-decoration-thickness: 4px; }
</style>
"#;

/// Enum for span boundary tag types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagType {
    Start,
    End,
}

/// Represents a span boundary point for HTML generation
#[derive(Debug, Clone)]
struct SpanPoint {
    /// Character position in the text
    position: usize,
    /// Type of span boundary (Start or End)
    tag_type: TagType,
    /// Index of the span for HTML data-idx attribute
    span_idx: usize,
    /// The extraction data associated with this span
    extraction: Extraction,
}

/// Options for visualization
#[derive(Debug, Clone)]
pub struct VisualizeOptions {
    /// Animation speed in seconds between extractions
    pub animation_speed: f32,
    /// If true, appends a color legend mapping extraction classes to colors
    pub show_legend: bool,
    /// If true, applies GIF-optimized styling with larger fonts
    pub gif_optimized: bool,
    /// Number of context characters to show around extractions
    pub context_chars: usize,
}

impl Default for VisualizeOptions {
    fn default() -> Self {
        Self {
            animation_speed: 1.0,
            show_legend: true,
            gif_optimized: true,
            context_chars: 150,
        }
    }
}

/// Data structure for JavaScript extraction data
#[derive(serde::Serialize)]
struct ExtractionData {
    index: usize,
    #[serde(rename = "class")]
    class_name: String,
    text: String,
    color: String,
    #[serde(rename = "startPos")]
    start_pos: usize,
    #[serde(rename = "endPos")]
    end_pos: usize,
    #[serde(rename = "beforeText")]
    before_text: String,
    #[serde(rename = "extractionText")]
    extraction_text: String,
    #[serde(rename = "afterText")]
    after_text: String,
    #[serde(rename = "attributesHtml")]
    attributes_html: String,
}

/// Error type for visualization operations
#[derive(Debug, thiserror::Error)]
pub enum VisualizeError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("No documents found in JSONL file")]
    NoDocuments,
    #[error("Document must contain text to visualize")]
    NoText,
    #[error("Document must contain extractions to visualize")]
    NoExtractions,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Assigns a background color to each extraction class
fn assign_colors(extractions: &[&Extraction]) -> HashMap<String, &'static str> {
    let classes: BTreeSet<_> = extractions
        .iter()
        .filter_map(|e| {
            e.char_interval.as_ref().and_then(|interval| {
                if interval.start_pos.is_some() && interval.end_pos.is_some() {
                    Some(&e.extraction_class)
                } else {
                    None
                }
            })
        })
        .collect();

    classes
        .into_iter()
        .zip(PALETTE.iter().cycle())
        .map(|(class, &color)| (class.clone(), color))
        .collect()
}

/// Filters extractions to only include those with valid char intervals
fn filter_valid_extractions(extractions: &[Extraction]) -> Vec<&Extraction> {
    extractions
        .iter()
        .filter(|e| {
            e.char_interval.as_ref().is_some_and(|interval| {
                interval.start_pos.is_some()
                    && interval.end_pos.is_some()
                    && interval.start_pos.unwrap() < interval.end_pos.unwrap()
            })
        })
        .collect()
}

/// Builds highlighted text with proper HTML nesting
fn build_highlighted_text(
    text: &str,
    extractions: &[&Extraction],
    color_map: &HashMap<String, &str>,
) -> Result<String, VisualizeError> {
    let mut points = Vec::new();
    let mut span_lengths = HashMap::new();

    for (index, extraction) in extractions.iter().enumerate() {
        let interval = extraction.char_interval.as_ref().unwrap();
        let start_pos = interval.start_pos.unwrap();
        let end_pos = interval.end_pos.unwrap();
        let span_length = end_pos - start_pos;

        points.push(SpanPoint {
            position: start_pos,
            tag_type: TagType::Start,
            span_idx: index,
            extraction: (*extraction).clone(),
        });
        points.push(SpanPoint {
            position: end_pos,
            tag_type: TagType::End,
            span_idx: index,
            extraction: (*extraction).clone(),
        });

        span_lengths.insert(index, span_length);
    }

    // Sort points for proper HTML nesting
    points.sort_by(|a, b| {
        use std::cmp::Ordering;

        match a.position.cmp(&b.position) {
            Ordering::Equal => {
                let a_span_length = span_lengths.get(&a.span_idx).unwrap_or(&0);
                let b_span_length = span_lengths.get(&b.span_idx).unwrap_or(&0);

                match (a.tag_type, b.tag_type) {
                    (TagType::End, TagType::Start) => Ordering::Less,
                    (TagType::Start, TagType::End) => Ordering::Greater,
                    (TagType::End, TagType::End) => a_span_length.cmp(b_span_length),
                    (TagType::Start, TagType::Start) => b_span_length.cmp(a_span_length),
                }
            }
            other => other,
        }
    });

    let mut html_parts = Vec::new();
    let mut cursor = 0;

    for point in points {
        if point.position > cursor {
            html_parts.push(encode_text(&text[cursor..point.position]).to_string());
        }

        match point.tag_type {
            TagType::Start => {
                let color = color_map.get(&point.extraction.extraction_class).unwrap_or(&"#ffff8d");
                let highlight_class = if point.span_idx == 0 {
                    " lx-current-highlight"
                } else {
                    ""
                };

                html_parts.push(format!(
                    r#"<span class="lx-highlight{}" data-idx="{}" style="background-color:{};">"#,
                    highlight_class, point.span_idx, color
                ));
            }
            TagType::End => {
                html_parts.push("</span>".to_string());
            }
        }

        cursor = point.position;
    }

    if cursor < text.len() {
        html_parts.push(encode_text(&text[cursor..]).to_string());
    }

    Ok(html_parts.join(""))
}

/// Builds legend HTML showing extraction classes and their colors
fn build_legend_html(color_map: &HashMap<String, &str>) -> String {
    if color_map.is_empty() {
        return String::new();
    }

    let legend_items: Vec<_> = color_map
        .iter()
        .map(|(class, color)| {
            format!(
                r#"<span class="lx-label" style="background-color:{};">{}</span>"#,
                color,
                encode_text(class)
            )
        })
        .collect();

    format!(
        r#"<div class="lx-legend">Highlights Legend: {}</div>"#,
        legend_items.join(" ")
    )
}

/// Formats attributes as a single-line string
fn format_attributes(attributes: &Option<HashMap<String, AttributeValue>>) -> String {
    let Some(attrs) = attributes else {
        return "{}".to_string();
    };

    if attrs.is_empty() {
        return "{}".to_string();
    }

    let mut attrs_parts = Vec::new();
    for (key, value) in attrs {
        let value_str = match value {
            AttributeValue::Single(s) if s.is_empty() => continue,
            AttributeValue::Single(s) => s.clone(),
            AttributeValue::Multiple(arr) => arr
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        };

        if value_str.is_empty() {
            continue;
        }

        attrs_parts.push(format!(
            r#"<span class="lx-attr-key">{}</span>: <span class="lx-attr-value">{}</span>"#,
            encode_text(key),
            encode_text(&value_str)
        ));
    }

    if attrs_parts.is_empty() {
        "{}".to_string()
    } else {
        format!("{{{}}}", attrs_parts.join(", "))
    }
}

/// Prepares JavaScript data for extractions
fn prepare_extraction_data(
    text: &str,
    extractions: &[&Extraction],
    color_map: &HashMap<String, &str>,
    context_chars: usize,
) -> Vec<ExtractionData> {
    extractions
        .iter()
        .enumerate()
        .map(|(i, extraction)| {
            let interval = extraction.char_interval.as_ref().unwrap();
            let start_pos = interval.start_pos.unwrap();
            let end_pos = interval.end_pos.unwrap();

            let context_start = start_pos.saturating_sub(context_chars);
            let context_end = (end_pos + context_chars).min(text.len());

            let before_text = &text[context_start..start_pos];
            let extraction_text = &text[start_pos..end_pos];
            let after_text = &text[end_pos..context_end];

            let color = color_map.get(&extraction.extraction_class).unwrap_or(&"#ffff8d");

            let attributes_html = format!(
                r#"<div><strong>class:</strong> {}</div><div><strong>attributes:</strong> {}</div>"#,
                encode_text(&extraction.extraction_class),
                format_attributes(&extraction.attributes)
            );

            ExtractionData {
                index: i,
                class_name: extraction.extraction_class.clone(),
                text: extraction.extraction_text.clone(),
                color: color.to_string(),
                start_pos,
                end_pos,
                before_text: encode_text(before_text).to_string(),
                extraction_text: encode_text(extraction_text).to_string(),
                after_text: encode_text(after_text).to_string(),
                attributes_html,
            }
        })
        .collect()
}

/// Builds the complete visualization HTML
fn build_visualization_html(
    text: &str,
    extractions: &[&Extraction],
    color_map: &HashMap<String, &str>,
    options: &VisualizeOptions,
) -> Result<String, VisualizeError> {
    if extractions.is_empty() {
        return Ok(r#"<div class="lx-animated-wrapper"><p>No extractions to animate.</p></div>"#.to_string());
    }

    // Sort extractions by position for proper HTML nesting
    let mut sorted_extractions = extractions.to_vec();
    sorted_extractions.sort_by(|a, b| {
        let a_interval = a.char_interval.as_ref().unwrap();
        let b_interval = b.char_interval.as_ref().unwrap();
        let a_start = a_interval.start_pos.unwrap();
        let b_start = b_interval.start_pos.unwrap();

        match a_start.cmp(&b_start) {
            std::cmp::Ordering::Equal => {
                let a_length = a_interval.end_pos.unwrap() - a_start;
                let b_length = b_interval.end_pos.unwrap() - b_start;
                b_length.cmp(&a_length) // Longer spans first
            }
            other => other,
        }
    });

    let highlighted_text = build_highlighted_text(text, &sorted_extractions, color_map)?;
    let extraction_data = prepare_extraction_data(text, &sorted_extractions, color_map, options.context_chars);
    let legend_html = if options.show_legend {
        build_legend_html(color_map)
    } else {
        String::new()
    };

    let js_data = serde_json::to_string(&extraction_data)?;

    let first_extraction = sorted_extractions[0];
    let first_interval = first_extraction.char_interval.as_ref().unwrap();
    let pos_info_str = format!(
        "[{}-{}]",
        first_interval.start_pos.unwrap(),
        first_interval.end_pos.unwrap()
    );

    let html_content = format!(
        r#"
<div class="lx-animated-wrapper">
  <div class="lx-attributes-panel">
    {}
    <div id="attributesContainer"></div>
  </div>
  <div class="lx-text-window" id="textWindow">
    {}
  </div>
  <div class="lx-controls">
    <div class="lx-button-row">
      <button class="lx-control-btn" onclick="playPause()">▶️ Play</button>
      <button class="lx-control-btn" onclick="prevExtraction()">⏮ Previous</button>
      <button class="lx-control-btn" onclick="nextExtraction()">⏭ Next</button>
    </div>
    <div class="lx-progress-container">
      <input type="range" id="progressSlider" class="lx-progress-slider"
             min="0" max="{}" value="0"
             onchange="jumpToExtraction(this.value)">
    </div>
    <div class="lx-status-text">
      Entity <span id="entityInfo">1/{}</span> |
      Pos <span id="posInfo">{}</span>
    </div>
  </div>
</div>

<script>
  (function() {{
    const extractions = {};
    let currentIndex = 0;
    let isPlaying = false;
    let animationInterval = null;
    let animationSpeed = {};

    function updateDisplay() {{
      const extraction = extractions[currentIndex];
      if (!extraction) return;

      document.getElementById('attributesContainer').innerHTML = extraction.attributesHtml;
      document.getElementById('entityInfo').textContent = (currentIndex + 1) + '/' + extractions.length;
      document.getElementById('posInfo').textContent = '[' + extraction.startPos + '-' + extraction.endPos + ']';
      document.getElementById('progressSlider').value = currentIndex;

      const playBtn = document.querySelector('.lx-control-btn');
      if (playBtn) playBtn.textContent = isPlaying ? '⏸ Pause' : '▶️ Play';

      const prevHighlight = document.querySelector('.lx-text-window .lx-current-highlight');
      if (prevHighlight) prevHighlight.classList.remove('lx-current-highlight');
      const currentSpan = document.querySelector('.lx-text-window span[data-idx="' + currentIndex + '"]');
      if (currentSpan) {{
        currentSpan.classList.add('lx-current-highlight');
        currentSpan.scrollIntoView({{block: 'center', behavior: 'smooth'}});
      }}
    }}

    function nextExtraction() {{
      currentIndex = (currentIndex + 1) % extractions.length;
      updateDisplay();
    }}

    function prevExtraction() {{
      currentIndex = (currentIndex - 1 + extractions.length) % extractions.length;
      updateDisplay();
    }}

    function jumpToExtraction(index) {{
      currentIndex = parseInt(index);
      updateDisplay();
    }}

    function playPause() {{
      if (isPlaying) {{
        clearInterval(animationInterval);
        isPlaying = false;
      }} else {{
        animationInterval = setInterval(nextExtraction, animationSpeed * 1000);
        isPlaying = true;
      }}
      updateDisplay();
    }}

    window.playPause = playPause;
    window.nextExtraction = nextExtraction;
    window.prevExtraction = prevExtraction;
    window.jumpToExtraction = jumpToExtraction;

    updateDisplay();
  }})();
</script>"#,
        legend_html,
        highlighted_text,
        extractions.len() - 1,
        extractions.len(),
        pos_info_str,
        js_data,
        options.animation_speed
    );

    Ok(html_content)
}

/// Data source for visualization
pub enum DataSource {
    Document(AnnotatedDocument),
    Path(Box<dyn AsRef<Path>>),
}

/// Visualizes extraction data as animated highlighted HTML
pub fn visualize(data_source: DataSource, options: VisualizeOptions) -> Result<String, VisualizeError> {
    let annotated_doc = match data_source {
        DataSource::Document(doc) => doc,
        DataSource::Path(_path) => {
            // Since we don't have access to the IO module, we'll return an error for now
            return Err(VisualizeError::FileNotFound("File loading not implemented".to_string()));
        }
    };

    let text = annotated_doc.text.as_ref().ok_or(VisualizeError::NoText)?;

    let extractions = annotated_doc
        .extractions
        .as_ref()
        .ok_or(VisualizeError::NoExtractions)?;

    let valid_extractions = filter_valid_extractions(extractions);

    if valid_extractions.is_empty() {
        let empty_html = r#"<div class="lx-animated-wrapper"><p>No valid extractions to animate.</p></div>"#;
        return Ok(format!("{}{}", VISUALIZATION_CSS, empty_html));
    }

    let color_map = assign_colors(&valid_extractions);
    let visualization_html = build_visualization_html(text, &valid_extractions, &color_map, &options)?;

    let mut full_html = format!("{}{}", VISUALIZATION_CSS, visualization_html);

    // Apply GIF optimizations if requested
    if options.gif_optimized {
        full_html = full_html.replace(
            r#"class="lx-animated-wrapper""#,
            r#"class="lx-animated-wrapper lx-gif-optimized""#,
        );
    }

    Ok(full_html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::AnnotatedDocument;
    use std::collections::HashMap;

    fn create_test_document() -> AnnotatedDocument {
        // Note: This is a placeholder. In real implementation, you'd use the proper
        // constructor methods from the AnnotatedDocument API
        // For now, we'll assume there are builder methods or constructors available
        AnnotatedDocument::new(
            Some("test_id".to_string()),
            Some(vec![]), // extractions - would need to create proper extractions
            Some("Hello world! This is a test document.".to_string()),
        )
    }

    #[test]
    fn test_assign_colors() {
        // This test would need to be implemented once we have access to proper
        // Extraction creation methods
        let extractions = vec![];
        let color_map = assign_colors(&extractions);
        assert!(color_map.is_empty());
    }

    #[test]
    fn test_filter_valid_extractions() {
        let extractions = vec![];
        let valid = filter_valid_extractions(&extractions);
        assert_eq!(valid.len(), 0);
    }

    #[test]
    fn test_build_legend_html_empty() {
        let color_map = HashMap::new();
        let legend = build_legend_html(&color_map);
        assert_eq!(legend, "");
    }

    #[test]
    fn test_format_attributes_empty() {
        assert_eq!(format_attributes(&None), "{}");
        assert_eq!(format_attributes(&Some(HashMap::new())), "{}");
    }

    #[test]
    fn test_options_default() {
        let options = VisualizeOptions::default();
        assert_eq!(options.animation_speed, 1.0);
        assert_eq!(options.show_legend, true);
        assert_eq!(options.gif_optimized, true);
        assert_eq!(options.context_chars, 150);
    }

    #[test]
    fn test_visualize_empty_extractions() -> Result<(), VisualizeError> {
        let doc = create_test_document();
        let html = visualize(DataSource::Document(doc), VisualizeOptions::default())?;
        assert!(html.contains("No valid extractions to animate"));
        Ok(())
    }
}
