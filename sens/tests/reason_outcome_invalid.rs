//! Adversarial B1 validation: malformed goal syntax is an `invalid`
//! observation, never logical `unknown`.

use sens::{eval_program, Session};

fn observe(source: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/unify.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/reason.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/result-status.lisp"), &mut session).unwrap();
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn not_without_nested_goal_is_invalid() {
    assert_eq!(
        observe(r#"(reason-observe (quote (not?)) (quote ()))"#),
        "(invalid invalid-goal (not?))"
    );
}

#[test]
fn not_with_multiple_nested_forms_is_invalid() {
    assert_eq!(
        observe(r#"(reason-observe (quote (not? (planet earth) extra)) (quote ()))"#),
        "(invalid invalid-goal (not? (planet earth) extra))"
    );
}

#[test]
fn non_symbol_predicate_head_is_invalid() {
    assert_eq!(
        observe(r#"(reason-observe (quote (42 earth)) (quote ()))"#),
        "(invalid invalid-goal (42 earth))"
    );
}
