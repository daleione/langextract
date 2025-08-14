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

/// Visualization style options
#[derive(Debug, Clone, PartialEq)]
pub enum VisualizationStyle {
    /// Original animated style with controls
    Animated,
    /// Static Chinese classical style
    ChineseClassical,
}

/// Represents a span boundary point for HTML generation
#[derive(Debug, Clone)]
struct SpanPoint<'a> {
    /// Character position in the text
    position: usize,
    /// Type of span boundary (Start or End)
    tag_type: TagType,
    /// Index of the span for HTML data-idx attribute
    span_idx: usize,
    /// The extraction data associated with this span
    extraction: &'a Extraction,
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
    /// Visualization style to use
    pub style: VisualizationStyle,
}

impl Default for VisualizeOptions {
    fn default() -> Self {
        Self {
            animation_speed: 1.0,
            show_legend: true,
            gif_optimized: true,
            context_chars: 150,
            style: VisualizationStyle::Animated,
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
    use std::cmp::Ordering;
    // Convert text to character vector for safe indexing
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

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
            extraction,
        });

        points.push(SpanPoint {
            position: end_pos,
            tag_type: TagType::End,
            span_idx: index,
            extraction,
        });

        span_lengths.insert(index, span_length);
    }

    points.sort_by(|a, b| match a.position.cmp(&b.position) {
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
    });

    let mut html_parts = Vec::new();
    let mut cursor = 0;

    for point in points {
        if point.position > cursor {
            // Extract characters from cursor to point.position and convert to string
            let text_slice: String = chars[cursor..point.position.min(total_chars)].iter().collect();
            html_parts.push(encode_text(&text_slice).to_string());
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

    if cursor < total_chars {
        // Extract remaining characters and convert to string
        let remaining_text: String = chars[cursor..].iter().collect();
        html_parts.push(encode_text(&remaining_text).to_string());
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
    // Convert text to character vector for safe indexing
    let chars: Vec<char> = text.chars().collect();
    let char_count = chars.len();

    extractions
        .iter()
        .enumerate()
        .map(|(i, extraction)| {
            let interval = extraction.char_interval.as_ref().unwrap();
            let start_pos = interval.start_pos.unwrap();
            let end_pos = interval.end_pos.unwrap();

            let context_start = start_pos.saturating_sub(context_chars);
            let context_end = (end_pos + context_chars).min(char_count);

            // Extract character ranges and convert back to strings
            let before_chars: String = chars[context_start..start_pos].iter().collect();
            let extraction_chars: String = chars[start_pos..end_pos].iter().collect();
            let after_chars: String = chars[end_pos..context_end].iter().collect();

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
                before_text: encode_text(&before_chars).to_string(),
                extraction_text: encode_text(&extraction_chars).to_string(),
                after_text: encode_text(&after_chars).to_string(),
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
        let empty_html = match options.style {
            VisualizationStyle::Animated => {
                r#"<div class="lx-animated-wrapper"><p>No valid extractions to animate.</p></div>"#
            }
            VisualizationStyle::ChineseClassical => {
                r#"<div class="chinese-container"><p>没有可显示的提取结果</p></div>"#
            }
        };
        return Ok(format!("{}{}", get_css_for_style(&options.style), empty_html));
    }

    let color_map = assign_colors(&valid_extractions);

    let visualization_html = match options.style {
        VisualizationStyle::Animated => build_visualization_html(text, &valid_extractions, &color_map, &options)?,
        VisualizationStyle::ChineseClassical => {
            build_chinese_classical_html(text, &valid_extractions, &color_map, &options)?
        }
    };

    let mut full_html = format!("{}{}", get_css_for_style(&options.style), visualization_html);

    // Apply GIF optimizations if requested for animated style
    if options.gif_optimized && options.style == VisualizationStyle::Animated {
        full_html = full_html.replace(
            r#"class="lx-animated-wrapper""#,
            r#"class="lx-animated-wrapper lx-gif-optimized""#,
        );
    }

    Ok(full_html)
}

fn get_css_for_style(style: &VisualizationStyle) -> &'static str {
    match style {
        VisualizationStyle::Animated => VISUALIZATION_CSS,
        VisualizationStyle::ChineseClassical => CHINESE_CLASSICAL_CSS,
    }
}

fn build_chinese_classical_html(
    text: &str,
    extractions: &[&Extraction],
    color_map: &HashMap<String, &str>,
    _options: &VisualizeOptions,
) -> Result<String, VisualizeError> {
    use std::collections::HashMap;

    // Count extractions by class
    let mut category_counts: HashMap<String, Vec<&Extraction>> = HashMap::new();
    for extraction in extractions {
        category_counts
            .entry(extraction.extraction_class.clone())
            .or_insert_with(Vec::new)
            .push(extraction);
    }

    let mut html = String::new();

    // HTML document header
    html.push_str(
        r#"<div class="chinese-container">
    <div class="chinese-header">
        <h1>🏮 古典文本实体可视化</h1>
        <p>现代AI技术与传统文学的完美融合</p>
    </div>

    <div class="chinese-content">
        <div class="chinese-decoration">🏮 ◆ ❋ ◆ 🏮</div>
"#,
    );

    // Statistics section with clickable items
    html.push_str(
        r#"        <div class="chinese-statistics">
            <h3>📊 提取统计</h3>
            <div class="stat-grid">
"#,
    );

    html.push_str(&format!(
        r#"                <div class="stat-item clickable" onclick="showExtractionDetails('all')">
                    <div class="stat-number">{}</div>
                    <div class="stat-label">总计实体</div>
                </div>"#,
        extractions.len()
    ));

    for (category, extractions_in_category) in &category_counts {
        let category_name = get_chinese_category_name(category);
        html.push_str(&format!(
            r#"                <div class="stat-item clickable" onclick="showExtractionDetails('{}')">
                    <div class="stat-number">{}</div>
                    <div class="stat-label">{}</div>
                </div>"#,
            category,
            extractions_in_category.len(),
            category_name
        ));
    }

    html.push_str(
        r#"            </div>
        </div>
"#,
    );

    // Legend
    html.push_str(
        r#"        <div class="chinese-legend">
            <div class="legend-title">🎨 实体类型图例</div>
            <div class="legend-grid">
"#,
    );

    for (class_name, color) in color_map {
        let category_name = get_chinese_category_name(class_name);
        let icon = get_category_icon(class_name);
        html.push_str(&format!(
            r#"                <div class="legend-item">
                    <div class="legend-color" style="background-color: {}"></div>
                    <span>{} {}</span>
                </div>"#,
            color, icon, category_name
        ));
    }

    html.push_str(
        r#"            </div>
        </div>
"#,
    );

    // Highlighted text
    html.push_str(
        r#"        <div class="chinese-text-content">
"#,
    );

    let highlighted_text = build_chinese_highlighted_text(text, extractions, color_map)?;
    html.push_str(&highlighted_text);

    html.push_str(
        r#"        </div>

        <div class="chinese-decoration">🌸 ◆ 🏛️ ◆ 📿 ◆ 👘 ◆ 🌸</div>
    </div>

    <div class="chinese-footer">
        <p>🏮 LangExtract 实体提取可视化 🏮</p>
        <p>展示了从文本中提取的 "#,
    );
    html.push_str(&extractions.len().to_string());
    html.push_str(
        r#" 个实体</p>
    </div>
</div>

<!-- Modal for extraction details -->
<div id="extractionModal" class="modal" onclick="closeModal()">
    <div class="modal-content" onclick="event.stopPropagation()">
        <div class="modal-header">
            <h2 id="modalTitle">提取详情</h2>
            <span class="close" onclick="closeModal()">&times;</span>
        </div>
        <div class="modal-body" id="modalBody">
        </div>
    </div>
</div>

<script>
const extractionData = "#,
    );

    // Generate JavaScript data
    html.push_str(&generate_extraction_js_data(extractions, &category_counts)?);

    html.push_str(
        r#";

function showExtractionDetails(category) {
    const modal = document.getElementById('extractionModal');
    const modalTitle = document.getElementById('modalTitle');
    const modalBody = document.getElementById('modalBody');

    let title, items;
    if (category === 'all') {
        title = '所有提取实体';
        items = extractionData.all;
    } else {
        title = getCategoryName(category) + ' 实体';
        items = extractionData.categories[category] || [];
    }

    modalTitle.textContent = title + ' (' + items.length + '个)';

    let html = '<div class="extraction-list">';
    items.forEach((item, index) => {
        html += `
            <div class="extraction-item">
                <div class="extraction-text">${item.text}</div>
                <div class="extraction-meta">
                    <span class="extraction-class">${getCategoryName(item.class)}</span>
                    <span class="extraction-position">[${item.start}-${item.end}]</span>
                </div>
                ${item.attributes ? `<div class="extraction-attributes">${item.attributes}</div>` : ''}
            </div>
        `;
    });
    html += '</div>';

    modalBody.innerHTML = html;
    modal.style.display = 'block';
}

function closeModal() {
    document.getElementById('extractionModal').style.display = 'none';
}

function getCategoryName(category) {
    const names = {
        'characters': '👤 人物角色',
        'locations': '🏛️ 地点场所',
        'objects': '📿 物品器具',
        'clothing': '👘 服饰装扮',
        'emotions': '💭 情感状态',
        'nature': '🌸 自然景物'
    };
    return names[category] || category;
}

// Add hover effects
document.addEventListener('DOMContentLoaded', function() {
    const highlights = document.querySelectorAll('.highlight');
    highlights.forEach(function(highlight) {
        highlight.addEventListener('mouseenter', function() {
            this.style.transform = 'scale(1.05)';
            this.style.zIndex = '10';
        });

        highlight.addEventListener('mouseleave', function() {
            this.style.transform = 'scale(1)';
            this.style.zIndex = 'auto';
        });
    });
});
</script>
</body>
</html>
"#,
    );

    Ok(html)
}

fn build_chinese_highlighted_text(
    text: &str,
    extractions: &[&Extraction],
    color_map: &HashMap<String, &str>,
) -> Result<String, VisualizeError> {
    // Convert text to character vector for safe indexing
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

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
            extraction,
        });

        points.push(SpanPoint {
            position: end_pos,
            tag_type: TagType::End,
            span_idx: index,
            extraction,
        });

        span_lengths.insert(index, span_length);
    }

    points.sort_by(|a, b| match a.position.cmp(&b.position) {
        std::cmp::Ordering::Equal => {
            let a_span_length = span_lengths.get(&a.span_idx).unwrap_or(&0);
            let b_span_length = span_lengths.get(&b.span_idx).unwrap_or(&0);

            match (a.tag_type, b.tag_type) {
                (TagType::End, TagType::Start) => std::cmp::Ordering::Less,
                (TagType::Start, TagType::End) => std::cmp::Ordering::Greater,
                (TagType::End, TagType::End) => a_span_length.cmp(b_span_length),
                (TagType::Start, TagType::Start) => b_span_length.cmp(a_span_length),
            }
        }
        other => other,
    });

    let mut html_parts = Vec::new();
    let mut cursor = 0;

    for point in points {
        if point.position > cursor {
            // Extract characters from cursor to point.position and convert to string
            let text_slice: String = chars[cursor..point.position.min(total_chars)].iter().collect();
            html_parts.push(text_slice);
        }

        match point.tag_type {
            TagType::Start => {
                let color = color_map.get(&point.extraction.extraction_class).unwrap_or(&"#ddd");
                let attributes_text = format_attributes(&point.extraction.attributes);
                let tooltip_content = if attributes_text.is_empty() {
                    format!(
                        "类型: {}",
                        get_chinese_category_name(&point.extraction.extraction_class)
                    )
                } else {
                    format!(
                        "类型: {} | {}",
                        get_chinese_category_name(&point.extraction.extraction_class),
                        attributes_text
                    )
                };

                html_parts.push(format!(
                    r#"<span class="highlight" style="background-color: {}; border-color: {};" title="{}">"#,
                    color, color, tooltip_content
                ));
            }
            TagType::End => {
                html_parts.push("</span>".to_string());
            }
        }

        cursor = point.position;
    }

    // Add remaining text after the last point
    if cursor < total_chars {
        let text_slice: String = chars[cursor..total_chars].iter().collect();
        html_parts.push(text_slice);
    }

    Ok(html_parts.join(""))
}

