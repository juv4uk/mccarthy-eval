use sens::{eval_program, Session};

fn escaped(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

fn meta_eval(source: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).expect("core bootstrap");
    sens::load_meta_evaluator_library(&mut session)
        .expect("meta-eval bootstrap");
    eval_program(
        &format!(r#"(my-eval (read "{}") (quote ()))"#, escaped(source)),
        &mut session,
    )
    .unwrap_or_else(|error| panic!("host failure while meta-evaluating {source}: {error}"))
    .value
    .to_string()
}

fn native_value(source: &str) -> String {
    let mut session = Session::default();
    eval_program(source, &mut session)
        .unwrap_or_else(|error| panic!("reference evaluator failed for {source}: {error}"))
        .value
        .to_string()
}

#[test]
fn error_shaped_user_data_keeps_value_provenance() {
    // native and meta are each checked against the same authored expected
    // value independently, never against each other.
    let source = "((lambda (x) (car x)) (quote (error ordinary data)))";
    let expected = "error";
    assert_eq!(native_value(source), expected);
    assert_eq!(meta_eval(source), expected);
}

#[test]
fn fail_shaped_user_data_is_not_internal_failure() {
    let source = "((lambda (x) (car x)) (quote (fail ordinary data)))";
    let expected = "fail";
    assert_eq!(native_value(source), expected);
    assert_eq!(meta_eval(source), expected);
}
