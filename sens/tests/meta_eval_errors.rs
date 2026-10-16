use sens::{eval_program, ErrorKind, Session};

fn escape_lisp_string(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

fn eval_meta(source: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    sens::load_meta_evaluator_library(&mut session).unwrap();

    let wrapper = format!(
        r#"(my-eval (read "{}") (quote ()))"#,
        escape_lisp_string(source),
    );

    eval_program(&wrapper, &mut session)
        .unwrap()
        .value
        .to_string()
}

fn eval_native_error(source: &str, expected_kind: ErrorKind, normalized: &str) -> String {
    let mut session = Session::default();
    let error = eval_program(source, &mut session).unwrap_err();
    assert_eq!(error.kind, expected_kind, "wrong native error kind for {source}");

    // Error wording and source spans are presentation details. S2 contracts the
    // named category; tests normalize only the semantic observation shared with
    // the Lisp meta-evaluator.
    normalized.to_string()
}

#[test]
fn unresolved_callable_has_named_error_parity() {
    for symbol in ["missing", "future-function", "not-defined-yet"] {
        let source = format!("({symbol})");
        let expected = format!("(error unbound-symbol {symbol})");
        let via_meta = eval_meta(&source);
        let via_native = eval_native_error(&source, ErrorKind::UnknownSymbol, &expected);

        assert_eq!(
            via_meta, via_native,
            "unbound-symbol parity failed for {source}"
        );
    }
}

#[test]
fn non_callable_values_are_type_failures_not_unknown_symbols() {
    for (source, expected) in [
        ("(42)", "(error not-callable 42)"),
        ("((quote missing))", "(error not-callable missing)"),
    ] {
        let via_meta = eval_meta(source);
        let via_native = eval_native_error(source, ErrorKind::Type, expected);

        assert_eq!(
            via_meta, via_native,
            "not-callable parity failed for {source}"
        );
    }
}

#[test]
fn fixed_lambda_arity_mismatch_is_named_lisp_data() {
    for (source, expected) in [
        (
            "((lambda (x y) x) 1)",
            "(error arity (expected (exact 2) received 1))",
        ),
        (
            "((lambda (x y) x) 1 2 3)",
            "(error arity (expected (exact 2) received 3))",
        ),
    ] {
        let via_meta = eval_meta(source);
        let via_native = eval_native_error(source, ErrorKind::Arity, expected);

        assert_eq!(via_meta, via_native, "fixed arity parity failed for {source}");
    }
}

#[test]
fn dotted_lambda_reports_minimum_arity_not_exact_arity() {
    let source = "((lambda (x y . rest) x) 1)";
    let expected = "(error arity (expected (at-least 2) received 1))";

    let via_meta = eval_meta(source);
    let via_native = eval_native_error(source, ErrorKind::Arity, expected);

    assert_eq!(via_meta, via_native, "dotted arity parity failed for {source}");
}

#[test]
fn invalid_lambda_list_structure_has_named_error_parity() {
    for (source, expected) in [
        (
            "(lambda (1) 1)",
            "(error invalid-form (lambda-parameters non-symbol-parameter 1))",
        ),
        (
            "(lambda (x x) x)",
            "(error invalid-form (lambda-parameters duplicate-parameter x))",
        ),
        (
            "(lambda (x . 1) x)",
            "(error invalid-form (lambda-parameters invalid-rest 1))",
        ),
    ] {
        let via_meta = eval_meta(source);
        let via_native = eval_native_error(source, ErrorKind::InvalidForm, expected);

        assert_eq!(
            via_meta, via_native,
            "invalid lambda-list parity failed for {source}"
        );
    }
}

#[test]
fn malformed_inline_lambda_does_not_degrade_to_not_callable() {
    let source = "((lambda (x x) x) 1 2)";
    let expected = "(error invalid-form (lambda-parameters duplicate-parameter x))";

    let via_meta = eval_meta(source);
    let via_native = eval_native_error(source, ErrorKind::InvalidForm, expected);

    assert_eq!(
        via_meta, via_native,
        "inline malformed lambda must fail during closure construction"
    );
}
