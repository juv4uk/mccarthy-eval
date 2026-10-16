//! Research witness for #3933.
//!
//! Proves the minimal current ARRAY/VECTOR semantic basis.
//! It does not ratify D8.

use sens::{eval_program, Session};

fn eval(source: &str) -> String {
    eval_program(source, &mut Session::default())
        .expect("vector recovery witness should evaluate")
        .value
        .to_string()
}

#[test]
fn vector_construct_and_index_observe_distinct_cells() {
    assert_eq!(eval("(vector-ref (vector 10 20 30) 0)"), "10");
    assert_eq!(eval("(vector-ref (vector 10 20 30) 2)"), "30");
}

#[test]
fn vector_update_is_visible_through_an_alias() {
    assert_eq!(
        eval("(define v (vector 10 20)) (define w v) (vector-set! v 0 99) (vector-ref w 0)"),
        "99"
    );
}

#[test]
fn vector_update_preserves_unmodified_cells() {
    assert_eq!(
        eval("(define v (vector 10 20)) (vector-set! v 0 99) (vector-ref v 1)"),
        "20"
    );
}
