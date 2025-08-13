//! Character extraction example using the LangExtract library.
//!
//! This example demonstrates how to extract characters, emotions, and relationships
//! from text using a DeepSeek language model.

use std::collections::HashMap;

use langextract::{
    annotation::Annotator,
    data::{AttributeValue, Document, FormatType},
    inference::DeepSeekLanguageModel,
    prompting::{ExampleData, Extraction as PromptExtraction, PromptTemplateStructured},
    resolver::Resolver,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define the prompt and extraction rules
    let prompt_description = r#"
Extract characters, emotions, and relationships in order of appearance.
Use exact text for extractions. Do not paraphrase or overlap entities.
Provide meaningful attributes for each entity to add context.
    "#
    .trim();

    // 2. Provide a high-quality example to guide the model
    let mut romeo_attributes = HashMap::new();
    romeo_attributes.insert(
        "emotional_state".to_string(),
        serde_json::Value::String("wonder".to_string()),
    );

    let mut emotion_attributes = HashMap::new();
    emotion_attributes.insert(
        "feeling".to_string(),
        serde_json::Value::String("gentle awe".to_string()),
    );

    let mut relationship_attributes = HashMap::new();
    relationship_attributes.insert("type".to_string(), serde_json::Value::String("metaphor".to_string()));

    let examples = vec![ExampleData {
        text: "ROMEO. But soft! What light through yonder window breaks? It is the east, and Juliet is the sun."
            .to_string(),
        extractions: vec![
            PromptExtraction {
                extraction_class: "character".to_string(),
                extraction_text: "ROMEO".to_string(),
                attributes: Some(romeo_attributes),
            },
            PromptExtraction {
                extraction_class: "emotion".to_string(),
                extraction_text: "But soft!".to_string(),
                attributes: Some(emotion_attributes),
            },
            PromptExtraction {
                extraction_class: "relationship".to_string(),
                extraction_text: "Juliet is the sun".to_string(),
                attributes: Some(relationship_attributes),
            },
        ],
    }];

    // Create the prompt template
    let prompt_template = PromptTemplateStructured {
        description: prompt_description.to_string(),
        examples,
    };

    // The input text to be processed
    let input_text = "Lady Juliet gazed longingly at the stars, her heart aching for Romeo";

    // 3. Initialize the DeepSeek language model
    // Make sure to set your DEEPSEEK_API_KEY environment variable
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY environment variable not set");

    let language_model = DeepSeekLanguageModel::new(
        Some("deepseek-chat".to_string()), // model_id
        api_key,                           // api_key
        None,                              // base_url (use default)
        Some(FormatType::Yaml),            // format_type
        Some(0.1),                         // temperature
        Some(1),                           // max_workers
        None,                              // extra_kwargs
    )?;

    // 4. Create the annotator
    let annotator = Annotator::new(
        language_model,
        prompt_template,
        FormatType::Yaml,
        Some("_attributes"),
        true, // fence_output
    );

    // 5. Create a resolver
    let resolver = Resolver::new(true, None, None, false);

    // 6. Create a document from the input text
    // 7. Run the extraction
    println!("Running extraction on: {}", input_text);
    println!("Using DeepSeek model...");

    let document = Document::new(input_text.to_string(), Some("example_doc".to_string()), None);

    let results = annotator
        .annotate_documents(
            vec![document],
            &resolver,
            2000, // max_char_buffer
            1,    // batch_length
            true, // debug
            1,    // extraction_passes
            None, // extra_args
        )
        .await?;

    let result = &results[0];

    // 8. Display results
    println!("\n=== Extraction Results ===");
    if let Some(extractions) = &result.extractions {
        for (i, extraction) in extractions.iter().enumerate() {
            println!("{}. Class: {}", i + 1, extraction.extraction_class);
            println!("   Text: {}", extraction.extraction_text);
            if let Some(attributes) = &extraction.attributes {
                println!("   Attributes:");
                for (key, value) in attributes {
                    match value {
                        AttributeValue::Single(s) => println!("     {}: {}", key, s),
                        AttributeValue::Multiple(v) => println!("     {}: {:?}", key, v),
                    }
                }
            }
            if let Some(char_interval) = &extraction.char_interval {
                println!(
                    "   Position: {:?} - {:?}",
                    char_interval.start_pos, char_interval.end_pos
                );
            }
            println!();
        }
    } else {
        println!("No extractions found.");
    }

    // 9. Save results (simplified for this example)
    println!("\n=== Summary ===");
    println!("✓ Extraction completed successfully");
    println!(
        "✓ Found {} extractions",
        result.extractions.as_ref().map_or(0, |e| e.len())
    );

    // Note: File I/O and visualization features would be implemented here
    // in a full production version of the library

    Ok(())
}
