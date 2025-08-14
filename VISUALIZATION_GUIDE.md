# LangExtract 可视化功能使用指南

本指南介绍如何使用 LangExtract 的两种可视化风格来展示文本实体提取结果。

## 功能概述

LangExtract 提供了两种不同的可视化风格：

1. **动画交互式风格** (`VisualizationStyle::Animated`) - 原有的动画播放风格
2. **中文古典静态风格** (`VisualizationStyle::ChineseClassical`) - 新增的静态展示风格，带有点击统计功能

## 快速开始

### 基本用法

```rust
use langextract::data::AnnotatedDocument;
use langextract::visualization::{DataSource, VisualizationStyle, VisualizeOptions, visualize};

// 创建可视化选项
let options = VisualizeOptions {
    animation_speed: 2.0,           // 动画速度（仅动画风格使用）
    show_legend: true,              // 是否显示图例
    gif_optimized: false,           // 是否启用GIF优化
    context_chars: 100,             // 上下文字符数
    style: VisualizationStyle::ChineseClassical,  // 选择风格
};

// 生成可视化HTML
let html = visualize(DataSource::Document(annotated_doc), options)?;

// 保存到文件
std::fs::write("visualization.html", html)?;
```

## 可视化风格对比

### 1. 动画交互式风格 (Animated)

**特点：**
- 动态播放实体提取结果
- 包含播放/暂停/上一个/下一个控制按钮
- 进度条显示当前进度
- 实时显示当前实体的属性信息
- 可手动跳转到指定实体

**适用场景：**
- 演示和教学
- 逐步分析文本结构
- 生成可视化GIF动画

**示例代码：**
```rust
let animated_options = VisualizeOptions {
    animation_speed: 1.5,  // 每1.5秒切换一个实体
    show_legend: true,
    gif_optimized: true,   // 适合录制GIF
    context_chars: 150,
    style: VisualizationStyle::Animated,
};
```

### 2. 中文古典静态风格 (ChineseClassical)

**特点：**
- 静态展示所有实体
- 古典中文风格的界面设计
- 可点击的统计信息
- 弹窗显示详细的实体列表
- 支持按类别查看实体
- 鼠标悬停显示实体详情

**适用场景：**
- 最终结果展示
- 中文文档处理
- 学术报告和文档
- 批量查看提取结果

**示例代码：**
```rust
let chinese_options = VisualizeOptions {
    animation_speed: 1.0,  // 静态风格中不使用
    show_legend: true,
    gif_optimized: false,
    context_chars: 200,
    style: VisualizationStyle::ChineseClassical,
};
```

## 配置选项详解

### VisualizeOptions 结构体

```rust
pub struct VisualizeOptions {
    /// 动画速度（秒），仅在动画风格中使用
    pub animation_speed: f32,

    /// 是否显示颜色图例
    pub show_legend: bool,

    /// 是否启用GIF优化（更大字体等）
    pub gif_optimized: bool,

    /// 实体周围显示的上下文字符数
    pub context_chars: usize,

    /// 可视化风格选择
    pub style: VisualizationStyle,
}
```

### VisualizationStyle 枚举

```rust
pub enum VisualizationStyle {
    /// 原始动画风格，带有控制界面
    Animated,

    /// 静态中文古典风格
    ChineseClassical,
}
```

## 中文古典风格的特色功能

### 1. 可点击统计

点击统计数字可以查看对应类别的所有实体：

- **总计实体** - 显示所有提取的实体列表
- **人物角色** - 显示所有人物实体及其属性
- **地点场所** - 显示所有地点实体及其属性
- **物品器具** - 显示所有物品实体及其属性
- **服饰装扮** - 显示所有服饰实体及其属性
- **情感状态** - 显示所有情感实体及其属性
- **自然景物** - 显示所有自然景物实体及其属性

### 2. 实体分类和图标

每种实体类型都有对应的中文名称和图标：

| 英文类别 | 中文名称 | 图标 |
|---------|----------|------|
| characters | 人物角色 | 👤 |
| locations | 地点场所 | 🏛️ |
| objects | 物品器具 | 📿 |
| clothing | 服饰装扮 | 👘 |
| emotions | 情感状态 | 💭 |
| nature | 自然景物 | 🌸 |

### 3. 弹窗详情

点击统计后弹出的详情窗口包含：

- **实体文本** - 提取的原始文本
- **实体类型** - 带图标的中文类型名称
- **位置信息** - 字符位置范围 `[start-end]`
- **属性信息** - 实体的额外属性（如果有）

## 完整示例

查看 `examples/dual_style_visualization.rs` 获取完整的使用示例：

```bash
cargo run --example dual_style_visualization
```

该示例会生成两个HTML文件：
- `animated_visualization.html` - 动画交互式风格
- `chinese_classical_visualization.html` - 中文古典静态风格

## 自定义样式

### 修改颜色方案

颜色由 `assign_colors` 函数自动分配，但您可以通过修改 `PALETTE` 常量来自定义颜色：

```rust
const PALETTE: [&str; 12] = [
    "#ffadad", "#ffd6a5", "#fdffb6", "#caffbf",
    "#9bf6ff", "#a0c4ff", "#bdb2ff", "#ffc6ff",
    "#ffb3ba", "#ffdfba", "#ffffba", "#baffc9"
];
```

### 扩展实体类型

要添加新的实体类型支持，需要：

1. 在 `get_chinese_category_name` 函数中添加映射
2. 在 `get_category_icon` 函数中添加图标
3. 在CSS中添加对应的样式类（如需要）

## 注意事项

1. **字符索引 vs 字节索引**：确保使用字符索引而非字节索引，特别是处理中文文本时
2. **HTML转义**：实体文本会自动进行HTML转义处理
3. **浏览器兼容性**：建议使用现代浏览器查看生成的HTML文件
4. **文件大小**：对于大量实体的文档，生成的HTML文件可能较大

## 故障排除

### 常见问题

1. **字符边界错误**：确保 `char_interval` 使用正确的字符位置
2. **样式不显示**：检查CSS是否正确加载
3. **JavaScript错误**：确保浏览器启用了JavaScript（中文古典风格的点击功能需要）

### 调试技巧

```rust
// 启用详细日志
RUST_LOG=debug cargo run --example dual_style_visualization

// 检查生成的HTML结构
// 在浏览器中打开开发者工具查看
```

## API 文档

完整的API文档请查看：

```bash
cargo doc --open
```

然后导航到 `langextract::visualization` 模块。
