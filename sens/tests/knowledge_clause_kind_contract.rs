//! #218 observer for Lisp-owned knowledge clause classification witnesses.
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
fn knowledge_clauses_report_explicit_domain_kind() {
    let source = include_str!("../../../tests/fixtures/knowledge-clause-kind-v1.lisp");
    let forms = parse(source).expect("knowledge-clause-kind-v1.lisp must parse");
    assert_eq!(forms.len(), 2, "#218 clause-kind slice must contain two rows");

    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).expect("core library");
    eval_program(include_str!("../../../lib/unify.lisp"), &mut session).expect("unify library");
    eval_program(include_str!("../../../lib/reason.lisp"), &mut session).expect("reason library");
    eval_program(include_str!("../../../lib/forward.lisp"), &mut session).expect("forward library");
    eval_program(include_str!("../../../lib/knowledge.lisp"), &mut session).expect("knowledge library");
    let witness = fs::read_to_string(repo_file("tests/fixtures/witness-runner.lisp"))
        .expect("Lisp-owned witness runner");
    eval_program(&witness, &mut session).expect("witness runner must load");

    for form in &forms {
        let ExprKind::List(entries) = &form.kind else {
            panic!("#218 clause-kind witness row must be an alist list");
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
            .expect("#218 clause-kind row must contain expr");

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
            .expect("#218 Lisp-owned clause-kind witness verdict")
            .value
            .to_string();

        assert_eq!(
            status, "pass",
            "#218 clause-kind witness rejected actual outcome for {expr}: {actual}"
        );
    }
}
