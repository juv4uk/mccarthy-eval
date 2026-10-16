//! B1 boundary ordering for knowledge-module observations: validate the input
//! before deciding that a module is merely absent.

use sens::{eval_program, Session};

fn observe_in(source: &str) -> String {
    let mut session = Session::default();
    for library in [
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/forward.lisp"),
        include_str!("../../../lib/knowledge.lisp"),
        include_str!("../../../lib/result-status.lisp"),
    ] {
        eval_program(library, &mut session).unwrap();
    }
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn malformed_module_name_is_invalid_not_unknown() {
    assert_eq!(
        observe_in(r#"(reason-in-observe 42 (quote (planet earth)))"#),
        "(invalid invalid-module 42)"
    );
}

#[test]
fn malformed_goal_is_invalid_even_when_module_is_missing() {
    assert_eq!(
        observe_in(r#"(reason-in-observe (quote missing) (quote (not?)))"#),
        "(invalid invalid-goal (not?))"
    );
}
