//! #219 observer for Lisp-owned unification outcome witnesses.
//! Rust transports evaluator results only; Lisp fixture rows own expectations.

use sens::{eval_program, parse, ExprKind, Session};
use std::fs;
use std::path::PathBuf;

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
}

fn escape_lisp_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[test]
fn completed_unification_queries_report_explicit_outcomes() {
    let source = include_str!("../../../tests/fixtures/unification-outcome-v1.lisp");
    let forms = parse(source).expect("unification-outcome-v1.lisp must parse");
    assert_eq!(forms.len(), 4, "#219 unification slice must contain four rows");

    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).expect("core library");
    eval_program(include_str!("../../../lib/unify.lisp"), &mut session).expect("unify library");
    eval_program(include_str!("../../../lib/unify-observe.lisp"), &mut session)
        .expect("unification observation layer");
    let witness = fs::read_to_string(repo_file("tests/fixtures/witness-runner.lisp"))
        .expect("Lisp-owned witness runner");
    eval_program(&witness, &mut session).expect("witness runner must load");

    for form in &forms {
        let ExprKind::List(entries) = &form.kind else {
            panic!("#219 unification witness row must be an alist list");
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
            .expect("#219 unification row must contain expr");

        let row_source = &source[form.span.start..form.span.end];
        let actual = match eval_program(&expr, &mut session) {
            Ok(result) => format!("(value \"{}\")", escape_lisp_string(&result.value.to_string())),
            Err(error) => format!("(error \"{:?}\")", error.kind),
        };
        let verdict = format!(
            "(witness-status (witness-verdict (quote {}) (quote {})))",
            row_source, actual
        );
        let status = eval_program(&verdict, &mut session)
            .expect("#219 Lisp-owned unification witness verdict")
            .value
            .to_string();

        assert_eq!(
            status, "pass",
            "#219 unification witness rejected actual outcome for {expr}: {actual}"
        );
    }
}

#[test]
fn reasoning_consumers_do_not_branch_on_private_fail_sentinel() {
    for (name, source) in [
        ("reason", include_str!("../../../lib/reason.lisp")),
        ("forward", include_str!("../../../lib/forward.lisp")),
    ] {
        assert!(
            !source.contains("(failed-subst?"),
            "#219 {name} must consume explicit unification outcomes, not branch on failed-subst?"
        );
        assert!(
            source.contains("(unify-observe"),
            "#219 {name} must consume the Lisp-owned unify-observe boundary"
        );
    }
}
