//! Storage crate integration tests using real SQLite file I/O.
//!
//! All tests operate on temporary on-disk SQLite databases (via `tempfile`),
//! exercising the full stack: Database open → schema migration → CRUD →
//! query → constraint validation.

use openclaw_storage::{
    Database, AgentStore, RunStore, AuditStore,
    RunRecord, RunStatus, StepRecord, StepKind,
    AuditEventRecord, AuditDecision,
    AgentManager, PlatformSummary,
};
use openclaw_security::{AgentProfile, AgentRole, AgentCapability};
use tempfile::tempdir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn open_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db = Database::open(&dir.path().join("platform.db")).unwrap();
    (db, dir)
}

fn make_profile(name: &str, owner: &str) -> AgentProfile {
    AgentProfile::new(name, AgentRole::TicketAssistant, owner, "admin")
}

// ── Database schema bootstrap ─────────────────────────────────────────────────

#[test]
fn db_opens_and_creates_real_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    assert!(!path.exists(), "db file should not exist before open");
    let db = Database::open(&path).unwrap();
    assert!(path.exists(), "db file must exist after open");
    assert_eq!(db.path, path);
}

#[test]
fn db_schema_idempotent_on_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("idem.db");
    // Open twice — second open must succeed with same schema
    let _db1 = Database::open(&path).unwrap();
    drop(_db1);
    let _db2 = Database::open(&path).unwrap();
    // If migration is not idempotent this will panic
}

#[test]
fn db_open_creates_parent_dirs() {
    let dir = tempdir().unwrap();
    let nested = dir.path().join("a").join("b").join("c").join("test.db");
    let _db = Database::open(&nested).unwrap();
    assert!(nested.exists(), "nested db file must be created");
}

// ── Agent store with real SQLite ──────────────────────────────────────────────

#[test]
fn agent_store_insert_and_get_real_db() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    let profile = make_profile("工单助手-实测-01", "acme");

    store.insert(&profile).unwrap();

    let loaded = store.get(profile.id.as_str()).unwrap().unwrap();
    assert_eq!(loaded.id,           profile.id);
    assert_eq!(loaded.display_name, "工单助手-实测-01");
    assert_eq!(loaded.owner,        "acme");
}

#[test]
fn agent_store_get_missing_returns_none() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    assert!(store.get("totally-fake-id-9999").unwrap().is_none());
}

#[test]
fn agent_store_update_persists_changes() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    let mut profile = make_profile("原始名称", "owner1");
    store.insert(&profile).unwrap();

    profile.display_name = "更新后名称".to_string();
    profile.touch();
    store.update(&profile).unwrap();

    let loaded = store.get(profile.id.as_str()).unwrap().unwrap();
    assert_eq!(loaded.display_name, "更新后名称");
}

#[test]
fn agent_store_update_nonexistent_returns_err() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    let profile = make_profile("幽灵", "nobody");
    // Not inserted → update must fail
    let err = store.update(&profile);
    assert!(err.is_err(), "updating non-existent agent must fail");
}

#[test]
fn agent_store_upsert_insert_then_update() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    let mut profile = make_profile("upsert-test", "owner");

    store.upsert(&profile).unwrap();
    assert_eq!(store.count().unwrap(), 1);

    profile.display_name = "upsert-v2".to_string();
    profile.touch();
    store.upsert(&profile).unwrap();
    assert_eq!(store.count().unwrap(), 1, "upsert must not create duplicate");

    let loaded = store.get(profile.id.as_str()).unwrap().unwrap();
    assert_eq!(loaded.display_name, "upsert-v2");
}

#[test]
fn agent_store_list_all_ordered_by_created_at_desc() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);

    for i in 0..5 {
        store.insert(&make_profile(&format!("助手-{}", i), "owner")).unwrap();
    }

    let all = store.list_all().unwrap();
    assert_eq!(all.len(), 5);
    // All must have non-empty IDs and display names
    for p in &all {
        assert!(!p.id.as_str().is_empty());
        assert!(!p.display_name.is_empty());
    }
}