fn generate_extraction_js_data(
    extractions: &[&Extraction],
    category_counts: &HashMap<String, Vec<&Extraction>>,
) -> Result<String, VisualizeError> {
    use serde_json::json;

    let mut all_items = Vec::new();
    let mut categories = serde_json::Map::new();

    for extraction in extractions {
        let interval = extraction.char_interval.as_ref().unwrap();
        let item = json!({
            "text": extraction.extraction_text,
            "class": extraction.extraction_class,
            "start": interval.start_pos.unwrap(),
            "end": interval.end_pos.unwrap(),
            "attributes": format_attributes(&extraction.attributes)
        });
        all_items.push(item);
    }

    for (category, extractions_in_category) in category_counts {
        let mut items = Vec::new();
        for extraction in extractions_in_category {
            let interval = extraction.char_interval.as_ref().unwrap();
            let item = json!({
                "text": extraction.extraction_text,
                "class": extraction.extraction_class,
                "start": interval.start_pos.unwrap(),
                "end": interval.end_pos.unwrap(),
                "attributes": format_attributes(&extraction.attributes)
            });
            items.push(item);
        }
        categories.insert(category.clone(), json!(items));
    }

    let data = json!({
        "all": all_items,
        "categories": categories
    });

    Ok(data.to_string())
}

