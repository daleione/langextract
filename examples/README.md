# LangExtract Examples

This directory contains example applications demonstrating how to use the LangExtract library for text extraction tasks.

## Prerequisites

1. **API Key**: You need a DeepSeek API key to run these examples. Get one from [DeepSeek](https://platform.deepseek.com/).

2. **Environment Variable**: Set your API key as an environment variable:

   ```bash
   export DEEPSEEK_API_KEY="your-api-key-here"
   ```

3. **Dependencies**: Make sure you have the required dependencies by adding this to your `Cargo.toml`:
   ```toml
   [dependencies]
   tokio = { version = "1.0", features = ["full"] }
   langextract = { path = "." }
   ```

## Available Examples

### 1. Getting Started (`getting_started.rs`)

The simplest possible example to get you started with LangExtract. Perfect for beginners!

**Run it:**

```bash
cargo run --example getting_started
```

**What it does:**

- Demonstrates the basic workflow from setup to results
- Extracts names of people from a simple text
- Uses minimal configuration with clear step-by-step output
- Great for understanding the core concepts

### 2. Simple Extraction (`simple_extraction.rs`)

A basic example that demonstrates:

- Setting up a DeepSeek language model
- Creating a simple prompt template
- Extracting entities from text
- Displaying results

**Run it:**

```bash
cargo run --example simple_extraction
```

**What it does:**

- Extracts characters, emotions, and locations from a Romeo and Juliet text
- Uses YAML format for structured output
- Shows basic entity extraction without complex examples

### 3. Chinese Classical Extraction (`chinese_classical_extraction.rs`)

A specialized example for Chinese text processing inspired by Dream of the Red Chamber (红楼梦) style:

- Demonstrates multi-lingual entity extraction
- Handles classical Chinese literary language
- Extracts characters, locations, objects, emotions, and clothing descriptions
- Uses original content written in classical Chinese style

**Run it:**

```bash
cargo run --example chinese_classical_extraction
```

**What it does:**

- Processes Chinese text with classical literary style
- Extracts 人物姓名 (character names), 地点名称 (locations), 物品器具 (objects), 情感状态 (emotions), and 服饰装扮 (clothing)
- Groups results by category with Chinese labels
- Demonstrates LangExtract's capability with non-English content

### 4. Character Extraction (`character_extraction.rs`)

A comprehensive example that demonstrates:

- Advanced prompt engineering with examples
- Detailed attribute extraction
- File I/O operations
- HTML visualization generation

**Run it:**

```bash
cargo run --example character_extraction
```

**What it does:**

- Extracts characters, emotions, and relationships from literary text
- Uses few-shot learning with detailed examples
- Saves results to JSONL format
- Generates an HTML visualization
- Demonstrates the complete workflow from text to visualization

## Example Output

When you run the examples, you'll see output like:

**Getting Started Example:**

```
🚀 LangExtract Getting Started Example
======================================

✅ API key loaded
✅ Prompt template created
✅ DeepSeek model initialized
✅ Annotator created
✅ Resolver created
📝 Input text: Alice met Bob at the coffee shop. Charlie joined them later for lunch.

🔄 Processing...

🎉 Extraction Results:
======================
1. 👤 Alice
2. 👤 Bob
3. 👤 Charlie

✨ Done! You've successfully run your first LangExtract example.
💡 Try modifying the prompt or input text to see different results.
```

**Simple Extraction Example:**

```
Input text: Romeo loved Juliet deeply. They met in Verona, feeling joy and sadness.
Processing with DeepSeek...

=== Extraction Results ===
1. [character] "Romeo"
   emotional_state: passionate

2. [character] "Juliet"
   relationship: beloved

3. [location] "Verona"
   significance: meeting_place

4. [emotion] "joy"
   intensity: high

5. [emotion] "sadness"
   context: love_story

=== Done ===
```

## File Outputs

The `character_extraction` example generates:

- `extraction_results.jsonl`: Structured extraction results in JSONL format
- `visualization.html`: Interactive HTML visualization of the extractions

## Troubleshooting

### "API key not set" Error

Make sure you've set the `DEEPSEEK_API_KEY` environment variable:

```bash
echo $DEEPSEEK_API_KEY  # Should print your API key
```

### Network/API Errors

- Check your internet connection
- Verify your API key is valid and has credits
- Try reducing the text length if you get timeout errors

### Compilation Errors

- Make sure you're running from the `langextract` directory
- Check that all dependencies are properly installed with `cargo check`

## Customization

You can modify these examples to:

1. **Use different models**: Change the model ID in `DeepSeekLanguageModel::new()`
2. **Add more examples**: Extend the `examples` vector in the prompt template
3. **Change output format**: Switch between YAML and JSON by changing `FormatType`
4. **Process different text types**: Replace the input text with your own content
5. **Customize extraction classes**: Modify the prompt to extract different entity types

## Advanced Usage

For production use, consider:

- Using async/await properly instead of `block_on()`
- Implementing error handling and retry logic
- Batching multiple documents for efficiency
- Caching results to avoid repeated API calls
- Using environment-specific configuration files

## Need Help?

- Check the main library documentation
- Look at the test files for more usage patterns
- Review the source code for detailed implementation examples
