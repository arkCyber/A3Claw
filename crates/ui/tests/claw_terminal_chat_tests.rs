//! Claw Terminal Agent Chat 功能测试
//!
//! 测试 Claw 终端的对话功能，包括：
//! - Agent 选择
//! - 消息发送和路由
//! - 多轮对话上下文管理
//! - 图片附件处理
//! - 响应显示

use std::collections::HashMap;

#[cfg(test)]
mod claw_terminal_chat_tests {
    use super::*;

    /// 模拟 ConversationTurn 结构
    #[derive(Debug, Clone, PartialEq)]
    struct ConversationTurn {
        role: String,
        content: String,
    }

    /// 模拟 Agent 对话历史管理
    struct AgentConversationManager {
        conversations: HashMap<String, Vec<ConversationTurn>>,
    }

    impl AgentConversationManager {
        fn new() -> Self {
            Self {
                conversations: HashMap::new(),
            }
        }

        /// 添加用户消息
        fn add_user_message(&mut self, agent_id: &str, content: String) {
            self.conversations
                .entry(agent_id.to_string())
                .or_insert_with(Vec::new)
                .push(ConversationTurn {
                    role: "user".to_string(),
                    content,
                });
        }

        /// 添加 Assistant 回复
        fn add_assistant_message(&mut self, agent_id: &str, content: String) {
            self.conversations
                .entry(agent_id.to_string())
                .or_insert_with(Vec::new)
                .push(ConversationTurn {
                    role: "assistant".to_string(),
                    content,
                });
        }

        /// 获取对话历史（最近 N 轮）
        fn get_history(&self, agent_id: &str, max_turns: usize) -> Vec<ConversationTurn> {
            self.conversations
                .get(agent_id)
                .map(|history| {
                    history
                        .iter()
                        .rev()
                        .take(max_turns)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect()
                })
                .unwrap_or_default()
        }

        /// 获取对话轮数
        fn get_turn_count(&self, agent_id: &str) -> usize {
            self.conversations
                .get(agent_id)
                .map(|h| h.len())
                .unwrap_or(0)
        }

        /// 清空对话历史
        fn clear_history(&mut self, agent_id: &str) {
            self.conversations.remove(agent_id);
        }
    }

    /// 模拟图片附件处理
    struct ImageAttachment {
        mime: String,
        base64: String,
    }

    impl ImageAttachment {
        fn new(mime: &str, base64: &str) -> Self {
            Self {
                mime: mime.to_string(),
                base64: base64.to_string(),
            }
        }

        /// 格式化为消息前缀
        fn to_message_prefix(&self) -> String {
            format!("[image:{};{}]", self.mime, self.base64)
        }
    }

    /// 处理图片附件消息
    fn process_image_message(attachment: Option<ImageAttachment>, text: &str) -> String {
        if let Some(att) = attachment {
            format!("{}\n{}", att.to_message_prefix(), text)
        } else {
            text.to_string()
        }
    }

    /// 优化图片消息显示（去除 base64）
    fn optimize_image_display(message: &str) -> String {
        if message.starts_with("[image:") {
            let text_part = message
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n");
            if text_part.trim().is_empty() {
                "📎 [图片已附加]".to_string()
            } else {
                format!("📎 [图片] {}", text_part)
            }
        } else {
            message.to_string()
        }
    }

    /// 优化存储内容（去除 base64）
    fn optimize_storage_content(message: &str) -> String {
        if message.starts_with("[image:") {
            let text_part = message
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n");
            if text_part.trim().is_empty() {
                "[图片已附加]".to_string()
            } else {
                format!("[图片] {}", text_part.trim())
            }
        } else {
            message.to_string()
        }
    }

    /// 截断内容到指定长度
    fn truncate_content(content: &str, max_len: usize) -> String {
        if content.len() > max_len {
            let mut truncated = content[..max_len].to_string();
            truncated.push_str("…");
            truncated
        } else {
            content.to_string()
        }
    }

    // ==================== 测试用例 ====================

    #[test]
    fn test_agent_conversation_manager_basic() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        // 添加用户消息
        manager.add_user_message(agent_id, "你好".to_string());
        assert_eq!(manager.get_turn_count(agent_id), 1);

        // 添加 Assistant 回复
        manager.add_assistant_message(agent_id, "你好！有什么可以帮助你的吗？".to_string());
        assert_eq!(manager.get_turn_count(agent_id), 2);