fn get_chinese_category_name(category: &str) -> &'static str {
    match category {
        "characters" => "人物角色",
        "locations" => "地点场所",
        "objects" => "物品器具",
        "clothing" => "服饰装扮",
        "emotions" => "情感状态",
        "nature" => "自然景物",
        _ => "其他",
    }
}

fn get_category_icon(category: &str) -> &'static str {
    match category {
        "characters" => "👤",
        "locations" => "🏛️",
        "objects" => "📿",
        "clothing" => "👘",
        "emotions" => "💭",
        "nature" => "🌸",
        _ => "🔸",
    }
}

const CHINESE_CLASSICAL_CSS: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🏮 古典文本实体可视化</title>
    <style>
body {
    font-family: "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB", "Noto Sans CJK SC", sans-serif;
    line-height: 2.0;
    margin: 0;
    padding: 20px;
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
    color: #333;
}

.chinese-container {
    max-width: 1200px;
    margin: 0 auto;
    background: rgba(255, 255, 255, 0.95);
    border-radius: 15px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
    overflow: hidden;
}

.chinese-header {
    background: linear-gradient(45deg, #8B4513, #DAA520);
    color: white;
    text-align: center;
    padding: 30px 20px;
}

.chinese-header h1 {
    margin: 0;
    font-size: 28px;
    text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.3);
}