#[test]
fn agent_store_list_by_owner_filters_correctly() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);

    store.insert(&make_profile("A1", "team-alpha")).unwrap();
    store.insert(&make_profile("A2", "team-alpha")).unwrap();
    store.insert(&make_profile("B1", "team-beta")).unwrap();

    let alpha = store.list_by_owner("team-alpha").unwrap();
    assert_eq!(alpha.len(), 2);
    let beta = store.list_by_owner("team-beta").unwrap();
    assert_eq!(beta.len(), 1);
    let none = store.list_by_owner("nonexistent").unwrap();
    assert!(none.is_empty());
}

#[test]
fn agent_store_archive_changes_status() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    let profile = make_profile("archive-target", "owner");
    store.insert(&profile).unwrap();

    store.archive(profile.id.as_str()).unwrap();

    let archived = store.list_by_status("archived").unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].id, profile.id);
}

#[test]
fn agent_store_archive_nonexistent_returns_err() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    assert!(store.archive("ghost-id").is_err());
}

#[test]
fn agent_store_capabilities_survive_json_roundtrip_in_sqlite() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);
    let mut profile = make_profile("capable-agent", "owner");
    profile.add_capability(AgentCapability::new("jira.create", "Create Jira ticket", 2));
    profile.add_capability(AgentCapability::new("slack.post",  "Post to Slack",       1));
    profile.add_capability(AgentCapability::new("github.pr",   "Open GitHub PR",      3));

    store.insert(&profile).unwrap();
    let loaded = store.get(profile.id.as_str()).unwrap().unwrap();

    assert_eq!(loaded.capabilities.len(), 3);
    let ids: Vec<&str> = loaded.capabilities.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&"jira.create"));
    assert!(ids.contains(&"slack.post"));
    assert!(ids.contains(&"github.pr"));
}

#[test]
fn agent_store_status_summary_correct_counts() {
    let (db, _dir) = open_db();
    let store = AgentStore::new(&db);

    store.insert(&make_profile("active-1", "o")).unwrap();
    store.insert(&make_profile("active-2", "o")).unwrap();
    let archived = make_profile("archived-1", "o");
    store.insert(&archived).unwrap();
    store.archive(archived.id.as_str()).unwrap();

    let summary = store.status_summary().unwrap();
    let total: u64 = summary.iter().map(|(_, c)| c).sum();
    assert_eq!(total, 3);

    // AgentStatus::Display returns "Active" / "Archived" (capital first letter)
    let active_count = summary.iter()
        .find(|(s, _)| s.eq_ignore_ascii_case("active"))
        .map(|(_, c)| *c).unwrap_or(0);
    assert_eq!(active_count, 2);
    let archived_count = summary.iter()
        .find(|(s, _)| s.eq_ignore_ascii_case("archived"))
        .map(|(_, c)| *c).unwrap_or(0);
    assert_eq!(archived_count, 1);
}

// ── Run store with real SQLite ────────────────────────────────────────────────

fn seed_agent_sqlite(db: &Database, id_hint: &str) -> String {
    let store = AgentStore::new(db);
    let profile = make_profile(id_hint, "owner");
    store.insert(&profile).unwrap();
    profile.id.as_str().to_string()
}

#[test]
fn run_store_create_and_get_real_db() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "run-test-agent");
    let store = RunStore::new(&db);

    let run = RunRecord::new(&agent_id, "Analyze Q4 financial report and generate summary");
    store.create_run(&run).unwrap();

    let loaded = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(loaded.id,               run.id);
    assert_eq!(loaded.agent_id,         agent_id);
    assert_eq!(loaded.task_description, "Analyze Q4 financial report and generate summary");
    assert_eq!(loaded.status,           RunStatus::Running);
    assert!(loaded.started_at > 0);
    assert!(loaded.finished_at.is_none());
}

#[test]
fn run_store_finish_run_success() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "finish-agent");
    let store = RunStore::new(&db);

    let run = RunRecord::new(&agent_id, "Generate weekly report");
    store.create_run(&run).unwrap();
    store.finish_run(&run.id, RunStatus::Success, Some("Report generated successfully")).unwrap();

    let loaded = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(loaded.status, RunStatus::Success);
    assert_eq!(loaded.summary.as_deref(), Some("Report generated successfully"));
    assert!(loaded.finished_at.is_some(), "finished_at must be set");
    assert!(loaded.finished_at.unwrap() >= loaded.started_at);
}

