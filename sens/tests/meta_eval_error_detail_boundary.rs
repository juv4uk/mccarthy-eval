use sens::{eval_program, ErrorKind, Session};

fn escaped(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

fn meta_eval(source: &str) -> String {
    let mut session = Session::default();
    sens::load_core_library(&mut session).expect("core bootstrap");
    sens::load_meta_evaluator_library(&mut session).expect("meta-eval bootstrap");
    eval_program(
        &format!(r#"(my-eval (read "{}") (quote ()))"#, escaped(source)),
        &mut session,
    )
    .expect("host must successfully run the meta-evaluator")
    .value
    .to_string()
}

// s2_explicitly_contracts_category_not_error_wording was a pure
// doc/source-text check, relocated to `cargo xtask verify` per
// TEST-ARCHITECTURE-1 step 4 — see crates/xtask/src/checks.rs.

#[test]
fn equivalent_surface_failures_need_not_have_identical_source_spans() {
    let mut en_session = Session::default();
    let en = eval_program("(define car 42)", &mut en_session).expect_err("Canon rebinding must fail");

    let mut uk_session = Session::default();
    let uk = eval_program("(визначити car 42)", &mut uk_session)
        .expect_err("the Ukrainian surface must reject the same Canon rebinding");

    assert_eq!(en.kind, ErrorKind::InvalidForm);
    assert_eq!(uk.kind, ErrorKind::InvalidForm);
    assert_ne!(
        en.span, uk.span,
        "source-location metadata is allowed to follow the concrete surface spelling"
    );
}

#[test]
fn meta_structured_diagnostics_remain_available_without_becoming_rust_prose() {
    assert_eq!(
        meta_eval("((lambda (x y) x) 1)"),
        "(error arity (expected (exact 2) received 1))"
    );
    assert_eq!(
        meta_eval("((lambda (x y . rest) x) 1)"),
        "(error arity (expected (at-least 2) received 1))"
    );
    assert_eq!(
        meta_eval("(lambda (x x) x)"),
        "(error invalid-form (lambda-parameters duplicate-parameter x))"
    );

    // The reference side intentionally proves only the contractual kind here.
    // Its human message/span are not parsed into the Lisp diagnostic shape.
    let mut session = Session::default();
    let native = eval_program("((lambda (x y) x) 1)", &mut session)
        .expect_err("reference arity witness must fail");
    assert_eq!(native.kind, ErrorKind::Arity);
}
