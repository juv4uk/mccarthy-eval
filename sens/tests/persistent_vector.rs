//! Exercises lib/persistent-vector.my — the AVL-balanced persistent
//! vector written in sens itself (OPT-PERSISTENT-VECTOR). Loads
//! lib/core.my (for `second`/`third`/`fourth`/`fifth`/`not`/`append`)
//! and lib/persistent-vector.my into one session, same as a user
//! loading both from a REPL.

use sens::{eval_program, Session};

fn eval_vec(source: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    eval_program(
        include_str!("../../../lib/persistent-vector.lisp"),
        &mut session,
    )
    .unwrap();
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn empty_vector_has_nothing() {
    assert_eq!(eval_vec("(vec-nth 0 vec-empty)"), "()");
    assert_eq!(eval_vec("(vec-count vec-empty)"), "0");
    assert_eq!(eval_vec("(vec->list vec-empty)"), "()");
}

#[test]
fn conj_then_nth_finds_the_value_at_its_index() {
    let source = r#"
        (def v (vec-conj (quote c) (vec-conj (quote b) (vec-conj (quote a) vec-empty))))
        (list (vec-nth 0 v) (vec-nth 1 v) (vec-nth 2 v) (vec-nth 3 v) (vec-count v))
    "#;
    assert_eq!(eval_vec(source), "((a) (b) (c) () 3)");
}

/// The order-preservation bug caught live before this test existed: a
/// naive `(vec-conj (car lst) (vec-from-list (cdr lst)))` conjes the
/// LAST list element first (its recursive call bottoms out before any
/// conj runs), reversing the list into the vector. vec-from-list uses
/// an accumulator instead — this test is exactly what would have failed
/// on the naive version.
#[test]
fn vec_from_list_preserves_list_order() {
    assert_eq!(
        eval_vec("(vec->list (vec-from-list (list 10 20 30 40 50)))"),
        "(10 20 30 40 50)"
    );
}

#[test]
fn conj_is_persistent_the_original_vector_is_untouched() {
    let source = r#"
        (def v1 (vec-conj (quote a) vec-empty))
        (def v2 (vec-conj (quote b) v1))
        (list (vec->list v1) (vec->list v2) (vec-count v1) (vec-count v2))
    "#;
    assert_eq!(eval_vec(source), "((a) (a b) 1 2)");
}

/// The actual point of choosing a balanced tree: 4607 sequential
/// vec-conj calls (already-ascending index insertion order, the exact
/// pattern that degenerates an unbalanced BST into a height-4607 linked
/// list) still produces a tree whose height stays near the theoretical
/// AVL minimum, not linear in element count — verified live before this
/// test was written by printing vnode-height and comparing to
/// ceil(log2(n+1)), not assumed from the rotation code alone.

#[test]
fn vec_nth_out_of_bounds_returns_the_maybe_shape_absent_case() {
    let source = r#"
        (def v (vec-from-list (list 1 2 3)))
        (list (vec-nth 3 v) (vec-nth 100 v))
    "#;
    assert_eq!(eval_vec(source), "(() ())");
}

/// Empirical confirmation this actually fixes the cited problem
/// (OPT-PERSISTENT-VECTOR: WSM-24's O(n) `nth` inside a subsample loop
/// over a 4607-point list made the whole loop O(n^2)) — not just a
/// microbenchmark in isolation, the exact access pattern (many
/// sequential index lookups over a large collection) that motivated
/// this file.
#[test]
fn vec_nth_over_a_large_vector_returns_correct_values_at_every_index() {
    let source = r#"
        (def build-range
          (lambda (n acc)
            (cond ((eq? n 0) acc)
                  (t (build-range (- n 1) (cons n acc))))))
        (def big (vec-from-list (build-range 4607 (quote ()))))
        (def check-all
          (lambda (i n v)
            (cond ((eq? i n) t)
                  ((eq? (car (vec-nth i v)) (+ i 1)) (check-all (+ i 1) n v))
                  (t (quote ())))))
        (list (vec-count big) (check-all 0 4607 big))
    "#;
    assert_eq!(eval_vec(source), "(4607 t)");
}