#[test]
fn run_store_finish_run_failed_with_no_summary() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "fail-agent");
    let store = RunStore::new(&db);

    let run = RunRecord::new(&agent_id, "risky task");
    store.create_run(&run).unwrap();
    store.finish_run(&run.id, RunStatus::Failed, None).unwrap();

    let loaded = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(loaded.status, RunStatus::Failed);
    assert!(loaded.summary.is_none());
}

#[test]
fn run_store_finish_run_nonexistent_returns_err() {
    let (db, _dir) = open_db();
    let store = RunStore::new(&db);
    let err = store.finish_run("no-such-run", RunStatus::Success, None);
    assert!(err.is_err(), "finishing non-existent run must fail");
}

#[test]
fn run_store_steps_complete_workflow() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "step-agent");
    let store = RunStore::new(&db);

    let run = RunRecord::new(&agent_id, "Multi-step workflow");
    store.create_run(&run).unwrap();

    // Step 0: inference (insert_step auto-increments step_count)
    let s0 = StepRecord::new(&run.id, 0, StepKind::Inference, "Calling LLM for plan");
    let s0_id = store.insert_step(&s0).unwrap();
    store.finish_step(s0_id, true, Some("Plan generated"), None).unwrap();

    // Step 1: tool call — file write
    let s1 = StepRecord::new(&run.id, 1, StepKind::ToolCall, "Writing output file");
    let s1_id = store.insert_step(&s1).unwrap();
    store.finish_step(s1_id, true, Some("File written"), None).unwrap();

    // Step 2: network request — failing
    let s2 = StepRecord::new(&run.id, 2, StepKind::NetworkRequest, "Fetching external API");
    let s2_id = store.insert_step(&s2).unwrap();
    store.finish_step(s2_id, false, None, Some("connection timeout")).unwrap();

    // Validate steps
    let steps = store.list_steps_for_run(&run.id).unwrap();
    assert_eq!(steps.len(), 3);
    assert!(steps[0].success, "inference step must succeed");
    assert!(steps[1].success, "file write step must succeed");
    assert!(!steps[2].success, "network step must fail");
    assert_eq!(steps[2].error.as_deref(), Some("connection timeout"));

    // Validate step_count on run (insert_step auto-increments)
    let loaded_run = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(loaded_run.step_count, 3);

    // Finish run
    store.finish_run(&run.id, RunStatus::Failed, Some("Step 2 timed out")).unwrap();
    let final_run = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(final_run.status, RunStatus::Failed);
    assert_eq!(final_run.summary.as_deref(), Some("Step 2 timed out"));
}

#[test]
fn run_store_list_running_excludes_finished() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "list-agent");
    let store = RunStore::new(&db);

    let r1 = RunRecord::new(&agent_id, "still running");
    let r2 = RunRecord::new(&agent_id, "already done");
    let r3 = RunRecord::new(&agent_id, "cancelled");
    store.create_run(&r1).unwrap();
    store.create_run(&r2).unwrap();
    store.create_run(&r3).unwrap();
    store.finish_run(&r2.id, RunStatus::Success, None).unwrap();
    store.finish_run(&r3.id, RunStatus::Cancelled, None).unwrap();

    let running = store.list_running().unwrap();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, r1.id);
}

#[test]
fn run_store_list_runs_for_agent_pagination() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "page-agent");
    let store = RunStore::new(&db);

    for i in 0..10 {
        store.create_run(&RunRecord::new(&agent_id, &format!("task-{}", i))).unwrap();
    }

    let page1 = store.list_runs_for_agent(&agent_id, 3).unwrap();
    assert_eq!(page1.len(), 3, "limit=3 must return 3 runs");

    let all = store.list_runs_for_agent(&agent_id, 100).unwrap();
    assert_eq!(all.len(), 10, "limit=100 must return all 10 runs");
}