        // 获取历史
        let history = manager.get_history(agent_id, 10);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "你好");
        assert_eq!(history[1].role, "assistant");
    }

    #[test]
    fn test_multi_turn_conversation() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        // 第一轮
        manager.add_user_message(agent_id, "如何重置密码？".to_string());
        manager.add_assistant_message(agent_id, "重置密码的步骤如下：...".to_string());

        // 第二轮
        manager.add_user_message(agent_id, "如果忘记了邮箱怎么办？".to_string());
        manager.add_assistant_message(agent_id, "可以联系管理员...".to_string());

        // 第三轮
        manager.add_user_message(agent_id, "管理员的联系方式是什么？".to_string());
        manager.add_assistant_message(agent_id, "管理员邮箱是 admin@example.com".to_string());

        assert_eq!(manager.get_turn_count(agent_id), 6);

        // 获取最近 4 轮
        let history = manager.get_history(agent_id, 4);
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].content, "如果忘记了邮箱怎么办？");
        assert_eq!(history[3].content, "管理员邮箱是 admin@example.com");
    }

    #[test]
    fn test_independent_agent_conversations() {
        let mut manager = AgentConversationManager::new();

        // Agent 1 对话
        manager.add_user_message("agent-001", "问题1".to_string());
        manager.add_assistant_message("agent-001", "回答1".to_string());

        // Agent 2 对话
        manager.add_user_message("agent-002", "问题2".to_string());
        manager.add_assistant_message("agent-002", "回答2".to_string());

        // 验证独立性
        assert_eq!(manager.get_turn_count("agent-001"), 2);
        assert_eq!(manager.get_turn_count("agent-002"), 2);

        let history1 = manager.get_history("agent-001", 10);
        let history2 = manager.get_history("agent-002", 10);

        assert_eq!(history1[0].content, "问题1");
        assert_eq!(history2[0].content, "问题2");
    }

    #[test]
    fn test_conversation_history_limit() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        // 添加 10 轮对话
        for i in 1..=10 {
            manager.add_user_message(agent_id, format!("问题{}", i));
            manager.add_assistant_message(agent_id, format!("回答{}", i));
        }

        assert_eq!(manager.get_turn_count(agent_id), 20);

        // 获取最近 6 轮（12 条消息）
        let history = manager.get_history(agent_id, 12);
        assert_eq!(history.len(), 12);
        assert_eq!(history[0].content, "问题5");
        assert_eq!(history[11].content, "回答10");
    }

    #[test]
    fn test_clear_conversation_history() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        manager.add_user_message(agent_id, "测试".to_string());
        manager.add_assistant_message(agent_id, "回复".to_string());
        assert_eq!(manager.get_turn_count(agent_id), 2);

        manager.clear_history(agent_id);
        assert_eq!(manager.get_turn_count(agent_id), 0);
    }

    #[test]
    fn test_image_attachment_basic() {
        let attachment = ImageAttachment::new("image/png", "iVBORw0KGgo...");
        let prefix = attachment.to_message_prefix();
        assert_eq!(prefix, "[image:image/png;iVBORw0KGgo...]");
    }

    #[test]
    fn test_process_image_message_with_attachment() {
        let attachment = Some(ImageAttachment::new("image/png", "base64data"));
        let message = process_image_message(attachment, "这是什么错误？");
        assert!(message.starts_with("[image:image/png;base64data]"));
        assert!(message.contains("这是什么错误？"));
    }

    #[test]
    fn test_process_image_message_without_attachment() {
        let message = process_image_message(None, "普通文本消息");
        assert_eq!(message, "普通文本消息");
    }

    #[test]
    fn test_optimize_image_display_with_text() {
        let message = "[image:image/png;base64data]\n这是什么错误？";
        let display = optimize_image_display(message);
        assert_eq!(display, "📎 [图片] 这是什么错误？");
    }

    #[test]
    fn test_optimize_image_display_without_text() {
        let message = "[image:image/png;base64data]\n";
        let display = optimize_image_display(message);
        assert_eq!(display, "📎 [图片已附加]");
    }

    #[test]
    fn test_optimize_image_display_plain_text() {
        let message = "普通文本消息";
        let display = optimize_image_display(message);
        assert_eq!(display, "普通文本消息");
    }

    #[test]
    fn test_optimize_storage_content_with_image() {
        let message = "[image:image/png;base64data]\n这是什么错误？";
        let stored = optimize_storage_content(message);
        assert_eq!(stored, "[图片] 这是什么错误？");
        assert!(!stored.contains("base64data"));
    }

    #[test]
    fn test_optimize_storage_content_without_text() {
        let message = "[image:image/png;base64data]\n";
        let stored = optimize_storage_content(message);
        assert_eq!(stored, "[图片已附加]");
    }

    #[test]
    fn test_truncate_content_short() {
        let content = "短消息";
        let truncated = truncate_content(content, 800);
        assert_eq!(truncated, "短消息");
    }

    #[test]
    fn test_truncate_content_long() {
        let content = "a".repeat(1000);
        let truncated = truncate_content(&content, 800);
        // "…" is 3 bytes in UTF-8
        assert_eq!(truncated.len(), 803); // 800 + "…" (3 bytes)
        assert!(truncated.ends_with("…"));
    }

    #[test]
    fn test_conversation_with_image_attachment() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        // 用户发送图片 + 文本
        let attachment = Some(ImageAttachment::new("image/png", "base64data"));
        let message = process_image_message(attachment, "这个错误怎么解决？");
        let stored = optimize_storage_content(&message);
        
        manager.add_user_message(agent_id, stored);
        manager.add_assistant_message(agent_id, "这是一个常见的错误...".to_string());

        let history = manager.get_history(agent_id, 10);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "[图片] 这个错误怎么解决？");
        assert!(!history[0].content.contains("base64data"));
    }

    #[test]
    fn test_multi_agent_with_images() {
        let mut manager = AgentConversationManager::new();

        // Agent 1: 代码审查员
        let attachment1 = Some(ImageAttachment::new("image/png", "code_screenshot"));
        let message1 = process_image_message(attachment1, "这段代码有问题吗？");
        let stored1 = optimize_storage_content(&message1);
        manager.add_user_message("code-reviewer", stored1);
        manager.add_assistant_message("code-reviewer", "代码存在以下问题...".to_string());

        // Agent 2: 客服助手
        manager.add_user_message("customer-support", "如何重置密码？".to_string());
        manager.add_assistant_message("customer-support", "重置密码的步骤...".to_string());

        // 验证独立性
        let history1 = manager.get_history("code-reviewer", 10);
        let history2 = manager.get_history("customer-support", 10);

        assert_eq!(history1.len(), 2);
        assert_eq!(history2.len(), 2);
        assert!(history1[0].content.contains("[图片]"));
        assert!(!history2[0].content.contains("[图片]"));
    }

    #[test]
    fn test_conversation_context_window() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        // 添加 8 轮对话
        for i in 1..=8 {
            manager.add_user_message(agent_id, format!("问题{}", i));
            manager.add_assistant_message(agent_id, format!("回答{}", i));
        }

        // 获取最近 6 轮（实际实现中的限制）
        let history = manager.get_history(agent_id, 12); // 6 轮 = 12 条消息
        assert_eq!(history.len(), 12);

        // 验证是最近的 6 轮
        assert_eq!(history[0].content, "问题3");
        assert_eq!(history[11].content, "回答8");
    }

    #[test]
    fn test_content_truncation_in_history() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        // 添加长消息
        let long_message = "a".repeat(1000);
        manager.add_user_message(agent_id, long_message.clone());

        // 获取历史并截断
        let history = manager.get_history(agent_id, 10);
        let truncated = truncate_content(&history[0].content, 800);

        // "…" is 3 bytes in UTF-8
        assert_eq!(truncated.len(), 803);
        assert!(truncated.ends_with("…"));
    }

    #[test]
    fn test_empty_conversation_history() {
        let manager = AgentConversationManager::new();
        let history = manager.get_history("non-existent-agent", 10);
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_conversation_turn_order() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "agent-001";

        manager.add_user_message(agent_id, "第一条".to_string());
        manager.add_assistant_message(agent_id, "第二条".to_string());
        manager.add_user_message(agent_id, "第三条".to_string());
        manager.add_assistant_message(agent_id, "第四条".to_string());

        let history = manager.get_history(agent_id, 10);
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].content, "第一条");
        assert_eq!(history[1].content, "第二条");
        assert_eq!(history[2].content, "第三条");
        assert_eq!(history[3].content, "第四条");
    }

    #[test]
    fn test_image_mime_types() {
        let png = ImageAttachment::new("image/png", "data1");
        let jpg = ImageAttachment::new("image/jpeg", "data2");
        let webp = ImageAttachment::new("image/webp", "data3");

        assert!(png.to_message_prefix().contains("image/png"));
        assert!(jpg.to_message_prefix().contains("image/jpeg"));
        assert!(webp.to_message_prefix().contains("image/webp"));
    }

    #[test]
    fn test_complex_conversation_scenario() {
        let mut manager = AgentConversationManager::new();
        let agent_id = "security-auditor";

        // 第一轮：文本问题
        manager.add_user_message(agent_id, "请审计这个系统的安全性".to_string());
        manager.add_assistant_message(agent_id, "我将从以下几个方面进行审计...".to_string());

        // 第二轮：带图片的问题
        let attachment = Some(ImageAttachment::new("image/png", "security_scan_result"));
        let message = process_image_message(attachment, "这是扫描结果，有什么问题吗？");
        let stored = optimize_storage_content(&message);
        manager.add_user_message(agent_id, stored);
        manager.add_assistant_message(agent_id, "从扫描结果来看，发现以下漏洞...".to_string());

        // 第三轮：追问
        manager.add_user_message(agent_id, "如何修复这些漏洞？".to_string());
        manager.add_assistant_message(agent_id, "修复建议如下...".to_string());

        // 验证完整对话
        let history = manager.get_history(agent_id, 10);
        assert_eq!(history.len(), 6);
        assert_eq!(history[0].content, "请审计这个系统的安全性");
        assert!(history[2].content.contains("[图片]"));
        assert_eq!(history[4].content, "如何修复这些漏洞？");
    }
}
