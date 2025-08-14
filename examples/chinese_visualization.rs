//! 中文可视化示例 - 红楼梦风格文本实体提取与可视化
//!
//! 本示例展示如何从中文古典文学文本中提取实体并生成中文古典风格的HTML可视化，
//! 包括可点击统计、实体高亮、图例和详细信息弹窗。
//!
//! 运行此示例:
//! 1. 设置环境变量: export DEEPSEEK_API_KEY="your-api-key-here"
//! 2. 运行: cargo run --example chinese_visualization
//! 3. 查看生成的文件: chinese_extraction_results.txt 和 chinese_visualization.html
//!
//! 特色功能:
//! - 中文古典风格界面设计
//! - 可点击的统计数字查看实体详情
//! - 支持按类别筛选实体
//! - 响应式设计，支持移动设备

use langextract::{
    annotation::Annotator,
    data::{Document, FormatType},
    inference::DeepSeekLanguageModel,
    io::save_str,
    prompting::{ExampleData, Extraction, PromptTemplateStructured},
    resolver::Resolver,
    visualization::{DataSource, VisualizationStyle, VisualizeOptions, visualize},
};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏮 中文文本可视化示例");
    println!("======================\n");

    // 步骤 1: 获取 API 密钥
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("❌ 请设置 DEEPSEEK_API_KEY 环境变量");

    println!("✅ API 密钥已加载");

    // 步骤 2: 创建带示例的详细提示词
    let examples = vec![
        ExampleData {
            text: "林黛玉身穿淡雅的月白色长裙，手持诗卷在潇湘馆中独自垂泪。".to_string(),
            extractions: vec![
                Extraction {
                    extraction_class: "characters".to_string(),
                    extraction_text: "林黛玉".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "clothing".to_string(),
                    extraction_text: "月白色长裙".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "objects".to_string(),
                    extraction_text: "诗卷".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "locations".to_string(),
                    extraction_text: "潇湘馆".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "emotions".to_string(),
                    extraction_text: "独自垂泪".to_string(),
                    attributes: None,
                },
            ],
        },
        ExampleData {
            text: "贾宝玉头戴束发紫金冠，身穿二色金百蝶穿花大红箭袖，在大观园的亭台楼阁间游玩。".to_string(),
            extractions: vec![
                Extraction {
                    extraction_class: "characters".to_string(),
                    extraction_text: "贾宝玉".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "clothing".to_string(),
                    extraction_text: "束发紫金冠".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "clothing".to_string(),
                    extraction_text: "二色金百蝶穿花大红箭袖".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "locations".to_string(),
                    extraction_text: "大观园".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "locations".to_string(),
                    extraction_text: "亭台楼阁".to_string(),
                    attributes: None,
                },
                Extraction {
                    extraction_class: "emotions".to_string(),
                    extraction_text: "游玩".to_string(),
                    attributes: None,
                },
            ],
        },
    ];

    let prompt = PromptTemplateStructured {
        description: r#"从中国古典文学文本中精确提取以下类型的实体：

- characters: 人物姓名和称谓
- locations: 地点、建筑、房间名称
- objects: 具体的物品、器具、书籍
- clothing: 服饰、装扮、首饰描述
- emotions: 情感状态、心理活动、行为表现
- nature: 自然景物、花草树木、天气现象

请保持原文用词，提取完整的描述性短语。"#
            .to_string(),
        examples,
    };

    println!("✅ 带示例的提示词模板已创建");

    // 步骤 3: 准备红楼梦风格的测试文本
    let chinese_text = r#"
