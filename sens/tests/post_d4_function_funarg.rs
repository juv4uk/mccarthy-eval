//! #2258 — FUNCTION/FUNARG historical-mechanism witness.
//!
//! Lisp 1.5 FUNCTION/FUNARG is treated here as a historical closure
//! representation mechanism. The observable closure behavior must already be
//! supplied by D4 LAMBDA without FUNCTION/FUNARG appearing in user programs.

use sens::{eval_program, Session};

fn eval(expr: &str) -> String {
    let mut session = Session::default();
    eval_program(expr, &mut session)
        .unwrap_or_else(|e| panic!("closure witness failed: {e}\nexpr: {expr}"))
        .value
        .to_string()
}

#[test]
fn d4_lambda_already_supplies_funarg_observable_behavior() {
    let cases = [
        (
            "(((lambda (x) (lambda (y) (cons x y))) (quote A)) (quote B))",
            "(A . B)",
        ),
        (
            "((lambda (f) (f (quote B))) (lambda (y) (cons (quote A) y)))",
            "(A . B)",
        ),
        (
            "(((lambda (x) (lambda (ignored) x)) (quote CAPTURED)) (quote CALLER))",
            "CAPTURED",
        ),
        (
            "((lambda (maker) ((maker (quote A)) (quote B))) (lambda (x) (lambda (y) (cons x y))))",
            "(A . B)",
        ),
    ];

    for (expr, expected) in cases {
        let upper = expr.to_ascii_uppercase();
        assert!(!upper.contains("FUNCTION"));
        assert!(!upper.contains("FUNARG"));
        assert_eq!(eval(expr), expected, "expr: {expr}");
    }
}

#[test]
fn core1_contract_classifies_function_funarg_as_bootstrap_closure_mechanism() {
    let contract = include_str!("../../../contracts/core1-bootstrap-contract.lisp");

    assert!(contract.contains("(first-class-closures . explicit-function-funarg)"));
    assert!(contract.contains("(closure-gap-policy . consume-lisp15-function-funarg)"));
    assert!(contract.contains("(closure-representation . (FUNARG fn captured-environment))"));
    assert!(contract.contains("function-funarg-adapter"));
}
