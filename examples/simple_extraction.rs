//! Simple character extraction example using the LangExtract library.
//!
//! This example demonstrates a basic synchronous extraction workflow
//! using DeepSeek model.
//!
//! To run this example:
//! 1. Set your DEEPSEEK_API_KEY environment variable
//! 2. Run: cargo run --example simple_extraction

use langextract::{
    annotation::Annotator, data::FormatType, inference::DeepSeekLanguageModel, prompting::PromptTemplateStructured,
    resolver::Resolver,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define the prompt description
    let prompt_description = r#"
Extract important entities from the text including:
- Characters (people mentioned)
- Emotions (feelings expressed)
- Locations (places mentioned)

Extract the exact text as it appears. Provide one attribute per extraction.
    "#
    .trim();

    // 2. Create a simple prompt template (no examples for simplicity)
    let prompt_template = PromptTemplateStructured {
        description: prompt_description.to_string(),
        examples: vec![], // Empty examples for simplicity
    };

    // 3. Input text to process
    let input_text = "Romeo loved Juliet deeply. They met in Verona, feeling joy and sadness.";

    // 4. Get API key from environment
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("Please set DEEPSEEK_API_KEY environment variable");

    // 5. Initialize DeepSeek model
    let language_model = DeepSeekLanguageModel::new(
        None, // Use default model
        api_key,
        None, // Use default base URL
        Some(FormatType::Yaml),
        Some(0.1), // Low temperature for consistent results
        Some(1),   // Single worker
        None,      // No extra kwargs
    )?;

    // 6. Create annotator
    let annotator = Annotator::new(
        language_model,
        prompt_template,
        FormatType::Yaml,
        None, // Use default attribute suffix
        true, // Use fenced output
    );

    // 7. Create resolver
    let resolver = Resolver::new(true, None, None, false);

    // 8. Run extraction
    println!("Input text: {}", input_text);
    println!("Processing with DeepSeek...\n");

    // Note: This is a simplified synchronous wrapper
    // In a real async environment, you'd use .await
    let document = langextract::data::Document::new(input_text.to_string(), Some("simple_example".to_string()), None);

    let result = annotator.annotate_documents(
        vec![document],
        &resolver,
        1000, // max_char_buffer
        1,    // batch_length
        true, // debug
        1,    // extraction_passes
        None, // extra_args
    )?;

    let result = &result[0];

    // 9. Display results
    println!("=== Extraction Results ===");

    if let Some(extractions) = &result.extractions {
        if extractions.is_empty() {
            println!("No extractions found.");
        } else {
            for (i, extraction) in extractions.iter().enumerate() {
                println!(
                    "{}. [{}] \"{}\"",
                    i + 1,
                    extraction.extraction_class,
                    extraction.extraction_text
                );

                if let Some(attributes) = &extraction.attributes {
                    for (key, value) in attributes {
                        match value {
                            langextract::data::AttributeValue::Single(s) => {
                                println!("   {}: {}", key, s);
                            }
                            langextract::data::AttributeValue::Multiple(v) => {
                                println!("   {}: {:?}", key, v);
                            }
                        }
                    }
                }
                println!();
            }
        }
    } else {
        println!("No extractions found.");
    }

    println!("=== Done ===");
    Ok(())
}
