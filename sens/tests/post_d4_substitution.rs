//! #2348 — SUBST/SUBLIS as ordinary structural recursion.
//!
//! Research-only. No post-D4 identity is allocated.
//! The derivation uses only D3 structure/EQ/COND plus D4 DEFINE/LAMBDA.
//! Historical Function8 SUBST/SUBLIS identities and canonical ASSOC are not
//! used by the derived source.

use sens::{eval_program, load_core_library, Session};

const DERIVED: &str = r#"
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
"#;

fn session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    eval_program(DERIVED, &mut session).expect("derived substitution functions should load");
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

    for forbidden in [
        "10101100", // historical SUBST
        "10101101", // historical SUBLIS
        "00101101", // canonical generic ASSOC
    ] {
        assert!(
            !DERIVED.contains(forbidden),
            "derivation must not reuse {forbidden}"
        );
    }
}

#[test]
fn subst_is_ordinary_recursive_tree_transformation() {
    let mut s = session();

    let cases = [
        ("(quote replacement)", "(quote a)", "(quote a)"),
        ("(quote replacement)", "(quote a)", "(quote b)"),
        (
            "(quote (new tree))",
            "(quote a)",
            "(quote (a (b a) . a))",
        ),
        ("(quote x)", "(quote b)", "(quote (a . b))"),
    ];

    for (x, y, z) in cases {
        let derived = run(&mut s, &format!("(post-d4-subst {x} {y} {z})"));
        let canonical = run(&mut s, &format!("(subst {x} {y} {z})"));
        assert_eq!(derived, canonical, "x={x} y={y} z={z}");
    }
}

#[test]
fn sublis_reconstructs_first_match_association_search_and_tree_walk() {
    let mut s = session();

    let cases = [
        (
            "(quote ((x first) (x second)))",
            "(quote x)",
        ),
        (
            "(quote ((x alpha) (y beta)))",
            "(quote ((x y) . x))",
        ),
        (
            "(quote ((x (a b))))",
            "(quote (x z))",
        ),
        (
            "(quote ())",
            "(quote (x y))",
        ),
    ];

    for (pairs, tree) in cases {
        let derived = run(&mut s, &format!("(post-d4-sublis {pairs} {tree})"));
        let canonical = run(&mut s, &format!("(sublis {pairs} {tree})"));
        assert_eq!(derived, canonical, "pairs={pairs} tree={tree}");
    }

    // Duplicate keys observe the same left-biased ordering as derived ASSOC:
    // the first alist row wins.
    assert_eq!(
        run(
            &mut s,
            "(post-d4-sublis (quote ((x first) (x second))) (quote x))"
        ),
        "first"
    );
}
