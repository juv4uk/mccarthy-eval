//! #219 observer for Lisp-owned reasoning-honesty witnesses.
//! Rust transports evaluator results; Lisp fixtures + witness logic own the
//! semantic expectations.

use sens::{eval_program, parse, ExprKind, Session};
use std::fs;
use std::path::PathBuf;

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
}

fn escape_lisp_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn assert_one_row_fixture(source: &str, libraries: &[(&str, &str)], label: &str) {
    let forms = parse(source).unwrap_or_else(|error| panic!("{label} fixture must parse: {error}"));
    assert_eq!(forms.len(), 1, "{label} fixture must contain one row");

    let ExprKind::List(entries) = &forms[0].kind else {
        panic!("{label} witness row must be an alist list");
    };
    let expr = entries
        .iter()
        .find_map(|entry| {
            let ExprKind::Pair(key, value) = &entry.kind else {
                return None;
            };
            let ExprKind::Symbol(key) = &key.kind else {
                return None;
            };
            if &**key != "expr" {
                return None;
            }
            let ExprKind::String(value) = &value.kind else {
                return None;
            };
            Some(value.to_string())
        })
        .unwrap_or_else(|| panic!("{label} row must contain expr"));

    let row_source = &source[forms[0].span.start..forms[0].span.end];

    let mut session = Session::default();
    for (name, library) in libraries {
        eval_program(library, &mut session)
            .unwrap_or_else(|error| panic!("{label}: {name} library must load: {error}"));
    }
    let witness = fs::read_to_string(repo_file("tests/fixtures/witness-runner.lisp"))
        .expect("Lisp-owned witness runner");
    eval_program(&witness, &mut session).expect("witness runner must load");

    let actual = match eval_program(&expr, &mut session) {
        Ok(result) => format!("(value \"{}\")", escape_lisp_string(&result.value.to_string())),
        Err(error) => format!("(error \"{:?}\")", error.kind),
    };
    let verdict = format!(
        "(witness-status (witness-verdict (quote {}) (quote {})))",
        row_source, actual
    );
    let status = eval_program(&verdict, &mut session)
        .unwrap_or_else(|error| panic!("{label}: Lisp-owned witness verdict failed: {error}"))
        .value
        .to_string();

    assert_eq!(
        status, "pass",
        "{label} Lisp-owned witness rejected actual outcome for {expr}: {actual}"
    );
}

#[test]
fn absence_of_proof_does_not_fabricate_negation() {
    assert_one_row_fixture(
        include_str!("../../../tests/fixtures/reason-honesty-v1.lisp"),
        &[
            ("core", include_str!("../../../lib/core.lisp")),
            ("unify", include_str!("../../../lib/unify.lisp")),
            ("reason", include_str!("../../../lib/reason.lisp")),
        ],
        "#219 no-proof-is-not-negation",
    );
}

#[test]
fn missing_module_is_blocked_by_established_precondition() {
    assert_one_row_fixture(
        include_str!("../../../tests/fixtures/reason-module-honesty-v1.lisp"),
        &[
            ("core", include_str!("../../../lib/core.lisp")),
            ("unify", include_str!("../../../lib/unify.lisp")),
            ("reason", include_str!("../../../lib/reason.lisp")),
            ("forward", include_str!("../../../lib/forward.lisp")),
            ("knowledge", include_str!("../../../lib/knowledge.lisp")),
            ("result-status", include_str!("../../../lib/result-status.lisp")),
        ],
        "#219 missing-module-is-blocked",
    );
}

#[test]
fn absence_of_reasoning_evidence_does_not_fabricate_unknown() {
    assert_one_row_fixture(
        include_str!("../../../tests/fixtures/reason-observe-honesty-v1.lisp"),
        &[
            ("core", include_str!("../../../lib/core.lisp")),
            ("unify", include_str!("../../../lib/unify.lisp")),
            ("reason", include_str!("../../../lib/reason.lisp")),
            ("result-status", include_str!("../../../lib/result-status.lisp")),
        ],
        "#219 no-evidence-is-not-unknown",
    );
}
