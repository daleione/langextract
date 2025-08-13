//! Simple test to verify the resolver fix works with the expected LLM output format
//! This simulates what happens in the getting_started example but without making API calls.

use langextract::resolver::Resolver;

fn main() {
    println!("🔧 Testing Resolver Fix");
    println!("=======================\n");

    // Create a resolver that matches the getting_started example configuration
    let resolver = Resolver::new(
        true, // fence_output - expects ```yaml``` fenced blocks
        None, // extraction_index_suffix
        None, // extraction_attributes_suffix
        true, // format_is_yaml
    );

    // This is the actual format returned by DeepSeek when asked to extract names
    let llm_response = r#"```yaml
- Alice
- Bob
- Charlie
```"#;

    println!("📝 LLM Response:");
    println!("{}", llm_response);
    println!();

    // Parse the response using the resolver
    match resolver.parse_extractions_from_string(llm_response) {
        Ok(extractions) => {
            println!("✅ Successfully parsed {} extractions!", extractions.len());
            println!();

            for (i, extraction) in extractions.iter().enumerate() {
                println!(
                    "{}. \"{}\" (class: {}, index: {}, group: {})",
                    i + 1,
                    extraction.extraction_text,
                    extraction.extraction_class,
                    extraction.extraction_index,
                    extraction.group_index
                );
            }

            println!();
            println!("🎉 The resolver fix works! The getting_started example should now show extractions.");
        }
        Err(e) => {
            println!("❌ Failed to parse LLM response: {:?}", e);
            std::process::exit(1);
        }
    }

    // Test the old structured format still works
    println!("\n🔄 Testing backward compatibility...");

    let structured_response = r#"```yaml
extractions:
  - Alice
  - Bob
  - Charlie
```"#;

    match resolver.parse_extractions_from_string(structured_response) {
        Ok(extractions) => {
            println!("✅ Structured format still works ({} extractions)", extractions.len());
        }
        Err(e) => {
            println!("❌ Structured format broken: {:?}", e);
            std::process::exit(1);
        }
    }

    // Test JSON format too
    println!("\n🔄 Testing JSON format...");

    let json_resolver = Resolver::new(
        true,  // fence_output
        None,  // extraction_index_suffix
        None,  // extraction_attributes_suffix
        false, // format_is_yaml = false (JSON)
    );

    let json_response = r#"```json
["Alice", "Bob", "Charlie"]
```"#;

    match json_resolver.parse_extractions_from_string(json_response) {
        Ok(extractions) => {
            println!("✅ JSON format works ({} extractions)", extractions.len());
        }
        Err(e) => {
            println!("❌ JSON format broken: {:?}", e);
            std::process::exit(1);
        }
    }

    println!("\n🎯 All tests passed! The resolver can now handle:");
    println!("   • Simple YAML arrays: [\"Alice\", \"Bob\", \"Charlie\"]");
    println!("   • Simple JSON arrays: [\"Alice\", \"Bob\", \"Charlie\"]");
    println!("   • Structured format: {{\"extractions\": [...]}}");
    println!("\n💡 This means the getting_started example should now work correctly!");
}
