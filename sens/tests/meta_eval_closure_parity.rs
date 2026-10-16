//! First code step of the evaluator ownership migration.
//! Keep the native evaluator as oracle and verify that the metacircular
//! evaluator preserves closure semantics.

use sens::{eval_program, Session};

fn native(expr: &str) -> String {
    let mut session = Session::default();
    eval_program(expr, &mut session)
        .expect("native evaluator should succeed")
        .value
        .to_string()
}

fn meta(expr: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session)
        .expect("core should load");
    sens::load_meta_evaluator_library(&mut session)
        .expect("meta evaluator should load");

    let source = format!("(my-eval (read \"{}\") (quote ()))", expr);
    eval_program(&source, &mut session)
        .expect("my-eval should succeed")
        .value
        .to_string()
}

#[test]
fn closure_capture_has_native_parity() {
    // Both sides are checked independently against an authored expected
    // value, never against each other — a bug shared by both evaluators
    // must not hide behind a native-vs-meta parity check that stays green.
    let expr = "((lambda (x) ((lambda (y) (+ x y)) 2)) 40)";
    let expected = "42";
    assert_eq!(native(expr), expected);
    assert_eq!(meta(expr), expected);
}
