//! RAG 实用案例集成测试
//! 
//! 测试真实使用场景下的 RAG 配置管理功能

use openclaw_config::{RagConfig, RagFolder, RagFile, RagSettings, IndexingStatus};
use std::path::PathBuf;
use std::time::SystemTime;

/// 案例1: 用户添加项目文档文件夹作为知识库
#[test]
fn case1_add_project_docs_folder() {
    let mut config = RagConfig::default();
    
    // 用户添加项目文档目录
    let docs_folder = RagFolder::new(
        PathBuf::from("/Users/dev/my-project/docs"),
        "Project Documentation"
    );
    
    assert!(config.add_folder(docs_folder));
    assert_eq!(config.folders.len(), 1);
    
    // 配置文件夹：只索引 markdown 和 PDF
    config.folders[0].include_extensions = vec!["md".to_string(), "pdf".to_string()];
    config.folders[0].watch_enabled = true;
    config.folders[0].allow_agent_write = false;
    
    // 验证配置
    assert!(config.folders[0].should_index_file(&PathBuf::from("/Users/dev/my-project/docs/README.md")));
    assert!(config.folders[0].should_index_file(&PathBuf::from("/Users/dev/my-project/docs/api.pdf")));
    assert!(!config.folders[0].should_index_file(&PathBuf::from("/Users/dev/my-project/docs/image.png")));
    
    println!("✓ 案例1: 成功添加项目文档文件夹，配置扩展名过滤和监控");
}

/// 案例2: 用户添加多个知识库文件夹（技术文档 + 会议记录）
#[test]
fn case2_multiple_knowledge_bases() {
    let mut config = RagConfig::default();
    
    // 添加技术文档文件夹
    let tech_docs = RagFolder::new(
        PathBuf::from("/Users/dev/tech-docs"),
        "Technical Documentation"
    );
    assert!(config.add_folder(tech_docs));
    
    // 添加会议记录文件夹
    let meeting_notes = RagFolder::new(
        PathBuf::from("/Users/dev/meetings"),
        "Meeting Notes"
    );
    assert!(config.add_folder(meeting_notes));
    
    // 尝试添加重复路径（应该被拒绝）
    let duplicate = RagFolder::new(
        PathBuf::from("/Users/dev/tech-docs"),
        "Tech Docs Duplicate"
    );
    assert!(!config.add_folder(duplicate));
    
    assert_eq!(config.folders.len(), 2);
    assert_eq!(config.folders[0].name, "Technical Documentation");
    assert_eq!(config.folders[1].name, "Meeting Notes");
    
    println!("✓ 案例2: 成功管理多个知识库文件夹，重复路径被正确拒绝");
}

/// 案例3: 用户添加单个重要文档（合同、规范等）
#[test]
fn case3_add_individual_important_documents() {
    let mut config = RagConfig::default();
    
    // 添加公司合同文档
    let contract = RagFile::new(
        PathBuf::from("/Users/legal/contract-2024.pdf"),
        "2024 Service Contract",
        "pdf"
    );
    assert!(config.add_file(contract));
    
    // 添加技术规范文档
    let spec = RagFile::new(
        PathBuf::from("/Users/specs/api-v2-spec.md"),
        "API v2 Specification",
        "md"
    );
    assert!(config.add_file(spec));
    
    // 设置优先级和标签
    config.files[0].priority = 10; // 高优先级
    config.files[0].tags = vec!["legal".to_string(), "contract".to_string()];
    
    config.files[1].priority = 8;
    config.files[1].tags = vec!["api".to_string(), "specification".to_string()];
    
    assert_eq!(config.files.len(), 2);
    assert!(config.files[0].enabled);
    assert_eq!(config.files[0].priority, 10);
    
    println!("✓ 案例3: 成功添加单个重要文档，配置优先级和标签");
}

/// 案例4: 用户调整 RAG 设置以优化性能
#[test]
fn case4_customize_rag_settings() {
    let mut config = RagConfig::default();
    
    // 默认设置
    assert_eq!(config.settings.chunk_size, 1000);
    assert_eq!(config.settings.chunk_overlap, 200);
    assert!(config.settings.auto_index_enabled);
    
    // 用户调整设置：更小的 chunk 以提高精度
    let mut new_settings = RagSettings::default();
    new_settings.chunk_size = 512;
    new_settings.chunk_overlap = 100;
    new_settings.max_total_size_mb = Some(2048); // 2GB 限制
    new_settings.ocr_enabled = true;
    new_settings.auto_index_enabled = true;
    
    config.update_settings(new_settings);
    
    // 验证更新
    assert_eq!(config.settings.chunk_size, 512);
    assert_eq!(config.settings.chunk_overlap, 100);
    assert_eq!(config.settings.max_total_size_mb, Some(2048));
    assert!(config.settings.ocr_enabled);
    
    println!("✓ 案例4: 成功调整 RAG 设置以优化性能");
}

