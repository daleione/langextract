//! Getting Started with LangExtract
//!
//! This is the simplest possible example to get you started with LangExtract.
//! It demonstrates basic text extraction using the DeepSeek model.
//!
//! To run this example:
//! 1. Set your DEEPSEEK_API_KEY environment variable:
//!    export DEEPSEEK_API_KEY="your-api-key-here"
//! 2. Run: cargo run --example getting_started

use langextract::{
    annotation::Annotator,
    data::{Document, FormatType},
    inference::DeepSeekLanguageModel,
    prompting::PromptTemplateStructured,
    resolver::Resolver,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 LangExtract Getting Started Example");
    println!("======================================\n");

    // Step 1: Get your API key
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("❌ Please set the DEEPSEEK_API_KEY environment variable");

    println!("✅ API key loaded");

    // Step 2: Create a simple prompt
    let prompt = PromptTemplateStructured {
        description: "Extract names of people mentioned in the text.".to_string(),
        examples: vec![], // No examples needed for this simple task
    };

    println!("✅ Prompt template created");

    // Step 3: Set up the DeepSeek model
    let model = DeepSeekLanguageModel::new(
        None,                   // Use default model
        api_key,                // Your API key
        None,                   // Use default URL
        Some(FormatType::Yaml), // Output format
        Some(0.1),              // Low temperature for consistent results
        Some(1),                // Single worker
        None,                   // No extra parameters
    )?;

    println!("✅ DeepSeek model initialized");

    // Step 4: Create the annotator
    let annotator = Annotator::new(
        model,
        prompt,
        FormatType::Yaml,
        None, // Use default attribute suffix
        true, // Use fenced output
    );

    println!("✅ Annotator created");

    // Step 5: Create a resolver
    let resolver = Resolver::new(true, None, None, false);

    println!("✅ Resolver created");

    // Step 6: Your input text
    let text = "Alice met Bob at the coffee shop. Charlie joined them later for lunch.";

    println!("📝 Input text: {}", text);
    println!("\n🔄 Processing...");

    // Step 7: Run the extraction
    let document = Document::new(text.to_string(), Some("getting_started".to_string()), None);

    let results = annotator.annotate_documents(
        vec![document],
        &resolver,
        1000, // max characters per chunk
        1,    // batch size
        true, // enable debug output
        1,    // number of extraction passes
        None, // no extra arguments
    )?;

    // Step 8: Display the results
    println!("\n🎉 Extraction Results:");
    println!("======================");

    if let Some(extractions) = &results[0].extractions {
        if extractions.is_empty() {
            println!("😔 No extractions found. Try adjusting your prompt.");
        } else {
            for (i, extraction) in extractions.iter().enumerate() {
                println!("{}. 👤 {}", i + 1, extraction.extraction_text);
                if let Some(attributes) = &extraction.attributes {
                    for (key, value) in attributes {
                        match value {
                            langextract::data::AttributeValue::Single(v) => {
                                println!("   📋 {}: {}", key, v);
                            }
                            langextract::data::AttributeValue::Multiple(v) => {
                                println!("   📋 {}: {:?}", key, v);
                            }
                        }
                    }
                }
            }
        }
    } else {
        println!("😔 No extractions found.");
    }

    println!("\n✨ Done! You've successfully run your first LangExtract example.");
    println!("💡 Try modifying the prompt or input text to see different results.");

    Ok(())
}
