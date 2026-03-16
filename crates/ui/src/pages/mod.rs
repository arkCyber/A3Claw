pub mod ai_chat;
// pub mod assistant;  // 暂时禁用，需要添加 openclaw-config 依赖
pub mod assistant;
pub mod dashboard;
pub mod env_check_page;
pub mod events;
pub mod settings;
pub mod confirm;
pub mod general_settings;
pub mod skills;
#[cfg(test)]
mod skills_test;
#[path = "claw_terminal.rs.bak"]
pub mod claw_terminal;