#[test]
fn run_store_increment_denied_and_approved() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "security-agent");
    let store = RunStore::new(&db);
    let run = RunRecord::new(&agent_id, "security-sensitive task");
    store.create_run(&run).unwrap();

    store.increment_denied(&run.id).unwrap();
    store.increment_denied(&run.id).unwrap();
    store.increment_denied(&run.id).unwrap();
    store.increment_approved(&run.id).unwrap();

    let loaded = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(loaded.denied_count,   3);
    assert_eq!(loaded.approved_count, 1);
}

#[test]
fn run_store_step_kinds_all_preserved() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "kinds-agent");
    let store = RunStore::new(&db);
    let run = RunRecord::new(&agent_id, "step kinds test");
    store.create_run(&run).unwrap();

    let kinds = [
        StepKind::Inference,
        StepKind::ToolCall,
        StepKind::FileRead,
        StepKind::NetworkRequest,
        StepKind::ShellExec,
    ];
    for (i, kind) in kinds.iter().enumerate() {
        let step = StepRecord::new(&run.id, i as u32, kind.clone(), "testing kind");
        store.insert_step(&step).unwrap(); // auto-increments step_count
    }

    let steps = store.list_steps_for_run(&run.id).unwrap();
    assert_eq!(steps.len(), 5);
    for (step, expected_kind) in steps.iter().zip(kinds.iter()) {
        assert_eq!(step.kind, *expected_kind, "kind mismatch at index {}", step.step_index);
    }
}

// ── Audit store with real SQLite ──────────────────────────────────────────────

#[test]
fn audit_store_append_and_query() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "audit-agent");
    let store = AuditStore::new(&db);

    let run = RunRecord::new(&agent_id, "audit test");
    RunStore::new(&db).create_run(&run).unwrap();

    let event = AuditEventRecord::new(
        &agent_id,
        "shell_exec",
        AuditDecision::AutoDenied,
        "policy-engine",
    );
    let row_id = store.append(&event).unwrap();
    assert!(row_id > 0, "rowid must be positive");

    let results = store.query(&openclaw_storage::AuditFilter::for_agent(&agent_id)).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].agent_id, agent_id);
    assert_eq!(results[0].event_kind, "shell_exec");
    assert_eq!(results[0].decision, AuditDecision::AutoDenied);
}

#[test]
fn audit_store_is_append_only() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "append-agent");
    let store = AuditStore::new(&db);

    for i in 0..5 {
        let mut event = AuditEventRecord::new(
            &agent_id,
            "file_access",
            AuditDecision::AutoAllowed,
            "policy-engine",
        );
        event.step_index = Some(i);
        store.append(&event).unwrap();
    }

    let results = store.query(&openclaw_storage::AuditFilter::for_agent(&agent_id)).unwrap();
    assert_eq!(results.len(), 5, "all 5 events must be stored");
    // Verify step indices are preserved
    let indices: Vec<Option<u32>> = results.iter().map(|e| e.step_index).collect();
    for i in 0u32..5 {
        assert!(indices.contains(&Some(i)), "step_index {} must be present", i);
    }
}

#[test]
fn audit_store_filter_by_decision() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "filter-agent");
    let store = AuditStore::new(&db);

    let allowed = AuditEventRecord::new(&agent_id, "file_read", AuditDecision::AutoAllowed, "engine");
    let denied1 = AuditEventRecord::new(&agent_id, "shell_exec", AuditDecision::AutoDenied, "engine");
    let denied2 = AuditEventRecord::new(&agent_id, "network_req", AuditDecision::HumanDenied, "human");
    let approved = AuditEventRecord::new(&agent_id, "file_delete", AuditDecision::HumanApproved, "human");
    store.append(&allowed).unwrap();
    store.append(&denied1).unwrap();
    store.append(&denied2).unwrap();
    store.append(&approved).unwrap();

    let filter = openclaw_storage::AuditFilter {
        agent_id: Some(agent_id.clone()),
        decision: Some(AuditDecision::AutoDenied.as_str().to_string()),
        limit: 100,
        ..Default::default()
    };
    let auto_denied = store.query(&filter).unwrap();
    assert_eq!(auto_denied.len(), 1);
    assert_eq!(auto_denied[0].event_kind, "shell_exec");
}

