//! 技能系统使用示例
//!
//! 演示如何使用 SkillManager 加载和管理技能

use openclaw_agent_executor::SkillManager;

fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🚀 OpenClaw+ 技能系统示例\n");

    // 1. 创建技能管理器
    println!("📦 创建技能管理器...");
    let manager = SkillManager::new()?;
    println!("✅ 技能目录: {:?}\n", manager.skills_dir());

    // 2. 加载所有技能
    println!("🔄 加载技能插件...");
    let count = manager.load_all_skills()?;
    println!("✅ 成功加载 {} 个插件\n", count);

    // 3. 显示统计信息
    println!("📊 统计信息:");
    println!("   - 插件数量: {}", manager.plugin_count());
    println!("   - 技能数量: {}\n", manager.skill_count());

    // 4. 列出所有技能
    println!("📋 可用技能列表:");
    let skills = manager.list_skills();
    
    if skills.is_empty() {
        println!("   ⚠️  未找到任何技能");
        println!("   💡 提示: 请将技能插件安装到 ~/.openclaw/skills/ 目录");
    } else {
        for (i, skill) in skills.iter().enumerate() {
            println!("   {}. {} - {}", i + 1, skill.name, skill.description);
            println!("      风险级别: {}", skill.risk);
            if !skill.params.is_empty() {
                println!("      参数:");
                for param in &skill.params {
                    let required = if param.required { "必需" } else { "可选" };
                    println!("        - {} ({}): {} [{}]", 
                        param.name, param.param_type, param.description, required);
                }
            }
            println!();
        }
    }

    // 5. 查询特定技能
    println!("🔍 查询特定技能:");
    let test_skills = vec!["json.parse", "string.uppercase", "math.add"];
    
    for skill_name in test_skills {
        if manager.has_skill(skill_name) {
            if let Some(info) = manager.get_skill_info(skill_name) {
                println!("   ✅ {}: {}", skill_name, info.display);
                if let Some(plugin_id) = manager.get_plugin_id(skill_name) {
                    println!("      插件: {}", plugin_id);
                }
            }
        } else {
            println!("   ❌ {} - 未找到", skill_name);
        }
    }

    println!("\n✨ 示例完成！");
    Ok(())
}
