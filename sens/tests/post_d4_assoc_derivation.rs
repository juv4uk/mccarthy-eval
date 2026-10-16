//! #2285 — historical ASSOC as ordinary structural recursion.
//!
//! Research-only. This test does not allocate a post-D4 identity.
//! It independently reconstructs deep equality from the existing structural
//! foundation, then reconstructs ASSOC on top of that recursion. The result is
//! compared against canonical ASSOC (00101101).
//!
//! The point is not that the repository already contains an `assoc`
//! definition; the point is that the observable behavior can be reproduced
//! without a new primitive and without D4 LOOKUP.

use sens::{eval_program, load_core_library, Session};

const DERIVED_ASSOC: &str = r#"
(00001001 рівні-структурно
  (00001000 (a b)
    (00000111
      ((00000010 a) ()
       (00000111
         ((00000010 b) () (00000001 (1)))
         ((00000010 b) (1) (00000001 (0)))
         ((00000010 b) (0) (00000001 (0)))))
      ((00000010 a) (1)
       (00000111
         ((00000010 b) () (00000001 (0)))
         ((00000010 b) (1)
          (00000111
            ((00000011 a b) (1) (00000001 (1)))
            ((00000011 a b) (0) (00000001 (0)))))
         ((00000010 b) (0) (00000001 (0)))))
      ((00000010 a) (0)
       (00000111
         ((00000010 b) () (00000001 (0)))
         ((00000010 b) (1) (00000001 (0)))
         ((00000010 b) (0)
          (00000111
            ((рівні-структурно (00000101 a) (00000101 b)) (1)
             (рівні-структурно (00000110 a) (00000110 b)))
            ((рівні-структурно (00000101 a) (00000101 b)) (0)
             (00000001 (0))))))))))

(00001001 знайти-пару
  (00001000 (key alist)
    (00000111
      ((00000010 alist) () (00000001 ()))
      ((00000010 alist) (0)
       (00000111
         ((рівні-структурно key (00000101 (00000101 alist))) (1)
          (00000101 alist))
         ((рівні-структурно key (00000101 (00000101 alist))) (0)
          (знайти-пару key (00000110 alist))))))))
"#;

fn session_with_derived_assoc() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    eval_program(DERIVED_ASSOC, &mut session).expect("derived structural ASSOC should load");
    session
}

fn eval_in(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

fn assert_matches_canonical(session: &mut Session, key: &str, alist: &str, expected: &str) {
    let derived = eval_in(
        session,
        &format!("(знайти-пару (00000001 {key}) (00000001 {alist}))"),
    );
    let canonical = eval_in(
        session,
        &format!("(00101101 (00000001 {key}) (00000001 {alist}))"),
    );

    assert_eq!(derived, canonical, "derived and canonical ASSOC diverged");
    assert_eq!(derived, expected);
}

#[test]
fn assoc_is_reproducible_by_structure_without_lookup() {
    let mut session = session_with_derived_assoc();

    assert_matches_canonical(
        &mut session,
        "x",
        "((x . first) (y . second))",
        "(x . first)",
    );

    assert_matches_canonical(
        &mut session,
        "z",
        "((x . first) (y . second))",
        "()",
    );

    // Association-list lookup is left-biased: the first matching pair wins.
    assert_matches_canonical(
        &mut session,
        "x",
        "((x . first) (x . second))",
        "(x . first)",
    );
}

#[test]
fn generic_assoc_needs_no_environment_specific_lookup_semantics() {
    let mut session = session_with_derived_assoc();

    // The generalized repository ASSOC accepts structural keys, not only the
    // symbol keys used by environment lookup. Deep equality itself is rebuilt
    // above from ATOM/EQ/CAR/CDR/COND.
    assert_matches_canonical(
        &mut session,
        "(a b)",
        "(((a b) . first) ((a c) . second))",
        "((a b) . first)",
    );

    // Values are opaque payload: ASSOC returns the original pair without
    // interpreting the value as an environment binding or evaluating it.
    assert_matches_canonical(
        &mut session,
        "k",
        "((k (deep (payload))) (other . value))",
        "(k (deep (payload)))",
    );
}
