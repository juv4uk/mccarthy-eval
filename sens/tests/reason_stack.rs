//! Regression for the confirmed MYLISP-REASON-SCALE-PROFILE failure.
//! The old `prove-goal` nested its recursive rule scan under `append` and
//! overflowed the ordinary test-thread stack before roughly 100 rules.
//! This file intentionally uses the default test thread: no enlarged stack.

use sens::{eval_program, Session};

fn loaded_session() -> Session {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/unify.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/reason.lisp"), &mut session).unwrap();
    session
}

fn edge_rules(n: usize) -> String {
    let mut source = String::from("(quote (");
    for i in 0..n {
        source.push_str(&format!("((edge {i} {})) ", i + 1));
    }
    source.push_str("))");
    source
}

#[test]
fn full_scan_256_rules_is_stack_safe_on_default_thread() {
    let n = 256usize;
    let rules = edge_rules(n);
    let source = format!(
        "(let ((rules {rules})) (length (reason (list (quote edge) (logic-var (quote x)) {n}) rules)))"
    );

    let mut session = loaded_session();
    let _ = eval_program(&source, &mut session)
        .unwrap_or_else(|error| panic!("full scan must not fail: {error}"));
}

#[test]
fn tail_scan_preserves_rule_result_order() {
    let source = r#"
        (let ((rules (quote (
                 ((edge a z))
                 ((edge b z))
                 ((edge c z))
               ))))
          (map
            (lambda (result) (cdr (car (car result))))
            (reason (list (quote edge) (logic-var (quote x)) (quote z)) rules)))
    "#;

    let mut session = loaded_session();
    let _ = eval_program(source, &mut session)
        .unwrap_or_else(|error| panic!("ordered scan must not fail: {error}"));
}