#[test]
fn audit_store_count_for_agent() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "count-agent");
    let store = AuditStore::new(&db);

    for _ in 0..7 {
        store.append(&AuditEventRecord::new(
            &agent_id, "test_event", AuditDecision::AutoAllowed, "engine"
        )).unwrap();
    }

    let count = store.count_for_agent(&agent_id).unwrap();
    assert_eq!(count, 7);
}

// ── Full pipeline: Agent → Run → Steps → Audit (end-to-end) ──────────────────

#[test]
fn full_pipeline_agent_lifecycle_with_audit_trail() {
    let (db, _dir) = open_db();
    let agent_store = AgentStore::new(&db);
    let run_store   = RunStore::new(&db);
    let audit_store = AuditStore::new(&db);

    // 1. Create agent
    let profile = make_profile("E2E-测试员工", "engineering-team");
    agent_store.insert(&profile).unwrap();
    let agent_id = profile.id.as_str().to_string();

    // 2. Start a run
    let run = RunRecord::new(&agent_id, "分析竞品价格并生成报告");
    run_store.create_run(&run).unwrap();

    // 3. Audit: run started
    let start_event = AuditEventRecord::new(
        &agent_id, "run_started", AuditDecision::AutoAllowed, "agent-executor"
    );
    audit_store.append(&start_event).unwrap();

    // 4. Execute steps
    for (i, (kind, desc)) in [
        (StepKind::Inference,      "生成分析计划"),
        (StepKind::NetworkRequest, "抓取竞品官网"),
        (StepKind::Inference,      "对比价格数据"),
        (StepKind::ToolCall,       "生成 PDF 报告"),
    ].iter().enumerate() {
        let step = StepRecord::new(&run.id, i as u32, kind.clone(), *desc);
        let step_id = run_store.insert_step(&step).unwrap(); // auto-increments step_count
        run_store.finish_step(step_id, true, Some(&format!("步骤 {} 完成", i)), None).unwrap();

        // Audit each step
        let mut step_event = AuditEventRecord::new(
            &agent_id, "step_completed", AuditDecision::AutoAllowed, "policy-engine"
        );
        step_event.step_index = Some(i as u32);
        audit_store.append(&step_event).unwrap();
    }

    // 5. Deny one sensitive action
    run_store.increment_denied(&run.id).unwrap();
    let denied_event = AuditEventRecord::new(
        &agent_id, "sensitive_file_access", AuditDecision::AutoDenied, "policy-engine"
    );
    audit_store.append(&denied_event).unwrap();

    // 6. Finish run
    run_store.finish_run(&run.id, RunStatus::Success, Some("报告已生成")).unwrap();

    // 7. Validate entire pipeline
    let final_run = run_store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(final_run.status,      RunStatus::Success);
    assert_eq!(final_run.step_count,  4);
    assert_eq!(final_run.denied_count, 1);
    assert_eq!(final_run.summary.as_deref(), Some("报告已生成"));

    let steps = run_store.list_steps_for_run(&run.id).unwrap();
    assert_eq!(steps.len(), 4);
    assert!(steps.iter().all(|s| s.success), "all steps must succeed");

    // Audit trail: 1 start + 4 steps + 1 denied = 6 events
    let all_events = audit_store.query(&openclaw_storage::AuditFilter::for_agent(&agent_id)).unwrap();
    assert_eq!(all_events.len(), 6, "audit trail must have exactly 6 events");

    let denied_events: Vec<_> = all_events.iter()
        .filter(|e| e.decision == AuditDecision::AutoDenied)
        .collect();
    assert_eq!(denied_events.len(), 1);
    assert_eq!(denied_events[0].event_kind, "sensitive_file_access");

    // Count verification
    assert_eq!(audit_store.count_for_agent(&agent_id).unwrap(), 6);
}

