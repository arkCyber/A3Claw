//! 测试 AI 助手与 Plugin Gateway 的集成

use openclaw_assistant::{OpenClawAssistant, SystemContext};
use openclaw_config::RagConfig;
use openclaw_security::SecurityConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 测试 OpenClaw+ AI 助手集成...");
    
    // 1. 创建助手实例
    println!("📝 创建 AI 助手实例...");
    let assistant = OpenClawAssistant::new()?;
    println!("✅ AI 助手创建成功");
    
    // 2. 创建系统上下文
    println!("🔧 创建系统上下文...");
    let security_config = SecurityConfig::default();
    let rag_config = RagConfig::default();
    let system_context = SystemContext::new(&security_config, &rag_config);
    println!("✅ 系统上下文创建成功");
    
    // 3. 测试意图解析
    println!("🧠 测试意图解析...");
    let test_queries = vec![
        "帮我诊断系统问题",
        "如何优化 WasmEdge 性能？",
        "检查安全配置",
        "系统状态如何？",
    ];
    
    for query in test_queries {
        println!("\n📋 查询: {}", query);
        match assistant.parse_intent(query) {
            Ok(intent) => println!("✅ 意图识别: {:?}", intent),
            Err(e) => println!("❌ 意图识别失败: {}", e),
        }
    }
    
    // 4. 测试知识库搜索
    println!("\n📚 测试知识库搜索...");
    match assistant.search_knowledge("WasmEdge", &system_context).await {
        Ok(docs) => println!("✅ 找到 {} 个相关文档", docs.len()),
        Err(e) => println!("❌ 知识库搜索失败: {}", e),
    }
    
    // 5. 测试助手响应生成
    println!("\n💬 测试助手响应生成...");
    let test_query = "如何配置 WasmEdge 沙箱？";
    match assistant.generate_response(test_query, &system_context).await {
        Ok(response) => {
            println!("✅ 响应生成成功:");
            println!("📄 内容: {}", response.content);
            if !response.actions.is_empty() {
                println!("🎯 建议操作:");
                for action in &response.actions {
                    println!("  - {}", action);
                }
            }
        }
        Err(e) => println!("❌ 响应生成失败: {}", e),
    }
    
    println!("\n🎉 AI 助手集成测试完成！");
    Ok(())
}
