use sens::{eval_program, Session};

#[test]
fn same_macro_value_under_two_heads_sees_same_operand_payload() {
    let mut session = Session::default();

    eval_program(
        r#"
        (define shared-macro (make-macro (lambda (x) x)))
        (define macro-a shared-macro)
        (define macro-b shared-macro)
        "#,
        &mut session,
    )
    .expect("shared macro aliases should be definable");

    let a = eval_program("(macro-a (quote payload))", &mut session)
        .expect("macro-a should expand");
    let b = eval_program("(macro-b (quote payload))", &mut session)
        .expect("macro-b should expand");

    assert_eq!(a.value.to_string(), "payload");
    assert_eq!(b.value.to_string(), "payload");
}

#[test]
fn evaluator_dispatches_operand_tail_to_apply_macro_without_call_head() {
    let evaluator = include_str!("../src/eval/mod.rs");
    let closures = include_str!("../src/eval/closures.rs");

    assert!(
        evaluator.contains("&items[1..],"),
        "list dispatch must separate the call head from the operand tail"
    );

    let call = "closures::apply_macro(closure.clone(), arguments, environment, span)";
    assert!(
        evaluator.matches(call).count() >= 2,
        "both macro dispatch paths should forward only the operand slice"
    );

    let start = closures
        .find("pub(super) fn apply_macro(")
        .expect("apply_macro must exist");
    let end = closures[start..]
        .find(") -> Result<EvalStep")
        .map(|offset| start + offset)
        .expect("apply_macro signature should terminate before Result<EvalStep");
    let signature = &closures[start..end];

    assert!(signature.contains("closure: Rc<Closure>"));
    assert!(signature.contains("arguments: &[Expr]"));
    assert!(signature.contains("calling_environment: &Environment"));
    assert!(!signature.contains("head_name"));
    assert!(!signature.contains("head_expr"));
    assert!(!signature.contains("head_sid"));
}
