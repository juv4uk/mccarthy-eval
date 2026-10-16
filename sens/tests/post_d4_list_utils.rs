//! #2287 — post-D4 historical list-utility derivation witness.
//!
//! Independently reconstructs APPEND, SUBST, SUBLIS and MAPLIST from the
//! already-ratified D3 structural primitives plus D4 DEFINE/LAMBDA.
//! The historical Function8 identities for SUBST/SUBLIS/MAPLIST are forbidden
//! from the derivation source; current library functions are used only as the
//! differential oracle.

use sens::{eval_program, load_core_library, Session};

const DERIVED: &str = r#"
(00001001 post-d4-append
  (00001000 (left right)
    (00000111
      ((00000010 left) () right)
      ((00000010 left) (0)
       (00000100
         (00000101 left)
         (post-d4-append (00000110 left) right))))))

(00001001 post-d4-subst
  (00001000 (x y z)
    (00000111
      ((00000010 z) (0)
       (00000100
         (post-d4-subst x y (00000101 z))
         (post-d4-subst x y (00000110 z))))
      ((00000010 z) ()
       (00000111
         ((00000011 z y) (1) x)
         ((00000011 z y) (0) z)))
      ((00000010 z) (1)
       (00000111
         ((00000011 z y) (1) x)
         ((00000011 z y) (0) z))))))

(00001001 post-d4-sublis-pair
  (00001000 (pairs atom)
    (00000111
      ((00000010 pairs) () atom)
      ((00000010 pairs) (0)
       (00000111
         ((00000011 (00000101 (00000101 pairs)) atom) (1)
          (00000101 (00000110 (00000101 pairs))))
         ((00000011 (00000101 (00000101 pairs)) atom) (0)
          (post-d4-sublis-pair (00000110 pairs) atom)))))))

(00001001 post-d4-sublis
  (00001000 (pairs tree)
    (00000111
      ((00000010 tree) (0)
       (00000100
         (post-d4-sublis pairs (00000101 tree))
         (post-d4-sublis pairs (00000110 tree))))
      ((00000010 tree) ()
       (post-d4-sublis-pair pairs tree))
      ((00000010 tree) (1)
       (post-d4-sublis-pair pairs tree)))))

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
    eval_program(DERIVED, &mut session).expect("D3+D4 derivations should load");
    session
}

fn run(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn derivation_source_uses_only_d3_plus_d4_define_lambda_identities() {
    const ALLOWED: &[&str] = &[
        "00000001", // QUOTE
        "00000010", // ATOM
        "00000011", // EQ
        "00000100", // CONS
        "00000101", // CAR
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

    for retired_historical_identity in ["10101100", "10101101", "10101110"] {
        assert!(
            !DERIVED.contains(retired_historical_identity),
            "derivation must not reuse historical Function8 identity {retired_historical_identity}"
        );
    }
}

#[test]
fn append_is_reconstructible_from_structure_and_recursion() {
    let mut s = session();
    let cases = [
        ("(quote ())", "(quote (c d))"),
        ("(quote (a b))", "(quote (c d))"),
        ("(quote (a (b c)))", "(quote (d))"),
    ];

    for (left, right) in cases {
        let derived = run(&mut s, &format!("(post-d4-append {left} {right})"));
        let canonical = run(&mut s, &format!("(append {left} {right})"));
        assert_eq!(derived, canonical, "left={left} right={right}");
    }

    let derived_error = eval_program(
        "(post-d4-append (quote atom) (quote (tail)))",
        &mut s,
    )
    .unwrap_err();
    let canonical_error =
        eval_program("(append (quote atom) (quote (tail)))", &mut s).unwrap_err();
    assert_eq!(derived_error.kind, canonical_error.kind);
}

#[test]
fn subst_is_reconstructible_from_d3_structure() {
    let mut s = session();
    let cases = [
        (
            "(quote (x . a))",
            "(quote b)",
            "(quote ((a . b) . c))",
        ),
        ("(quote replacement)", "(quote a)", "(quote (a (b a)))"),
        ("(quote x)", "(quote absent)", "(quote (a b c))"),
    ];

    for (x, y, z) in cases {
        let derived = run(&mut s, &format!("(post-d4-subst {x} {y} {z})"));
        let canonical = run(&mut s, &format!("(subst {x} {y} {z})"));
        assert_eq!(derived, canonical, "x={x} y={y} z={z}");
    }
}

#[test]
fn sublis_is_reconstructible_from_pair_recursion() {
    let mut s = session();
    let cases = [
        (
            "(quote ((x (a b)) (y (b c))))",
            "(quote (x y z))",
        ),
        ("(quote ())", "(quote (x y))"),
        (
            "(quote ((x alpha) (y beta)))",
            "(quote ((x y) (z x)))",
        ),
    ];

    for (pairs, tree) in cases {
        let derived = run(&mut s, &format!("(post-d4-sublis {pairs} {tree})"));
        let canonical = run(&mut s, &format!("(sublis {pairs} {tree})"));
        assert_eq!(derived, canonical, "pairs={pairs} tree={tree}");
    }
}

#[test]
fn maplist_is_reconstructible_from_lambda_application_and_list_recursion() {
    let mut s = session();
    let cases = [
        (
            "(quote (a b c))",
            "(lambda (tail) tail)",
        ),
        (
            "(quote (a b c))",
            "(lambda (tail) (car tail))",
        ),
        (
            "(quote ())",
            "(lambda (tail) tail)",
        ),
    ];

    for (values, function) in cases {
        let derived = run(
            &mut s,
            &format!("(post-d4-maplist {values} {function})"),
        );
        let canonical = run(&mut s, &format!("(maplist {values} {function})"));
        assert_eq!(derived, canonical, "values={values} function={function}");
    }
}