/// 案例5: 文件索引状态追踪（模拟索引过程）
#[test]
fn case5_file_indexing_lifecycle() {
    let mut config = RagConfig::default();
    
    // 添加文档
    let mut doc = RagFile::new(
        PathBuf::from("/docs/guide.pdf"),
        "User Guide",
        "pdf"
    );
    
    // 初始状态：待索引
    assert_eq!(doc.indexing_status, IndexingStatus::Pending);
    assert!(!doc.indexing_status.is_indexing());
    assert!(!doc.indexing_status.is_successfully_indexed());
    
    // 开始索引
    doc.indexing_status = IndexingStatus::Indexing {
        progress: 50,
        operation: "Extracting text from PDF".to_string(),
        started_at: SystemTime::now(),
    };
    assert!(doc.indexing_status.is_indexing());
    assert_eq!(doc.get_indexing_progress(), 50);
    
    // 索引完成
    doc.indexing_status = IndexingStatus::Indexed {
        completed_at: SystemTime::now(),
        chunk_count: 42,
        duration_ms: 1500,
    };
    assert!(doc.indexing_status.is_successfully_indexed());
    assert_eq!(doc.get_indexing_progress(), 100);
    assert_eq!(doc.get_status_description(), "Indexed");
    
    // 内容变更，需要重新索引
    doc.indexing_status = IndexingStatus::NeedsReindex {
        reason: "File modified".to_string(),
        detected_at: SystemTime::now(),
    };
    assert!(doc.needs_reindexing());
    
    config.add_file(doc);
    assert_eq!(config.files.len(), 1);
    
    println!("✓ 案例5: 成功追踪文件索引生命周期状态");
}

/// 案例6: 配置持久化与恢复（完整工作流）
#[test]
fn case6_save_and_restore_workflow() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("rag.toml");
    
    // 第一步：用户配置知识库
    let mut config = RagConfig::default();
    
    // 添加文件夹
    config.add_folder(RagFolder::new(
        PathBuf::from("/work/projects"),
        "Work Projects"
    ));
    
    // 添加文件
    config.add_file(RagFile::new(
        PathBuf::from("/docs/important.pdf"),
        "Important Document",
        "pdf"
    ));
    
    // 自定义设置
    let mut settings = RagSettings::default();
    settings.chunk_size = 800;
    settings.max_total_size_mb = Some(512);
    config.update_settings(settings);
    
    // 保存配置
    let toml_content = toml::to_string_pretty(&config).expect("serialize");
    std::fs::write(&config_path, toml_content).expect("write config");
    
    // 第二步：应用重启，恢复配置
    let restored_content = std::fs::read_to_string(&config_path).expect("read config");
    let restored: RagConfig = toml::from_str(&restored_content).expect("deserialize");
    
    // 验证恢复的配置
    assert_eq!(restored.folders.len(), 1);
    assert_eq!(restored.folders[0].name, "Work Projects");
    assert_eq!(restored.files.len(), 1);
    assert_eq!(restored.files[0].name, "Important Document");
    assert_eq!(restored.settings.chunk_size, 800);
    assert_eq!(restored.settings.max_total_size_mb, Some(512));
    
    println!("✓ 案例6: 成功完成配置持久化与恢复工作流");
}

/// 案例7: 文件夹 Watch 和 Write 权限管理
#[test]
fn case7_folder_permissions_management() {
    let mut config = RagConfig::default();
    
    // 只读知识库（技术文档）
    let mut readonly_kb = RagFolder::new(
        PathBuf::from("/company/tech-docs"),
        "Company Tech Docs (Read-Only)"
    );
    readonly_kb.watch_enabled = true;
    readonly_kb.allow_agent_write = false;
    config.add_folder(readonly_kb);
    
    // 可写工作区（Agent 可以创建文件）
    let mut writable_workspace = RagFolder::new(
        PathBuf::from("/workspace/agent-notes"),
        "Agent Workspace (Writable)"
    );
    writable_workspace.watch_enabled = true;
    writable_workspace.allow_agent_write = true;
    config.add_folder(writable_workspace);
    
    // 验证权限设置
    assert!(config.folders[0].watch_enabled);
    assert!(!config.folders[0].allow_agent_write);
    
    assert!(config.folders[1].watch_enabled);
    assert!(config.folders[1].allow_agent_write);
    
    println!("✓ 案例7: 成功管理文件夹的 Watch 和 Write 权限");
}