原来王夫人时常居坐宴息，亦不在这正室，只在这正室东边的三间耳房内。于是老嬷嬷引黛玉进东房门来。临窗大炕上铺着猩红洋罽，正面设着大红金钱蟒靠背，石青金钱蟒引枕，秋香色金钱蟒大条褥。两边设一对梅花式洋漆小几。左边几上文王鼎匙箸香盒；右边几上汝窑美人觚——觚内插着时鲜花卉，并茗碗痰盒等物。地下面西一溜四张椅上，都搭着银红撒花椅搭，底下四副脚踏。椅之两边，也有一对高几，几上茗碗瓶花俱备。其余陈设，自不必细说。
老嬷嬷们让黛玉炕上坐，炕沿上却有两个锦褥对设，黛玉度其位次，便不上炕，只向东边椅子上坐了。本房内的丫鬟忙捧上茶来。黛玉一面吃茶，一面打谅这些丫鬟们，妆饰衣裙，举止行动，果亦与别家不同。茶未吃了，只见一个穿红绫袄青缎掐牙背心的丫鬟走来笑说道：“太太说，请林姑娘到那边坐罢。”老嬷嬷听了，于是又引黛玉出来，到了东廊三间小正房内。
正面炕上横设一张炕桌，桌上磊着书籍茶具，靠东壁面西设着半旧的青缎靠背引枕。王夫人却坐在西边下首，亦是半旧的青缎靠背坐褥。见黛玉来了，便往东让。黛玉心中料定这是贾政之位。因见挨炕一溜三张椅子上，也搭着半旧的弹墨椅袱，黛玉便向椅上坐了。王夫人再四携他上炕，他方挨王夫人坐了。王夫人因说：“你舅舅今日斋戒去了，再见罢。只是有一句话嘱咐你：你三个姊妹倒都极好，以后一处念书认字学针线，或是偶一顽笑，都有尽让的。但我不放心的最是一件：我有一个孽根祸胎，是家里的‘混世魔王’，今日因庙里还愿去了，尚未回来，晚间你看见便知了。你只以后不要睬他，你这些姊妹都不敢沾惹他的。”
黛玉亦常听得母亲说过，二舅母生的有个表兄，乃衔玉而诞，顽劣异常，极恶读书，最喜在内帏厮混；外祖母又极溺爱，无人敢管。今见王夫人如此说，便知说的是这表兄了。因陪笑道：“舅母说的，可是衔玉所生的这位哥哥？在家时亦曾听见母亲常说，这位哥哥比我大一岁，小名就唤宝玉，虽极憨顽，说在姊妹情中极好的。况我来了，自然只和姊妹同处，兄弟们自是别院另室的，岂得去沾惹之理？”王夫人笑道：“你不知道原故：他与别人不同，自幼因老太太疼爱，原系同姊妹们一处娇养惯了的。若姊妹们有日不理他，他倒还安静些，纵然他没趣，不过出了二门，背地里拿着他两个小幺儿出气，咕唧一会子就完了。若这一日姊妹们和他多说一句话，他心里一乐，便生出多少事来。所以嘱咐你别睬他。他嘴里一时甜言蜜语，一时有天无日，一时又疯疯傻傻，只休信他。”
黛玉一一的都答应着。只见一个丫鬟来回：“老太太那里传晚饭了。”王夫人忙携黛玉从后房门由后廊往西，出了角门，是一条南北宽夹道。南边是倒座三间小小的抱厦厅，北边立着一个粉油大影壁，后有一半大门，小小一所房室。王夫人笑指向黛玉道：“这是你凤姐姐的屋子，回来你好往这里找他来，少什么东西，你只管和他说就是了。”这院门上也有四五个才总角的小厮，都垂手侍立。王夫人遂携黛玉穿过一个东西穿堂，便是贾母的后院了。
于是，进入后房门，已有多人在此伺候，见王夫人来了，方安设桌椅。贾珠之妻李氏捧饭，熙凤安箸，王夫人进羹。贾母正面榻上独坐，两边四张空椅，熙凤忙拉了黛玉在左边第一张椅上坐了，黛玉十分推让。贾母笑道：“你舅母你嫂子们不在这里吃饭。你是客，原应如此坐的。”黛玉方告了座，坐了。贾母命王夫人坐了。迎春姊妹三个告了座方上来。迎春便坐右手第一，探春坐左第二，惜春坐右第二。旁边丫鬟执着拂尘、漱盂、巾帕。李、凤二人立于案旁布让。外间伺候之媳妇丫鬟虽多，却连一声咳嗽不闻。
寂然饭毕，各有丫鬟用小茶盘捧上茶来。当日林如海教女以惜福养身，云饭后务待饭粒咽尽，过一时再吃茶，方不伤脾胃。今黛玉见了这里许多事情不合家中之式，不得不随的，少不得一一改过来，因而接了茶。早见人又捧过漱盂来，黛玉也照样漱了口。盥手毕，又捧上茶来，这方是吃的茶。贾母便说：“你们去罢，让我们自在说话儿。”王夫人听了，忙起身，又说了两句闲话，方引凤、李二人去了。贾母因问黛玉念何书。黛玉道：“只刚念了《四书》。”黛玉又问姊妹们读何书。贾母道：“读的是什么书，不过是认得两个字，不是睁眼的瞎子罢了！”
    "#
    .trim();

    println!("📝 红楼梦风格测试文本:");
    println!("{}\n", chinese_text);

    // 步骤 4: 设置 DeepSeek 模型
    let model = DeepSeekLanguageModel::new(
        None,                   // 使用默认模型
        api_key,                // API 密钥
        None,                   // 使用默认 URL
        Some(FormatType::Yaml), // 输出格式
        Some(0.2),              // 稍高的温度以获得更丰富的提取
        Some(1),                // 单一工作线程
        None,                   // 无额外参数
    )?;

    println!("✅ DeepSeek 模型已初始化");

    // 步骤 5: 创建注释器
    let annotator = Annotator::new(
        model,
        prompt,
        FormatType::Yaml,
        Some("_attributes"), // 使用属性后缀
        true,                // 使用围栏输出
    );

    println!("✅ 注释器已创建");

    // 步骤 6: 创建解析器
    let resolver = Resolver::new(true, None, None, true);

    println!("✅ 解析器已创建");
    println!("\n🔄 正在提取中文实体...");

    // 步骤 7: 运行提取
    let document = Document::new(chinese_text.to_string(), Some("chinese_classic".to_string()), None);

    let results = annotator
        .annotate_documents(
            vec![document],
            &resolver,
            2000, // 最大字符数 (中文需要更多空间)
            1,    // 批处理大小
            true, // 启用调试输出以查看API响应
            1,    // 提取次数
            None, // 无额外参数
        )
        .await?;

    let mut annotated_doc = results[0].clone();

    // 步骤 8: 显示提取结果统计
    if let Some(extractions) = &annotated_doc.extractions {
        println!("✅ 成功提取 {} 个实体!\n", extractions.len());

        // 按类别分组统计
        let mut categories: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();

        for extraction in extractions {
            let category = classify_extraction_category(&extraction.extraction_class);
            categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push(&extraction.extraction_text);
        }

        println!("📊 提取统计:");
        for (category, items) in &categories {
            println!("  • {}: {} 项", category, items.len());
        }
        println!();

        // 显示详细结果
        println!("📋 详细提取结果:");
        for (category, items) in categories {
            println!("\n🏷️  {}:", category);
            for (i, item) in items.iter().enumerate() {
                println!("    {}. 「{}」", i + 1, item);
            }
        }
    } else {
        println!("😔 未找到提取内容。");
        return Ok(());
    }

    // 步骤 9: 保存结果到文件
    println!("\n💾 保存结果到文件...");
    let results_filename = "chinese_extraction_results.txt";

    // 创建简单的文本总结
    let mut results_text = String::new();
    results_text.push_str("🏮 中文文本实体提取结果\n");
    results_text.push_str("======================\n\n");
    results_text.push_str(&format!("📝 原文:\n{}\n\n", chinese_text));

    if let Some(extractions) = &annotated_doc.extractions {
        results_text.push_str(&format!("📊 提取统计: {} 个实体\n\n", extractions.len()));

        // 按类别分组
        let mut categories: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
        for extraction in extractions {
            let category = classify_extraction_category(&extraction.extraction_class);
            categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push(&extraction.extraction_text);
        }

        for (category, items) in categories {
            results_text.push_str(&format!("🏷️  {}:\n", category));
            for (i, item) in items.iter().enumerate() {
                results_text.push_str(&format!("    {}. 「{}」\n", i + 1, item));
            }
            results_text.push('\n');
        }
    }

    save_str(Path::new(results_filename), &results_text)?;
    println!("✅ 结果已保存到: {}", results_filename);

    // 步骤 9.5: 添加文本对齐信息
    if let Some(extractions) = &annotated_doc.extractions {
        if !extractions.is_empty() {
            println!("🔧 正在为实体添加位置信息...");
            let mut enhanced_extractions = Vec::new();

            for extraction in extractions {
                let mut enhanced = extraction.clone();

                // 在文本中查找实体位置 (使用字符索引而不是字节索引)
                if let Some(byte_start_pos) = chinese_text.find(&extraction.extraction_text) {
                    let byte_end_pos = byte_start_pos + extraction.extraction_text.len();

                    // 转换字节索引为字符索引
                    let char_start_pos = chinese_text[..byte_start_pos].chars().count();
                    let char_end_pos = chinese_text[..byte_end_pos].chars().count();

                    enhanced.char_interval = Some(langextract::data::CharInterval {
                        start_pos: Some(char_start_pos),
                        end_pos: Some(char_end_pos),
                    });
                    enhanced.alignment_status = Some(langextract::data::AlignmentStatus::MatchExact);
                }
                enhanced_extractions.push(enhanced);
            }

            annotated_doc.extractions = Some(enhanced_extractions);
            println!("✅ 位置信息添加完成");
        }
    }

    // 步骤 10: 生成 HTML 可视化
    println!("🎨 生成 HTML 可视化...");

    let visualization_options = VisualizeOptions {
        animation_speed: 1.0, // 静态风格中不使用此参数
        show_legend: true,
        gif_optimized: false,
        context_chars: 100,
        style: VisualizationStyle::ChineseClassical,
    };

    // 直接使用实际提取结果生成可视化
    let html_content = visualize(DataSource::Document(annotated_doc.clone()), visualization_options)?;

    let html_filename = "chinese_visualization.html";
    fs::write(html_filename, html_content)?;
    println!("✅ HTML 可视化已保存到: {}", html_filename);

    // 步骤 11: 完成总结
    println!("\n🎉 中文可视化示例完成!");
    println!("📁 生成的文件:");
    println!("   • {}: 文本格式提取结果", results_filename);
    println!("   • {}: 交互式可视化", html_filename);
    println!("\n💡 打开 {} 查看精美的中文实体可视化效果!", html_filename);
    println!("🏮 体验古典文学与现代技术的完美融合!");

    Ok(())
}

/// 分类提取实体的类别
fn classify_extraction_category(extraction_class: &str) -> String {
    match extraction_class {
        s if s.contains("characters") => "👤 人物角色".to_string(),
        s if s.contains("locations") => "🏛️ 地点场所".to_string(),
        s if s.contains("objects") => "📿 物品器具".to_string(),
        s if s.contains("clothing") => "👘 服饰装扮".to_string(),
        s if s.contains("emotions") => "💭 情感状态".to_string(),
        s if s.contains("nature") => "🌸 自然景物".to_string(),
        _ => "🔍 其他实体".to_string(),
    }
}
