//! Unit tests for Auto Test and Page Test functionality

#[cfg(test)]
mod tests {
    use crate::app::{AppMessage, OpenClawApp};
    use cosmic::Application;
    use tokio::runtime::Runtime;

    fn create_test_app() -> OpenClawApp {
        let rt = Runtime::new().expect("tokio runtime");
        let _guard = rt.enter();
        let (app, _task) = <OpenClawApp as Application>::init(cosmic::app::Core::default(), ());
        app
    }

    #[test]
    fn auto_test_start_is_idempotent() {
        let mut app = create_test_app();

        let _ = app.update(AppMessage::ClawRunAutoTest);
        let _ = app.update(AppMessage::ClawRunAutoTest);
    }

    #[test]
    fn page_test_start_is_idempotent() {
        let mut app = create_test_app();

        let _ = app.update(AppMessage::RunPageAutoTest);
        let _ = app.update(AppMessage::RunPageAutoTest);
    }

    #[test]
    fn auto_test_starts_with_zero_steps() {
        let mut app = create_test_app();
        assert_eq!(app.claw_auto_test_steps_done, 0);
        assert!(!app.claw_auto_test_running);

        let _ = app.update(AppMessage::ClawRunAutoTest);
        assert!(app.claw_auto_test_running);
        assert_eq!(app.claw_auto_test_steps_done, 0);
    }

    #[test]
    fn auto_test_step_updates_progress() {
        let mut app = create_test_app();
        let _ = app.update(AppMessage::ClawRunAutoTest);

        let _ = app.update(AppMessage::ClawAutoTestStepDone {
            step: 1,
            result: "Test step 1".to_string(),
            success: true,
        });
        assert_eq!(app.claw_auto_test_steps_done, 1);
        assert!(app.claw_auto_test_running);

        let _ = app.update(AppMessage::ClawAutoTestStepDone {
            step: 5,
            result: "Test step 5".to_string(),
            success: true,
        });
        assert_eq!(app.claw_auto_test_steps_done, 5);
        assert!(app.claw_auto_test_running);
    }

    #[test]
    fn auto_test_completes_at_step_10() {
        let mut app = create_test_app();
        let _ = app.update(AppMessage::ClawRunAutoTest);

        let _ = app.update(AppMessage::ClawAutoTestStepDone {
            step: 10,
            result: "Final step".to_string(),
            success: true,
        });
        assert_eq!(app.claw_auto_test_steps_done, 10);
        assert!(!app.claw_auto_test_running);
    }

    #[test]
    fn auto_test_stop_resets_state() {
        let mut app = create_test_app();
        let _ = app.update(AppMessage::ClawRunAutoTest);
        let _ = app.update(AppMessage::ClawAutoTestStepDone {
            step: 3,
            result: "Partial progress".to_string(),
            success: true,
        });

        let _ = app.update(AppMessage::ClawStopAutoTest);
        assert!(!app.claw_auto_test_running);
        assert_eq!(app.claw_auto_test_steps_done, 0);
    }

    #[test]
    fn auto_test_adds_history_entry() {
        let mut app = create_test_app();
        let initial_count = app.claw_history.len();

        let _ = app.update(AppMessage::ClawRunAutoTest);
        assert_eq!(app.claw_history.len(), initial_count + 1);

        if let Some(entry) = app.claw_history.last() {
            assert!(entry.command.contains("Auto Test"));
        }
    }

    #[test]
    fn page_test_starts_correctly() {
        let mut app = create_test_app();
        assert!(!app.page_auto_test_running);

        let _ = app.update(AppMessage::RunPageAutoTest);
        assert!(app.page_auto_test_running);
    }

    #[test]
    fn page_test_step_updates_progress() {
        let mut app = create_test_app();
        let _ = app.update(AppMessage::RunPageAutoTest);

        let _ = app.update(AppMessage::PageAutoTestStepDone {
            step: 1,
            result: "Page 1".to_string(),
            success: true,
        });
        assert!(app.page_auto_test_running);

        let _ = app.update(AppMessage::PageAutoTestStepDone {
            step: 3,
            result: "Page 3".to_string(),
            success: true,
        });
        assert!(app.page_auto_test_running);
    }

    #[test]
    fn page_test_completes_at_step_5() {
        let mut app = create_test_app();
        let _ = app.update(AppMessage::RunPageAutoTest);

        let _ = app.update(AppMessage::PageAutoTestStepDone {
            step: 5,
            result: "Final page".to_string(),
            success: true,
        });
        assert!(!app.page_auto_test_running);
    }

    #[test]
    fn page_test_adds_history_entry() {
        let mut app = create_test_app();
        let initial_count = app.claw_history.len();

        let _ = app.update(AppMessage::RunPageAutoTest);
        assert_eq!(app.claw_history.len(), initial_count + 1);

        if let Some(entry) = app.claw_history.last() {
            assert!(entry.command.contains("Page Test"));
        }
    }

    #[test]
    fn auto_test_step_ignores_when_not_running() {
        let mut app = create_test_app();
        assert!(!app.claw_auto_test_running);

        let _ = app.update(AppMessage::ClawAutoTestStepDone {
            step: 1,
            result: "Should be ignored".to_string(),
            success: true,
        });
        assert_eq!(app.claw_auto_test_steps_done, 0);
    }

    #[test]
    fn page_test_step_ignores_when_not_running() {
        let mut app = create_test_app();
        assert!(!app.page_auto_test_running);

        let _ = app.update(AppMessage::PageAutoTestStepDone {
            step: 1,
            result: "Should be ignored".to_string(),
            success: true,
        });
        assert!(!app.page_auto_test_running);
    }
}
