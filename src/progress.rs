use std::fmt::Write;
use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle, ProgressIterator, ProgressDrawTarget};
use url::Url;

// ANSI color codes
const BLUE: &str = "\x1b[94m";
const GREEN: &str = "\x1b[92m";
const CYAN: &str = "\x1b[96m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const GOOGLE_BLUE: &str = "#4285F4";

/// Creates a download progress bar
///
/// * `total_size` - Total bytes to download
/// * `url` - Download URL
/// * `ncols` - Progress bar width (default: 100)
/// * `max_url_length` - Maximum URL display length (default: 50)
pub fn create_download_progress_bar(
    total_size: u64,
    url: &str,
    ncols: Option<usize>,
    max_url_length: usize,
) -> ProgressBar {
    let url_display = truncate_url(url, max_url_length);
    let prefix = format!(
        "{}{}LangExtract{}: Downloading {}{}{}",
        BLUE, BOLD, RESET, GREEN, url_display, RESET
    );

    let pb = ProgressBar::new(total_size);
    let width = ncols.unwrap_or(100);

    pb.set_style(
        ProgressStyle::with_template(&format!(
            "{{prefix}}: {{percent:>3}}% |{{bar:{}}}| {{bytes}}/{{total_bytes}} [{{elapsed}}<{{remaining}}, {{bytes_per_sec}}]",
            width
        ))
        .unwrap()
        .progress_chars("=>- "),
    );
    pb.set_prefix(prefix);
    pb
}

/// Creates an extraction progress bar
///
/// * `iterable` - Iterator to wrap
/// * `model_info` - Model information (optional)
/// * `disable` - Whether to disable progress bar
pub fn create_extraction_progress_bar<I: Iterator>(
    iterable: I,
    model_info: Option<&str>,
    disable: bool,
) -> impl Iterator<Item = I::Item> {
    let desc = format_extraction_progress(model_info, None, None);
    let pb = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::with_template("{prefix} [{elapsed}]")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        )
        .with_prefix(desc);

    pb.enable_steady_tick(Duration::from_millis(100));
    if disable {
        pb.set_draw_target(ProgressDrawTarget::hidden());
    }
    iterable.progress_with(pb)
}

/// Prints download completion message
///
/// * `char_count` - Character count
/// * `word_count` - Word count
/// * `filename` - Source filename
pub fn print_download_complete(char_count: usize, word_count: usize, filename: &str) {
    println!(
        "{}✓{} Downloaded {}{}{} characters ({}{}{} words) from {}{}{}",
        GREEN, RESET,
        BOLD, char_count, RESET,
        BOLD, word_count, RESET,
        BLUE, filename, RESET
    );
}

/// Prints extraction completion message
pub fn print_extraction_complete() {
    println!("{}✓{} Extraction processing complete", GREEN, RESET);
}

/// Prints extraction summary statistics
///
/// * `num_extractions` - Number of extracted entities
/// * `unique_classes` - Number of unique classes
/// * `elapsed_time` - Processing time in seconds (optional)
/// * `chars_processed` - Characters processed (optional)
/// * `num_chunks` - Number of chunks processed (optional)
pub fn print_extraction_summary(
    num_extractions: usize,
    unique_classes: usize,
    elapsed_time: Option<f64>,
    chars_processed: Option<usize>,
    num_chunks: Option<usize>,
) {
    println!(
        "{}✓{} Extracted {}{}{} entities ({}{}{} unique types)",
        GREEN, RESET,
        BOLD, num_extractions, RESET,
        BOLD, unique_classes, RESET
    );

    if let (Some(elapsed), Some(chars)) = (elapsed_time, chars_processed) {
        let mut metrics = vec![format!("Time: {}{:.2}s{}", BOLD, elapsed, RESET)];

        if elapsed > 0.0 {
            let speed = chars as f64 / elapsed;
            metrics.push(format!("Speed: {}{:.0}{} chars/sec", BOLD, speed, RESET));
        }

        if let Some(chunks) = num_chunks {
            metrics.push(format!("Chunks: {}{}{}", BOLD, chunks, RESET));
        }

        for metric in metrics {
            println!("  {}•{} {}", CYAN, RESET, metric);
        }
    }
}

/// Creates save progress bar
///
/// * `output_path` - Output file path
/// * `disable` - Whether to disable progress bar
pub fn create_save_progress_bar(output_path: &str, disable: bool) -> ProgressBar {
    let filename = output_path.split('/').last().unwrap_or("unknown");
    let pb = ProgressBar::new_spinner()
        .with_prefix(format!(
            "{}{}LangExtract{}: Saving to {}{}{}",
            BLUE, BOLD, RESET, GREEN, filename, RESET
        ));

    pb.enable_steady_tick(Duration::from_millis(100));
    if disable {
        pb.set_draw_target(ProgressDrawTarget::hidden());
    }
    pb
}

