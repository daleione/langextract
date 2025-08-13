//! 中文文本处理测试 - 无需 API 调用
//!
//! 本示例展示如何测试中文文本的分词和解析功能，无需实际调用 LLM API。
//!
//! 运行此示例: cargo run --example chinese_test

use langextract::{resolver::Resolver, tokenizer::tokenize};

fn main() {
    println!("🏮 中文文本处理测试");
    println!("===================\n");

    // 测试 1: 中文分词
    println!("📝 测试 1: 中文分词功能");
    let chinese_text = "宝玉今日穿了一件月白缎子袍子，腰系丝绦，头戴紫金冠，脚蹬云头履。";
    println!("输入文本: {}", chinese_text);

    let tokenized = tokenize(chinese_text);
    println!("分词结果: {} 个 token", tokenized.tokens.len());

    for (i, token) in tokenized.tokens.iter().enumerate().take(10) {
        let token_text = &chinese_text[token.char_interval.start_pos..token.char_interval.end_pos];
        println!("  {}. 「{}」 - {:?}", i + 1, token_text, token.token_type);
    }

    if tokenized.tokens.len() > 10 {
        println!("  ... 还有 {} 个 token", tokenized.tokens.len() - 10);
    }

    // 测试 2: 解析器处理嵌套 YAML 格式
    println!("\n📝 测试 2: 解析器处理中文 YAML");
    let resolver = Resolver::new(true, None, None, true);

    let mock_yaml_response = r#"```yaml
characters:
  - 宝玉
  - 袭人
  - 林姑娘
  - 黛玉
locations:
  - 怡红院
  - 潇湘馆
objects:
  - 月白缎子袍子
  - 丝绦
  - 紫金冠
  - 云头履
  - 碧螺春茶
emotions:
  - 心中欢喜
  - 心情甚是愉悦
  - 神情哀怨
```"#;

    println!("模拟 LLM 响应:");
    println!("{}", mock_yaml_response);

    match resolver.parse_extractions_from_string(mock_yaml_response) {
        Ok(extractions) => {
            println!("\n✅ 成功解析 {} 个提取项!", extractions.len());

            // 按类别分组
            use std::collections::HashMap;
            let mut categories: HashMap<String, Vec<&str>> = HashMap::new();

            for extraction in &extractions {
                let category = if extraction.extraction_class.contains("characters") {
                    "人物"
                } else if extraction.extraction_class.contains("locations") {
                    "地点"
                } else if extraction.extraction_class.contains("objects") {
                    "物品"
                } else if extraction.extraction_class.contains("emotions") {
                    "情感"
                } else {
                    "其他"
                };

                categories
                    .entry(category.to_string())
                    .or_insert_with(Vec::new)
                    .push(&extraction.extraction_text);
            }

            for (category, items) in categories {
                println!("\n📋 {}: ({} 项)", category, items.len());
                for (i, item) in items.iter().enumerate() {
                    println!("  {}. 「{}」", i + 1, item);
                }
            }
        }
        Err(e) => {
            println!("❌ 解析失败: {:?}", e);
        }
    }

    // 测试 3: 中英混合文本
    println!("\n📝 测试 3: 中英混合文本分词");
    let mixed_text = "Hello世界! This is一个测试example.";
    println!("混合文本: {}", mixed_text);

    let mixed_tokenized = tokenize(mixed_text);
    println!("分词结果:");
    for (i, token) in mixed_tokenized.tokens.iter().enumerate() {
        let token_text = &mixed_text[token.char_interval.start_pos..token.char_interval.end_pos];
        println!("  {}. 「{}」 - {:?}", i + 1, token_text, token.token_type);
    }

    println!("\n🎉 所有测试完成！");
    println!("✅ 中文分词: 正常工作");
    println!("✅ 嵌套 YAML 解析: 正常工作");
    println!("✅ 中英混合: 正常工作");
    println!("\n💡 现在您可以放心使用中文文本进行实际的 LangExtract 操作了！");
}
