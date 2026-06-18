#[test]
fn fix_engine_is_conservative_in_the_skeleton() {
    let report = cargo_shed::fix::FixReport {
        applied: Vec::new(),
        skipped: Vec::new(),
        failed: false,
    };

    assert_eq!(report.to_human(), "No safe fixes are available yet.\n");
}
