//! #2349 — MAPLIST from D3 list recursion + D4 first-class LAMBDA.
//!
//! Research-only. No post-D4 identity is allocated.
//! The derived function traverses successive tails recursively and invokes the
//! passed D4 callable as an ordinary first-class value.

use sens::{eval_program, load_core_library, Session};

const DERIVED: &str = r#"
(00001001 post-d4-maplist
  (00001000 (values f)
    (00000111
      ((00000010 values) () (00000001 ()))
      ((00000010 values) (0)
       (00000100
         (f values)
         (post-d4-maplist (00000110 values) f))))))
"#;

fn session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    eval_program(DERIVED, &mut session).expect("derived MAPLIST should load");
    session
}

fn run(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn derivation_uses_only_d3_structure_plus_d4_define_lambda() {
    const ALLOWED: &[&str] = &[
        "00000001", // QUOTE
        "00000010", // ATOM
        "00000100", // CONS
        "00000110", // CDR
        "00000111", // COND
        "00001000", // LAMBDA
        "00001001", // DEFINE
    ];

    for token in DERIVED.split(|c: char| c != '0' && c != '1') {
        if token.len() == 8 {
            assert!(
                ALLOWED.contains(&token),
                "unexpected non-D3/D4 identity in derivation: {token}"
            );
        }
    }

    assert!(
        !DERIVED.contains("10101110"),
        "derivation must not reuse historical MAPLIST Function8 identity"
    );
}

#[test]
fn maplist_is_successive_tail_recursion() {
    let mut s = session();

    for (values, function) in [
        ("(quote ())", "(lambda (tail) tail)"),
        ("(quote (a))", "(lambda (tail) tail)"),
        ("(quote (a b c))", "(lambda (tail) tail)"),
        ("(quote (a b c))", "(lambda (tail) (car tail))"),
    ] {
        let derived = run(
            &mut s,
            &format!("(post-d4-maplist {values} {function})"),
        );
        let canonical = run(&mut s, &format!("(maplist {values} {function})"));
        assert_eq!(derived, canonical, "values={values} function={function}");
    }

    assert_eq!(
        run(
            &mut s,
            "(post-d4-maplist (quote (a b c)) (lambda (tail) tail))"
        ),
        "((a b c) (b c) (c))"
    );
}

#[test]
fn callback_is_a_first_class_closure_with_lexical_capture() {
    let mut s = session();

    let derived = run(
        &mut s,
        "((lambda (marker)
            (post-d4-maplist
              (quote (a b))
              (lambda (tail) (cons marker tail))))
          (quote m))",
    );
    let canonical = run(
        &mut s,
        "((lambda (marker)
            (maplist
              (quote (a b))
              (lambda (tail) (cons marker tail))))
          (quote m))",
    );

    assert_eq!(derived, canonical);
    assert_eq!(derived, "((m a b) (m b))");
}

#[test]
fn callback_may_be_passed_as_a_value() {
    let mut s = session();

    let derived = run(
        &mut s,
        "((lambda (f)
            (post-d4-maplist (quote (a b)) f))
          (lambda (tail) (car tail)))",
    );
    let canonical = run(
        &mut s,
        "((lambda (f)
            (maplist (quote (a b)) f))
          (lambda (tail) (car tail)))",
    );

    assert_eq!(derived, canonical);
    assert_eq!(derived, "(a b)");
}

#[test]
fn callback_errors_propagate_without_new_traversal_semantics() {
    let mut s = session();

    let derived = eval_program(
        "(post-d4-maplist
           (quote (a))
           (lambda (tail) (car (quote ()))))",
        &mut s,
    )
    .unwrap_err();

    let canonical = eval_program(
        "(maplist
           (quote (a))
           (lambda (tail) (car (quote ()))))",
        &mut s,
    )
    .unwrap_err();

    assert_eq!(derived.kind, canonical.kind);
}