.chinese-header p {
    margin: 10px 0 0 0;
    font-size: 16px;
    opacity: 0.9;
}

.chinese-content {
    padding: 30px;
}

.chinese-text-content {
    background: #FFFEF7;
    border: 2px solid #DAA520;
    border-radius: 12px;
    padding: 25px;
    margin: 20px 0;
    font-size: 18px;
    letter-spacing: 0.5px;
    line-height: 2.2;
    box-shadow: inset 0 2px 8px rgba(218, 165, 32, 0.1);
}

.chinese-legend {
    background: linear-gradient(135deg, #FFF8DC, #F5DEB3);
    border: 2px solid #CD853F;
    border-radius: 12px;
    padding: 20px;
    margin: 20px 0;
    box-shadow: 0 4px 12px rgba(205, 133, 63, 0.2);
}

.legend-title {
    color: #8B4513;
    font-weight: bold;
    font-size: 18px;
    margin-bottom: 15px;
    text-align: center;
    border-bottom: 2px solid #DAA520;
    padding-bottom: 10px;
}

.legend-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
}

.legend-item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.6);
    transition: all 0.3s ease;
}

.legend-item:hover {
    background: rgba(255, 255, 255, 0.9);
    transform: scale(1.02);
}

.legend-color {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    margin-right: 10px;
    border: 1px solid rgba(0, 0, 0, 0.2);
}

.highlight {
    padding: 3px 6px;
    border-radius: 6px;
    font-weight: 600;
    cursor: help;
    transition: all 0.3s ease;
    border: 1px solid rgba(0, 0, 0, 0.1);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.highlight:hover {
    transform: scale(1.05);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
    z-index: 10;
    position: relative;
}

.chinese-statistics {
    background: linear-gradient(135deg, #E6F3FF, #CCE7FF);
    border: 2px solid #4682B4;
    border-radius: 12px;
    padding: 20px;
    margin: 20px 0;
}

.chinese-statistics h3 {
    text-align: center;
    color: #2F4F4F;
    margin-bottom: 15px;
}

.stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 15px;
    margin-top: 15px;
}

.stat-item {
    text-align: center;
    padding: 15px;
    background: rgba(255, 255, 255, 0.7);
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    transition: all 0.3s ease;
}

.stat-item.clickable {
    cursor: pointer;
}

.stat-item.clickable:hover {
    background: rgba(255, 255, 255, 0.9);
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}

.stat-number {
    font-size: 24px;
    font-weight: bold;
    color: #2F4F4F;
}

.stat-label {
    font-size: 14px;
    color: #666;
    margin-top: 5px;
}

.chinese-decoration {
    text-align: center;
    color: #DAA520;
    font-size: 24px;
    margin: 20px 0;
    text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.1);
}