#[test]
fn full_pipeline_multi_agent_isolation() {
    let (db, _dir) = open_db();
    let agent_store = AgentStore::new(&db);
    let run_store   = RunStore::new(&db);
    let audit_store = AuditStore::new(&db);

    // Create two separate agents
    let agent_a = make_profile("工单助手-A", "hr-team");
    let agent_b = make_profile("财务分析-B", "finance-team");
    agent_store.insert(&agent_a).unwrap();
    agent_store.insert(&agent_b).unwrap();

    let id_a = agent_a.id.as_str().to_string();
    let id_b = agent_b.id.as_str().to_string();

    // Agent A: 3 runs
    for i in 0..3 {
        let run = RunRecord::new(&id_a, &format!("工单任务-{}", i));
        run_store.create_run(&run).unwrap();
        run_store.finish_run(&run.id, RunStatus::Success, None).unwrap();
        audit_store.append(&AuditEventRecord::new(
            &id_a, "run_completed", AuditDecision::AutoAllowed, "engine"
        )).unwrap();
    }

    // Agent B: 2 runs (one failed)
    let run_b1 = RunRecord::new(&id_b, "财务报表分析");
    run_store.create_run(&run_b1).unwrap();
    run_store.finish_run(&run_b1.id, RunStatus::Success, None).unwrap();

    let run_b2 = RunRecord::new(&id_b, "外部 API 调用");
    run_store.create_run(&run_b2).unwrap();
    run_store.finish_run(&run_b2.id, RunStatus::Failed, Some("API timeout")).unwrap();

    // Validate isolation
    let runs_a = run_store.list_runs_for_agent(&id_a, 100).unwrap();
    assert_eq!(runs_a.len(), 3, "agent A must have 3 runs");

    let runs_b = run_store.list_runs_for_agent(&id_b, 100).unwrap();
    assert_eq!(runs_b.len(), 2, "agent B must have 2 runs");

    let events_a = audit_store.count_for_agent(&id_a).unwrap();
    assert_eq!(events_a, 3, "agent A must have 3 audit events");

    let events_b = audit_store.count_for_agent(&id_b).unwrap();
    assert_eq!(events_b, 0, "agent B must have 0 audit events");

    // Total runs in the system
    assert_eq!(run_store.count_runs().unwrap(), 5);
}

#[test]
fn full_pipeline_run_with_cancelled_status() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "cancel-agent");
    let store = RunStore::new(&db);

    let run = RunRecord::new(&agent_id, "long running task");
    store.create_run(&run).unwrap();

    // Partially execute some steps
    let s = StepRecord::new(&run.id, 0, StepKind::Inference, "started planning");
    let s_id = store.insert_step(&s).unwrap(); // auto-increments step_count
    store.finish_step(s_id, true, None, None).unwrap();

    // Cancel before completion
    store.finish_run(&run.id, RunStatus::Cancelled, Some("User cancelled")).unwrap();

    let final_run = store.get_run(&run.id).unwrap().unwrap();
    assert_eq!(final_run.status, RunStatus::Cancelled);
    assert!(RunStatus::Cancelled.is_terminal());
    assert_eq!(final_run.step_count, 1);
}

// ── AuditFilter edge cases ────────────────────────────────────────────────────

#[test]
fn audit_filter_empty_returns_all() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "filter-all-agent");
    let store = AuditStore::new(&db);

    for _ in 0..8 {
        store.append(&AuditEventRecord::new(
            &agent_id, "event", AuditDecision::AutoAllowed, "engine"
        )).unwrap();
    }

    let all = store.query(&openclaw_storage::AuditFilter::new()).unwrap();
    assert_eq!(all.len(), 8);
}

#[test]
fn audit_filter_limit_respected() {
    let (db, _dir) = open_db();
    let agent_id = seed_agent_sqlite(&db, "limit-agent");
    let store = AuditStore::new(&db);

    for _ in 0..20 {
        store.append(&AuditEventRecord::new(
            &agent_id, "event", AuditDecision::AutoAllowed, "engine"
        )).unwrap();
    }

    let filter = openclaw_storage::AuditFilter { limit: 5, ..Default::default() };
    let limited = store.query(&filter).unwrap();
    assert_eq!(limited.len(), 5, "limit=5 must return exactly 5 events");
}
