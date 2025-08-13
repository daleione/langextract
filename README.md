# LangExtract

A Rust library for extracting structured data from unstructured text using Large Language Models (LLMs), featuring precise source attribution and interactive visualization capabilities.

## Features

- 🚀 **Multiple LLM Support**: DeepSeek, OpenAI, and Ollama models
- 📝 **Structured Extraction**: Extract entities with attributes and relationships
- 🎯 **Precise Attribution**: Track exact source positions for every extraction
- 🔄 **Flexible Formats**: Support for YAML and JSON output formats
- 📊 **Interactive Visualization**: Generate HTML visualizations of results
- ⚡ **Async/Await**: Built with modern Rust async patterns
- 🛡️ **Type Safety**: Leverage Rust's type system for reliable extractions

## Quick Start

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
langextract = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Basic Usage

```rust
use langextract::{
    annotation::Annotator,
    data::{Document, FormatType},
    inference::DeepSeekLanguageModel,
    prompting::PromptTemplateStructured,
    resolver::Resolver,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up your API key
    let api_key = std::env::var("DEEPSEEK_API_KEY")?;

    // 2. Create a prompt template
    let prompt = PromptTemplateStructured {
        description: "Extract names of people mentioned in the text.".to_string(),
        examples: vec![],
    };

    // 3. Initialize the model
    let model = DeepSeekLanguageModel::new(
        None, api_key, None, Some(FormatType::Yaml), Some(0.1), Some(1), None
    )?;

    // 4. Create annotator and resolver
    let annotator = Annotator::new(model, prompt, FormatType::Yaml, None, true);
    let resolver = Resolver::new(true, None, None, false);

    // 5. Process your text
    let text = "Alice met Bob at the coffee shop. Charlie joined them later.";
    let document = Document::new(text.to_string(), Some("example".to_string()), None);

    let results = annotator.annotate_documents(
        vec![document], &resolver, 1000, 1, true, 1, None
    )?;

    // 6. Use the results
    if let Some(extractions) = &results[0].extractions {
        for extraction in extractions {
            println!("Found: {}", extraction.extraction_text);
        }
    }

    Ok(())
}
```

## Examples

We provide several examples to help you get started:

### 🌟 Getting Started

The simplest possible example - perfect for beginners!

```bash
export DEEPSEEK_API_KEY="your-api-key-here"
cargo run --example getting_started
```

### 📝 Simple Extraction

Basic entity extraction with minimal setup:

```bash
cargo run --example simple_extraction
```

### 🎭 Character Extraction

Advanced example with detailed prompts and attributes:

```bash
cargo run --example character_extraction
```

See the [`examples/`](examples/) directory for complete code and detailed documentation.

## Supported Models

### DeepSeek

```rust
let model = DeepSeekLanguageModel::new(
    Some("deepseek-chat".to_string()),
    api_key,
    None, // Use default base URL
    Some(FormatType::Yaml),
    Some(0.1), // Temperature
    Some(1),   // Max workers
    None,      // Extra kwargs
)?;
```

### OpenAI

```rust
let model = OpenAILanguageModel::new(
    Some("gpt-4".to_string()),
    api_key,
    None, // Use default base URL
    None, // Organization
    Some(FormatType::Json),
    Some(0.1),
    Some(1),
    None,
)?;
```

### Ollama

```rust
let model = OllamaLanguageModel::new(
    "llama2:latest",
    Some("http://localhost:11434".to_string()),
    Some("json".to_string()),
    None,
    None,
);
```

## API Documentation

### Core Components

- **`Annotator`**: Main interface for text processing
- **`Document`**: Represents input text with metadata
- **`Extraction`**: Structured output with source attribution
- **`Resolver`**: Converts LLM output to structured data
- **`PromptTemplateStructured`**: Template for few-shot learning

### Output Formats

LangExtract supports both YAML and JSON output formats:

```rust
// YAML format (default)
let annotator = Annotator::new(model, prompt, FormatType::Yaml, None, true);

// JSON format
let annotator = Annotator::new(model, prompt, FormatType::Json, None, true);
```

## Environment Variables

| Variable           | Description      | Required            |
| ------------------ | ---------------- | ------------------- |
| `DEEPSEEK_API_KEY` | DeepSeek API key | For DeepSeek models |
| `OPENAI_API_KEY`   | OpenAI API key   | For OpenAI models   |

## Error Handling

LangExtract uses custom error types for clear error reporting:

```rust
use langextract::inference::InferenceOutputError;

match annotator.annotate_documents(documents, &resolver, 1000, 1, true, 1, None) {
    Ok(results) => println!("Success: {} documents processed", results.len()),
    Err(InferenceOutputError { message }) => eprintln!("Error: {}", message),
}
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Inspired by the Python langextract library, this Rust implementation brings type safety and performance to structured text extraction.
