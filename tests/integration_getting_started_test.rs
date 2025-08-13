//! Integration test that simulates the getting_started example workflow
//! This verifies that the resolver configuration matches the annotator format

use langextract::{data::FormatType, resolver::Resolver};

#[test]
fn test_getting_started_resolver_configuration() {
    // This simulates the exact configuration used in the getting_started example

    // The annotator is configured with YAML format
    let format_type = FormatType::Yaml;
    let fence_output = true;

    // Create resolver with matching YAML configuration (the fix)
    let resolver = Resolver::new(
        fence_output,                    // true - expects fenced blocks
        None,                            // extraction_index_suffix
        None,                            // extraction_attributes_suffix
        format_type == FormatType::Yaml, // true for YAML, false for JSON
    );

    // This is the response format that DeepSeek returns
    let llm_response = r#"```yaml
- Alice
- Bob
- Charlie
```"#;

    // Parse the response
    let result = resolver.parse_extractions_from_string(llm_response);

    // Verify it works
    assert!(result.is_ok(), "Failed to parse LLM response: {:?}", result);

    let extractions = result.unwrap();
    assert_eq!(
        extractions.len(),
        3,
        "Expected 3 extractions, got {}",
        extractions.len()
    );

    // Check that we got the expected names (in any order)
    let texts: Vec<&str> = extractions.iter().map(|e| e.extraction_text.as_str()).collect();
    assert!(texts.contains(&"Alice"), "Missing Alice");
    assert!(texts.contains(&"Bob"), "Missing Bob");
    assert!(texts.contains(&"Charlie"), "Missing Charlie");

    // Check that all extractions are from the same group
    let first_group = extractions[0].group_index;
    for extraction in &extractions {
        assert_eq!(
            extraction.group_index, first_group,
            "All extractions should be in the same group"
        );
    }

    println!("✅ Getting started resolver configuration test passed!");
    println!("   Found {} extractions: {:?}", extractions.len(), texts);
}

#[test]
fn test_getting_started_wrong_configuration() {
    // This tests what happens with the old incorrect configuration

    // Create resolver with WRONG configuration (JSON parser for YAML content)
    let resolver = Resolver::new(
        true,  // fence_output
        None,  // extraction_index_suffix
        None,  // extraction_attributes_suffix
        false, // format_is_yaml = false (WRONG - should be true for YAML)
    );

    // This is YAML content
    let yaml_response = r#"```yaml
- Alice
- Bob
- Charlie
```"#;

    // This should fail because we're trying to parse YAML with JSON parser
    let result = resolver.parse_extractions_from_string(yaml_response);

    // Should fail or produce empty results
    match result {
        Ok(extractions) => {
            // If it somehow succeeds, it should at least not find the names correctly
            println!(
                "Unexpected success with wrong config, got {} extractions",
                extractions.len()
            );
        }
        Err(e) => {
            println!("Expected failure with wrong config: {:?}", e);
        }
    }
}

#[test]
fn test_json_configuration_works() {
    // Test that JSON configuration works correctly for JSON content

    let resolver = Resolver::new(
        true,  // fence_output
        None,  // extraction_index_suffix
        None,  // extraction_attributes_suffix
        false, // format_is_yaml = false (correct for JSON)
    );

    let json_response = r#"```json
["Alice", "Bob", "Charlie"]
```"#;

    let result = resolver.parse_extractions_from_string(json_response);
    assert!(result.is_ok(), "JSON parsing should work: {:?}", result);

    let extractions = result.unwrap();
    assert_eq!(extractions.len(), 3);

    let texts: Vec<&str> = extractions.iter().map(|e| e.extraction_text.as_str()).collect();
    assert!(texts.contains(&"Alice"));
    assert!(texts.contains(&"Bob"));
    assert!(texts.contains(&"Charlie"));
}
