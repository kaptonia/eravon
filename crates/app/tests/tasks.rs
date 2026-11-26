#[test]
fn every_task_kind_has_a_progress_slot() {
    for k in app::task::Kind::ALL {
        app::task::stage_at(k, 1);
        assert_eq!(app::task::stage(k).map(|s| s.at), Some(1), "{} has no progress slot", k.as_str());
    }
}