/// Creates load progress bar
///
/// * `file_path` - File path to load
/// * `total_size` - Total file size in bytes (optional)
/// * `disable` - Whether to disable progress bar
pub fn create_load_progress_bar(
    file_path: &str,
    total_size: Option<u64>,
    disable: bool,
) -> ProgressBar {
    let filename = file_path.split('/').last().unwrap_or("unknown");
    let pb = if let Some(size) = total_size {
        ProgressBar::new(size).with_style(
            ProgressStyle::with_template(&format!(
                "{{prefix}}: [{{bar:{}.40}}] {{bytes}}/{{total_bytes}}",
                GOOGLE_BLUE
            ))
            .unwrap()
        )
    } else {
        ProgressBar::new_spinner()
    };

    pb.set_prefix(format!(
        "{}{}LangExtract{}: Loading {}{}{}",
        BLUE, BOLD, RESET, GREEN, filename, RESET
    ));

    if disable {
        pb.set_draw_target(ProgressDrawTarget::hidden());
    }
    pb
}

/// Formats extraction progress description
///
/// * `model_info` - Model information (optional)
/// * `current_chars` - Current character count (optional)
/// * `processed_chars` - Processed character count (optional)
pub fn format_extraction_progress(
    model_info: Option<&str>,
    current_chars: Option<usize>,
    processed_chars: Option<usize>,
) -> String {
    let mut desc = if let Some(model) = model_info {
        format!(
            "{}{}LangExtract{}: model={}{}{}",
            BLUE, BOLD, RESET, GREEN, model, RESET
        )
    } else {
        format!(
            "{}{}LangExtract{}: Processing",
            BLUE, BOLD, RESET
        )
    };

    if let (Some(current), Some(processed)) = (current_chars, processed_chars) {
        write!(
            &mut desc,
            ", current={}{}{} chars, processed={}{}{} chars",
            GREEN, current, RESET,
            GREEN, processed, RESET
        ).unwrap();
    }

    desc
}

// --- Helper functions ---

/// Truncates URL for display purposes
fn truncate_url(url: &str, max_length: usize) -> String {
    if url.len() <= max_length {
        return url.to_string();
    }

    let parsed = Url::parse(url).ok();
    let (domain, filename) = parsed.map_or_else(
        || (String::from("unknown"), String::from("file")),
        |url| {
            let domain = url.host_str().map_or("unknown", |h| h).to_string();
            let filename = url.path_segments()
                .and_then(|s| s.last())
                .unwrap_or("file")
                .to_string();
            (domain, filename)
        }
    );

    let candidate = format!("{}/.../{}", domain, filename);
    if candidate.len() <= max_length {
        candidate
    } else if max_length > 3 {
        format!("{}...", &url[..max_length - 3])
    } else {
        "...".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_truncation() {
        let long_url = "https://example.com/very/long/path/to/a/specific/file.txt";
        assert_eq!(truncate_url(long_url, 30), "example.com/.../file.txt");
        assert_eq!(truncate_url("short", 10), "short");
        assert_eq!(truncate_url(long_url, 5), "ht...");
    }

    #[test]
    fn test_format_extraction_progress() {
        assert_eq!(
            format_extraction_progress(Some("model-1"), None, None),
            format!("\x1b[94m\x1b[1mLangExtract\x1b[0m: model=\x1b[92mmodel-1\x1b[0m")
        );

        assert_eq!(
            format_extraction_progress(None, Some(100), Some(1000)),
            format!("\x1b[94m\x1b[1mLangExtract\x1b[0m: Processing, current=\x1b[92m100\x1b[0m chars, processed=\x1b[92m1000\x1b[0m chars")
        );
    }

    #[test]
    fn test_progress_bar_creation() {
        let pb = create_download_progress_bar(1024, "http://example.com/file", Some(80), 50);
        assert_eq!(pb.length(), Some(1024));

        let iter = vec![1, 2, 3].into_iter();
        let wrapped = create_extraction_progress_bar(iter, Some("model"), false);
        assert_eq!(wrapped.collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn test_extraction_summary() {
        // Test without performance metrics
        print_extraction_summary(150, 20, None, None, None);

        // Test with performance metrics
        print_extraction_summary(150, 20, Some(5.0), Some(5000), Some(10));
    }
}
