//! #2347 — historical APPEND as ordinary structural recursion.
//!
//! Research-only. No post-D4 identity is allocated here.
//!
//! The derived implementation uses only the existing structural/evaluator
//! foundation: DEFINE/LAMBDA for recursion plus COND/ATOM/CAR/CDR/CONS.
//! Canonical `append` is used only as an oracle after the independent
//! implementation has been loaded.

use sens::{eval_program, load_core_library, Session};

const DERIVED_APPEND: &str = r#"
(00001001 приєднати-структурно
  (00001000 (left right)
    (00000111
      ((00000010 left) () right)
      ((00000010 left) (0)
       (00000100
         (00000101 left)
         (приєднати-структурно (00000110 left) right))))))
"#;

fn session_with_derived_append() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");

    let lowered = DERIVED_APPEND.to_ascii_lowercase();
    for forbidden in ["append", "reverse", "map", "filter", "reduce"] {
        assert!(
            !lowered.contains(forbidden),
            "derived APPEND witness must not depend on helper {forbidden}"
        );
    }

    eval_program(DERIVED_APPEND, &mut session)
        .expect("independent structural APPEND should load");
    session
}

fn eval_in(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

fn assert_matches_canonical(
    session: &mut Session,
    left: &str,
    right: &str,
    expected: &str,
) {
    let derived = eval_in(
        session,
        &format!(
            "(приєднати-структурно (00000001 {left}) (00000001 {right}))"
        ),
    );
    let canonical = eval_in(
        session,
        &format!("(append (00000001 {left}) (00000001 {right}))"),
    );

    assert_eq!(derived, canonical, "derived and canonical APPEND diverged");
    assert_eq!(derived, expected);
}

#[test]
fn append_is_reproducible_by_structural_recursion() {
    let mut session = session_with_derived_append();

    assert_matches_canonical(&mut session, "()", "(a b)", "(a b)");
    assert_matches_canonical(&mut session, "(a)", "(b c)", "(a b c)");
    assert_matches_canonical(&mut session, "(a b)", "()", "(a b)");
    assert_matches_canonical(
        &mut session,
        "((a b) c)",
        "(d)",
        "((a b) c d)",
    );

    // APPEND must not inspect or normalize the right payload. Rebuilding only
    // the left spine naturally preserves an improper/dotted right tail.
    assert_matches_canonical(
        &mut session,
        "(a b)",
        "(c . d)",
        "(a b c . d)",
    );
}

#[test]
fn append_needs_no_cons_family_generator_semantics() {
    let mut session = session_with_derived_append();

    // The first list contributes only its CAR payloads and CDR traversal.
    // The second argument is returned unchanged at the recursion base.
    assert_eq!(
        eval_in(
            &mut session,
            "(приєднати-структурно (00000001 ()) (00000001 (payload . tail)))"
        ),
        "(payload . tail)"
    );

    // A dotted/improper left spine is not silently normalized into a list.
    // The research witness requires both the independent structural definition
    // and the repository's canonical APPEND to reject that boundary.
    let derived = eval_program(
        "(приєднати-структурно (00000001 (a . b)) (00000001 (c)))",
        &mut session,
    );
    let canonical = eval_program(
        "(append (00000001 (a . b)) (00000001 (c)))",
        &mut session,
    );

    assert!(derived.is_err(), "derived APPEND accepted improper left spine");
    assert!(
        canonical.is_err(),
        "canonical APPEND accepted improper left spine unexpectedly"
    );
}
