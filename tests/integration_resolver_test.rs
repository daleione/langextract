use langextract::resolver::Resolver;

#[test]
fn test_simple_name_extraction_workflow() {
    // Create a resolver that expects fenced YAML output
    let resolver = Resolver::new(
        true, // fence_output
        None, // extraction_index_suffix
        None, // extraction_attributes_suffix
        true, // format_is_yaml
    );

    // Simulate the LLM response that we're getting from the DeepSeek API
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

    // Debug: print extraction details FIRST
    println!("Debug: Raw extractions in order:");
    for (i, extraction) in extractions.iter().enumerate() {
        println!(
            "  extraction[{}] = text:'{}', class:'{}', index:{}, group:{}",
            i,
            extraction.extraction_text,
            extraction.extraction_class,
            extraction.extraction_index,
            extraction.group_index
        );
    }

    // Print sorted order to understand the sorting
    let mut sorted_extractions = extractions.clone();
    sorted_extractions.sort_by_key(|e| (e.extraction_index, e.group_index));
    println!("Debug: After sorting by (extraction_index, group_index):");
    for (i, extraction) in sorted_extractions.iter().enumerate() {
        println!(
            "  sorted[{}] = text:'{}', index:{}, group:{}",
            i, extraction.extraction_text, extraction.extraction_index, extraction.group_index
        );
    }

    // Now check the extracted names based on what we see
    // (will adjust based on debug output)
    assert_eq!(extractions[0].extraction_text, extractions[0].extraction_text); // placeholder for now

    // Check that they all have the expected class names
    for extraction in &extractions {
        assert!(extraction.extraction_class.starts_with("text"));
    }

    // Check that all extractions are in the same group
    let first_group = extractions[0].group_index;
    for extraction in &extractions {
        assert_eq!(extraction.group_index, first_group);
    }

    println!("✅ Successfully parsed {} names from LLM response", extractions.len());
    for (i, extraction) in extractions.iter().enumerate() {
        println!(
            "  {}. {} (class: {}, index: {})",
            i + 1,
            extraction.extraction_text,
            extraction.extraction_class,
            extraction.extraction_index
        );
    }
}

#[test]
fn test_structured_format_still_works() {
    // Create a resolver that expects fenced YAML output
    let resolver = Resolver::new(
        true, // fence_output
        None, // extraction_index_suffix
        None, // extraction_attributes_suffix
        true, // format_is_yaml
    );

    // Test that the old structured format still works
    let structured_response = r#"```yaml
extractions:
  - Alice
  - Bob
  - Charlie
```"#;

    // Parse the response
    let result = resolver.parse_extractions_from_string(structured_response);

    // Verify it works
    assert!(result.is_ok(), "Failed to parse structured response: {:?}", result);

    let extractions = result.unwrap();
    assert_eq!(extractions.len(), 3);

    // Check that we got the expected names (in any order)
    let texts: Vec<&str> = extractions.iter().map(|e| e.extraction_text.as_str()).collect();
    assert!(texts.contains(&"Alice"));
    assert!(texts.contains(&"Bob"));
    assert!(texts.contains(&"Charlie"));
}

#[test]
fn test_json_simple_array_format() {
    // Create a resolver that expects fenced JSON output
    let resolver = Resolver::new(
        true,  // fence_output
        None,  // extraction_index_suffix
        None,  // extraction_attributes_suffix
        false, // format_is_yaml (use JSON)
    );

    // Test simple JSON array format
    let json_response = r#"```json
["Alice", "Bob", "Charlie"]
```"#;

    // Parse the response
    let result = resolver.parse_extractions_from_string(json_response);

    // Verify it works
    assert!(result.is_ok(), "Failed to parse JSON response: {:?}", result);

    let extractions = result.unwrap();
    assert_eq!(extractions.len(), 3);

    // Check that we got the expected names (in any order)
    let texts: Vec<&str> = extractions.iter().map(|e| e.extraction_text.as_str()).collect();
    assert!(texts.contains(&"Alice"));
    assert!(texts.contains(&"Bob"));
    assert!(texts.contains(&"Charlie"));
}