/// 案例8: 移除和禁用文档
#[test]
fn case8_remove_and_disable_documents() {
    let mut config = RagConfig::default();
    
    // 添加3个文档
    config.add_file(RagFile::new(PathBuf::from("/docs/a.pdf"), "Doc A", "pdf"));
    config.add_file(RagFile::new(PathBuf::from("/docs/b.pdf"), "Doc B", "pdf"));
    config.add_file(RagFile::new(PathBuf::from("/docs/c.pdf"), "Doc C", "pdf"));
    assert_eq!(config.files.len(), 3);
    
    // 禁用第二个文档（暂时不用，但保留配置）
    config.files[1].enabled = false;
    assert!(!config.files[1].enabled);
    
    // 移除第一个文档（永久删除）
    let removed = config.remove_file(0);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "Doc A");
    assert_eq!(config.files.len(), 2);
    
    // 现在 index 0 是原来的 Doc B（已禁用），index 1 是 Doc C（启用）
    assert_eq!(config.files[0].name, "Doc B");
    assert!(!config.files[0].enabled);
    assert_eq!(config.files[1].name, "Doc C");
    assert!(config.files[1].enabled);
    
    println!("✓ 案例8: 成功移除和禁用文档");
}

/// 案例9: 大规模知识库配置（性能测试）
#[test]
fn case9_large_scale_knowledge_base() {
    let mut config = RagConfig::default();
    
    // 添加10个文件夹
    for i in 0..10 {
        let folder = RagFolder::new(
            PathBuf::from(format!("/kb/folder-{}", i)),
            &format!("Knowledge Base {}", i)
        );
        assert!(config.add_folder(folder));
    }
    
    // 添加100个文件
    for i in 0..100 {
        let file = RagFile::new(
            PathBuf::from(format!("/docs/doc-{}.pdf", i)),
            &format!("Document {}", i),
            "pdf"
        );
        assert!(config.add_file(file));
    }
    
    assert_eq!(config.folders.len(), 10);
    assert_eq!(config.files.len(), 100);
    
    // 验证序列化性能（应该很快）
    let start = std::time::Instant::now();
    let toml_str = toml::to_string_pretty(&config).expect("serialize");
    let serialize_duration = start.elapsed();
    
    // 验证反序列化性能
    let start = std::time::Instant::now();
    let restored: RagConfig = toml::from_str(&toml_str).expect("deserialize");
    let deserialize_duration = start.elapsed();
    
    assert_eq!(restored.folders.len(), 10);
    assert_eq!(restored.files.len(), 100);
    
    println!("✓ 案例9: 大规模知识库配置测试通过");
    println!("  - 序列化耗时: {:?}", serialize_duration);
    println!("  - 反序列化耗时: {:?}", deserialize_duration);
    println!("  - TOML 大小: {} bytes", toml_str.len());
}

/// 案例10: 错误处理和边界条件
#[test]
fn case10_error_handling_and_edge_cases() {
    let mut config = RagConfig::default();
    
    // 边界1: 移除不存在的索引
    assert!(config.remove_file(999).is_none());
    assert!(config.remove_folder(999).is_none());
    
    // 边界2: 空扩展名列表（索引所有文件）
    let mut folder = RagFolder::new(PathBuf::from("/all"), "All Files");
    folder.include_extensions.clear();
    assert!(folder.should_index_file(&PathBuf::from("/all/any.xyz")));
    config.add_folder(folder);
    
    // 边界3: 无限制大小设置
    let mut settings = RagSettings::default();
    settings.max_total_size_mb = None;
    settings.max_file_size_mb = None;
    config.update_settings(settings);
    assert!(config.settings.max_total_size_mb.is_none());
    
    // 边界4: 最小 chunk 设置
    let mut settings = RagSettings::default();
    settings.chunk_size = 64;
    settings.chunk_overlap = 0;
    config.update_settings(settings);
    assert_eq!(config.settings.chunk_size, 64);
    assert_eq!(config.settings.chunk_overlap, 0);
    
    println!("✓ 案例10: 错误处理和边界条件测试通过");
}
