//! Vector family acceptance — contract 2.1 style, value-level semantics.

use sens::{eval_program, ErrorKind, Session};

fn session_with_core() -> Session {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session)
        .expect("core.my should preload cleanly");
    session
}

fn eval_source(source: &str) -> String {
    let result = eval_program(source, &mut session_with_core()).expect("eval should succeed");
    result.value.to_string()
}

fn eval_err(source: &str) -> ErrorKind {
    let mut session = session_with_core();
    eval_program(source, &mut session)
        .expect_err("should fail named")
        .kind
}

#[test]
fn vector_literal_and_ref() {
    assert_eq!(eval_source("(vector-ref (vector 10 20 30) 0)"), "10");
    assert_eq!(eval_source("(vector-ref (vector 10 20 30) 2)"), "30");
}

#[test]
fn make_vector_fills_with_nil_and_length_reports_size() {
    assert_eq!(eval_source("(vector-length (make-vector 5))"), "5");
    assert_eq!(eval_source("(vector-ref (make-vector 3) 1)"), "()");
}

#[test]
fn vector_set_bang_mutates_through_the_binding() {
    assert_eq!(
        eval_source("(def v (vector 10 20 30))\n(vector-set! v 0 99)\n(vector-ref v 0)"),
        "99"
    );
    // other slots untouched
    assert_eq!(
        eval_source("(def v (vector 10 20 30))\n(vector-set! v 0 99)\n(vector-ref v 1)"),
        "20"
    );
}

#[test]
fn out_of_bounds_fails_named() {
    assert_eq!(
        eval_err("(vector-ref (vector 1) 5)"),
        ErrorKind::InvalidForm
    );
    assert_eq!(
        eval_err("(vector-set! (make-vector 2) 7 99)"),
        ErrorKind::InvalidForm
    );
}

#[test]
fn type_errors_fail_named() {
    assert_eq!(eval_err("(vector-ref (quote (1 2)) 0)"), ErrorKind::Type);
    assert_eq!(eval_err("(vector-length 42)"), ErrorKind::Type);
    assert_eq!(eval_err("(make-vector -1)"), ErrorKind::Type);
    assert_eq!(eval_err("(make-vector 2.5)"), ErrorKind::Type);
    assert_eq!(eval_err("(vector-ref (vector 1) 1.5)"), ErrorKind::Type);
}

#[test]
fn write_to_string_round_trip_uses_hash_paren_syntax() {
    assert_eq!(
        eval_source("(write-to-string (vector 1 (list 2) \"a\"))"),
        "\"#(#q2:1/1 (#q2:10/1) \\\"a\\\")\""
    );
}