.chinese-footer {
    background: linear-gradient(45deg, #F5DEB3, #DDD);
    color: #8B4513;
    text-align: center;
    padding: 20px;
    font-weight: bold;
}

.chinese-footer p {
    margin: 5px 0;
}

/* Modal Styles */
.modal {
    display: none;
    position: fixed;
    z-index: 1000;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
}

.modal-content {
    background-color: #fefefe;
    margin: 5% auto;
    padding: 0;
    border-radius: 12px;
    width: 80%;
    max-width: 800px;
    max-height: 80%;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.modal-header {
    background: linear-gradient(45deg, #8B4513, #DAA520);
    color: white;
    padding: 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.modal-header h2 {
    margin: 0;
    font-size: 20px;
}

.close {
    font-size: 28px;
    font-weight: bold;
    cursor: pointer;
    opacity: 0.8;
}

.close:hover {
    opacity: 1;
}

.modal-body {
    padding: 20px;
    max-height: 60vh;
    overflow-y: auto;
}

.extraction-list {
    display: flex;
    flex-direction: column;
    gap: 15px;
}

.extraction-item {
    padding: 15px;
    border: 1px solid #ddd;
    border-radius: 8px;
    background: #f9f9f9;
    transition: all 0.3s ease;
}

.extraction-item:hover {
    background: #f0f0f0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.extraction-text {
    font-weight: bold;
    font-size: 16px;
    color: #333;
    margin-bottom: 8px;
}

.extraction-meta {
    display: flex;
    gap: 15px;
    font-size: 14px;
    color: #666;
}

.extraction-class {
    background: #e3f2fd;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 500;
}

.extraction-position {
    background: #f3e5f5;
    padding: 2px 8px;
    border-radius: 4px;
    font-family: monospace;
}

.extraction-attributes {
    margin-top: 8px;
    font-size: 13px;
    color: #555;
    font-style: italic;
}

@media (max-width: 768px) {
    body { padding: 10px; }
    .chinese-content { padding: 20px; }
    .chinese-text-content { padding: 15px; font-size: 16px; }
    .chinese-header h1 { font-size: 24px; }
    .legend-grid { grid-template-columns: 1fr; }
    .modal-content { width: 95%; margin: 10% auto; }
    .extraction-meta { flex-direction: column; gap: 5px; }
}
</style>
</head>
<body>
"#;

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

    #[test]
    fn test_visualization_style_default() {
        let options = VisualizeOptions::default();
        assert_eq!(options.style, VisualizationStyle::Animated);
    }

    #[test]
    fn test_visualization_style_selection() {
        let animated_options = VisualizeOptions {
            style: VisualizationStyle::Animated,
            ..Default::default()
        };
        assert_eq!(animated_options.style, VisualizationStyle::Animated);

        let chinese_options = VisualizeOptions {
            style: VisualizationStyle::ChineseClassical,
            ..Default::default()
        };
        assert_eq!(chinese_options.style, VisualizationStyle::ChineseClassical);
    }

    #[test]
    fn test_get_chinese_category_name() {
        assert_eq!(get_chinese_category_name("characters"), "人物角色");
        assert_eq!(get_chinese_category_name("locations"), "地点场所");
        assert_eq!(get_chinese_category_name("objects"), "物品器具");
        assert_eq!(get_chinese_category_name("clothing"), "服饰装扮");
        assert_eq!(get_chinese_category_name("emotions"), "情感状态");
        assert_eq!(get_chinese_category_name("nature"), "自然景物");
        assert_eq!(get_chinese_category_name("unknown"), "其他");
    }

    #[test]
    fn test_get_category_icon() {
        assert_eq!(get_category_icon("characters"), "👤");
        assert_eq!(get_category_icon("locations"), "🏛️");
        assert_eq!(get_category_icon("objects"), "📿");
        assert_eq!(get_category_icon("clothing"), "👘");
        assert_eq!(get_category_icon("emotions"), "💭");
        assert_eq!(get_category_icon("nature"), "🌸");
        assert_eq!(get_category_icon("unknown"), "🔸");
    }

    #[test]
    fn test_get_css_for_style() {
        assert_eq!(get_css_for_style(&VisualizationStyle::Animated), VISUALIZATION_CSS);
        assert_eq!(
            get_css_for_style(&VisualizationStyle::ChineseClassical),
            CHINESE_CLASSICAL_CSS
        );
    }

    #[test]
    fn test_empty_extractions_different_styles() -> Result<(), VisualizeError> {
        let doc = create_test_document();

        // Test animated style with empty extractions
        let animated_options = VisualizeOptions {
            style: VisualizationStyle::Animated,
            ..Default::default()
        };
        let animated_html = visualize(DataSource::Document(doc.clone()), animated_options)?;
        assert!(animated_html.contains("No valid extractions to animate"));

        // Test Chinese classical style with empty extractions
        let chinese_options = VisualizeOptions {
            style: VisualizationStyle::ChineseClassical,
            ..Default::default()
        };
        let chinese_html = visualize(DataSource::Document(doc), chinese_options)?;
        assert!(chinese_html.contains("没有可显示的提取结果"));

        Ok(())
    }
}
