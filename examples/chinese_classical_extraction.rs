//! 中文古典文本提取示例 - 红楼梦风格
//!
//! 本示例展示如何使用 LangExtract 库从中文古典文学风格的文本中提取实体。
//! 使用红楼梦的写作风格创作的原创文本进行演示。
//!
//! 运行此示例:
//! 1. 设置环境变量: export DEEPSEEK_API_KEY="your-api-key-here"
//! 2. 运行: cargo run --example chinese_classical_extraction

use langextract::{
    annotation::Annotator,
    data::{Document, FormatType},
    inference::DeepSeekLanguageModel,
    prompting::PromptTemplateStructured,
    resolver::Resolver,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏮 中文古典文学实体提取示例");
    println!("==============================\n");

    // 步骤 1: 获取 API 密钥
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("❌ 请设置 DEEPSEEK_API_KEY 环境变量");

    println!("✅ API 密钥已加载");

    // 步骤 2: 创建提取提示词
    let prompt = PromptTemplateStructured {
        description: r#"从中文古典文学文本中提取以下实体:
- 人物姓名 (characters): 文中提到的人物姓名
- 地点名称 (locations): 园林、房间、建筑物等地点
- 物品器具 (objects): 文中提到的具体物品
- 情感状态 (emotions): 描述的情感或心理状态
- 服饰装扮 (clothing): 衣物、首饰等描述

请提取文中出现的确切文字，保持原文用词。"#
            .to_string(),
        examples: vec![], // 简单起见不使用示例
    };

    println!("✅ 提示词模板已创建");

    // 步骤 3: 准备测试文本 (红楼梦风格的原创文本)
    let chinese_text = r#"
宝玉今日穿了一件月白缎子袍子，腰系丝绦，头戴紫金冠，脚蹬云头履，
正在怡红院中读书。忽见袭人端了一盏碧螺春茶进来，笑道："二爷，
林姑娘在潇湘馆等您呢。"宝玉听了，心中欢喜，忙放下手中的《西厢记》，
起身便往潇湘馆而去。一路上花香阵阵，春风习习，心情甚是愉悦。
到了潇湘馆，只见黛玉正对着梨花垂泪，手持花锄，神情哀怨。
    "#
    .trim();

    println!("📝 输入文本:");
    println!("{}", chinese_text);
    println!();

    // 步骤 4: 设置 DeepSeek 模型
    let model = DeepSeekLanguageModel::new(
        None,                   // 使用默认模型
        api_key,                // API 密钥
        None,                   // 使用默认 URL
        Some(FormatType::Yaml), // 输出格式
        Some(0.1),              // 低温度以获得一致结果
        Some(1),                // 单一工作线程
        None,                   // 无额外参数
    )?;

    println!("✅ DeepSeek 模型已初始化");

    // 步骤 5: 创建注释器
    let annotator = Annotator::new(
        model,
        prompt,
        FormatType::Yaml,
        None, // 使用默认属性后缀
        true, // 使用围栏输出
    );

    println!("✅ 注释器已创建");

    // 步骤 6: 创建解析器 (注意: format_is_yaml 应该设为 true)
    let resolver = Resolver::new(true, None, None, true);

    println!("✅ 解析器已创建");
    println!("\n🔄 正在处理中文文本...");

    // 步骤 7: 运行提取
    let document = Document::new(chinese_text.to_string(), Some("chinese_classical".to_string()), None);

    let results = annotator
        .annotate_documents(
            vec![document],
            &resolver,
            2000, // 最大字符数 (中文需要更多空间)
            1,    // 批处理大小
            true, // 启用调试输出
            1,    // 提取次数
            None, // 无额外参数
        )
        .await?;

    // 步骤 8: 显示结果
    println!("\n🎭 提取结果:");
    println!("================");

    if let Some(extractions) = &results[0].extractions {
        if extractions.is_empty() {
            println!("😔 未找到提取内容。请尝试调整提示词。");
        } else {
            // 按类别分组显示
            let mut categories: std::collections::HashMap<String, Vec<&langextract::data::Extraction>> =
                std::collections::HashMap::new();

            for extraction in extractions {
                categories
                    .entry(extraction.extraction_class.clone())
                    .or_insert_with(Vec::new)
                    .push(extraction);
            }

            for (category, items) in categories {
                println!(
                    "\n📋 {} ({} 项):",
                    match category.as_str() {
                        "characters" => "人物姓名",
                        "locations" => "地点名称",
                        "objects" => "物品器具",
                        "emotions" => "情感状态",
                        "clothing" => "服饰装扮",
                        _ => &category,
                    },
                    items.len()
                );

                for (i, item) in items.iter().enumerate() {
                    println!("  {}. 「{}」", i + 1, item.extraction_text);

                    if let Some(attributes) = &item.attributes {
                        for (key, value) in attributes {
                            match value {
                                langextract::data::AttributeValue::Single(v) => {
                                    println!("     💭 {}: {}", key, v);
                                }
                                langextract::data::AttributeValue::Multiple(v) => {
                                    println!("     💭 {}: {:?}", key, v);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        println!("😔 未找到提取内容。");
    }

    println!("\n✨ 完成！您已成功运行中文古典文学提取示例。");
    println!("💡 尝试修改文本或提示词以查看不同的结果。");
    println!("🏮 愿您在古典文学的海洋中尽情遨游！");

    Ok(())
}
