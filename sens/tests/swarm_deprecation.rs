use sens::{eval_program, Session};

const DEPRECATION: &str = include_str!("../../../knowledge/swarm-legacy-deprecation.lisp");
const NO_LIVE_CALLERS_AUDIT: &str =
    include_str!("../../../knowledge/swarm-no-live-callers-audit.lisp");

const RETIRED_COORDINATION_OPS: &[&str] = &[
    "hello",
    "heartbeat",
    "claim",
    "release",
    "complete-task",
    "define-task",
    "validate-tasks",
    "sync-tasks",
    "sync-milestone",
    "next-best-action",
    "list-task-state",
    "list-tasks",
    "presence",
    "list-claims",
    "capability-request",
    "subscribe",
    "publish",
    "notify",
    "poll",
];

#[test]
fn legacy_coordination_deprecation_records_retired_physical_surface() {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    eval_program(DEPRECATION, &mut session).unwrap();

    let value = session
        .environment
        .get("*swarm-legacy-coordination*")
        .expect("deprecation marker should define machine-readable migration data")
        .to_string();

    assert!(value.contains("(status . deprecated)"), "{value}");
    assert!(value.contains("(physical-status . removed)"), "{value}");
    assert!(
        value.contains("(coordination-authority . swarm-node)"),
        "{value}"
    );
    assert!(value.contains("(runtime-rejection . confirmed)"), "{value}");
    assert!(
        DEPRECATION.contains("legacy_coordination_rejected.rs"),
        "machine marker must point to the runtime rejection witness"
    );
    for op in RETIRED_COORDINATION_OPS {
        assert!(value.contains(op), "retired operation {op} is missing: {value}");
    }
}

#[test]
fn no_live_callers_audit_records_physical_removal() {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    eval_program(NO_LIVE_CALLERS_AUDIT, &mut session).unwrap();

    let value = session
        .environment
        .get("*swarm-no-live-callers-audit*")
        .expect("no-live-callers audit should be executable machine-readable data")
        .to_string();

    assert!(
        value.contains("(schema . swarm-no-live-callers-audit/1)"),
        "{value}"
    );
    assert!(value.contains("(scope . ecosystem)"), "{value}");
    assert!(value.contains("(status . removed)"), "{value}");
    assert!(
        NO_LIVE_CALLERS_AUDIT.contains("(safe-to-remove . t)"),
        "the historical removal gate must remain recorded as open"
    );
    assert!(
        value.contains("(my-lisp-production-operational . confirmed)"),
        "{value}"
    );
    assert!(
        NO_LIVE_CALLERS_AUDIT.contains("(my-lisp-production-operational-callers . ())"),
        "production caller inventory must remain empty"
    );
    assert!(
        NO_LIVE_CALLERS_AUDIT.contains("(my-lisp-compatibility-callers . ())"),
        "compatibility callers must be gone with the physical surface"
    );
    assert!(NO_LIVE_CALLERS_AUDIT.contains("(blockers . ())"));
    assert!(NO_LIVE_CALLERS_AUDIT.contains("(legacy-compatibility-tests . ())"));
    assert!(NO_LIVE_CALLERS_AUDIT.contains("(physical-surface-still-present . ())"));
    assert!(NO_LIVE_CALLERS_AUDIT.contains("physical-removal-commit . 32a087f"));
    assert!(NO_LIVE_CALLERS_AUDIT.contains("swarm_live_caller_inventory.rs"));
    assert!(NO_LIVE_CALLERS_AUDIT.contains("legacy_coordination_rejected.rs"));
    assert!(NO_LIVE_CALLERS_AUDIT.contains("semantic_oracle_preservation.rs"));
}

// current_agent_authority_records_removed_legacy_coordination and
// human_migration_doc_keeps_semantic_and_coordination_planes_separate were
// pure markdown/doc-text checks, relocated to `cargo xtask verify` per
// TEST-ARCHITECTURE-1 step 4 — see crates/xtask/src/checks.rs.
