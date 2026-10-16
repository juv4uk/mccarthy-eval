//! #2315 / #2407 / #2408 — PROG/GO/RETURN lower-bound research.
//!
//! Research-only. No post-D4 identity is allocated.
//!
//! GO is modeled as explicit quoted state + tail recursion.
//! RETURN is modeled as an explicit first-class exit continuation.
//! No host jump/exception/continuation object is used.

use sens::{eval_program, load_core_library, Session};

const DERIVED_CONTROL: &str = r#"
(00001001 post-d4-go-driver
  (00001000 (state remaining acc)
    (00000111
      ((00000011 state (00000001 start)) (1)
       (post-d4-go-driver
         (00000001 loop)
         remaining
         acc))
      ((00000011 state (00000001 loop)) (1)
       (00000111
         ((00000010 remaining) ()
          (post-d4-go-driver
            (00000001 done)
            remaining
            acc))
         ((00000010 remaining) (0)
          (post-d4-go-driver
            (00000001 loop)
            (00000110 remaining)
            (00000100 (00000101 remaining) acc)))))
      ((00000011 state (00000001 done)) (1)
       acc)
      ((00000011 state state) (1) (00000001 (control-error missing-state))))))

(00001001 post-d4-return-helper2
  (00001000 (mode continue-k exit-k payload)
    (00000111
      ((00000011 mode (00000001 early)) (1)
       (exit-k payload))
      ((00000011 mode (00000001 normal)) (1)
       (continue-k payload))
      (T (00000001 (control-error bad-mode))))))

(00001001 post-d4-return-helper1
  (00001000 (mode continue-k exit-k payload)
    (post-d4-return-helper2 mode continue-k exit-k payload)))

(00001001 post-d4-return-program
  (00001000 (mode marker)
    ((00001000 (exit-k)
       (post-d4-return-helper1
         mode
         (00001000 (value)
           (00000100 (00000001 after)
             (00000100 value (00000001 ()))))
         exit-k
         marker))
     (00001000 (value)
       (00000100 (00000001 returned)
         (00000100 value (00000001 ())))))))

(00001001 post-d4-nested-scope
  (00001000 (inner-mode marker)
    ((00001000 (outer-exit)
       ((00001000 (inner-exit)
          (post-d4-return-helper2
            inner-mode
            (00001000 (value)
              (00000100 (00000001 inner-after)
                (00000100 value (00000001 ()))))
            inner-exit
            marker))
        (00001000 (value)
          (00000100 (00000001 inner-returned)
            (00000100 value (00000001 ()))))))
     (00001000 (value)
       (00000100 (00000001 outer-returned)
         (00000100 value (00000001 ())))))))
"#;

fn session() -> Session {
    let mut s = Session::default();
    load_core_library(&mut s).expect("core library should load");
    eval_program(DERIVED_CONTROL, &mut s).expect("derived control model should load");
    s
}

fn run(s: &mut Session, source: &str) -> String {
    eval_program(source, s)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn go_is_explicit_state_plus_tail_recursion() {
    let mut s = session();

    // Forward transition START -> LOOP skips any distinct intermediary form.
    // LOOP then performs backward self-transition until REMAINING is empty.
    assert_eq!(
        run(
            &mut s,
            "(post-d4-go-driver
               (quote start)
               (quote (a b c))
               (quote ()))"
        ),
        "(c b a)"
    );

    assert_eq!(
        run(
            &mut s,
            "(post-d4-go-driver
               (quote done)
               (quote ignored)
               (quote (kept)))"
        ),
        "(kept)"
    );

    assert_eq!(
        run(
            &mut s,
            "(post-d4-go-driver
               (quote missing)
               (quote ())
               (quote ()))"
        ),
        "(control-error missing-state)"
    );
}

#[test]
fn return_is_explicit_exit_continuation_across_nested_calls() {
    let mut s = session();

    // Early path chooses exit-k and therefore never reaches the explicit
    // continue-k ("after" branch).
    assert_eq!(
        run(
            &mut s,
            "(post-d4-return-program (quote early) (quote payload))"
        ),
        "(returned payload)"
    );

    // Normal path chooses continue-k.
    assert_eq!(
        run(
            &mut s,
            "(post-d4-return-program (quote normal) (quote payload))"
        ),
        "(after payload)"
    );
}

#[test]
fn exit_continuation_is_an_ordinary_closure_with_lexical_capture() {
    let mut s = session();

    let result = run(
        &mut s,
        "((lambda (captured)
            ((lambda (exit-k)
               (post-d4-return-helper1
                 (quote early)
                 (lambda (value) (cons (quote after) (cons value (quote ()))))
                 exit-k
                 (quote ignored)))
             (lambda (value)
               (cons captured (cons value (quote ()))))))
          (quote lexical))",
    );

    assert_eq!(result, "(lexical ignored)");
}

#[test]
fn nested_scopes_use_distinct_explicit_exit_values() {
    let mut s = session();

    assert_eq!(
        run(
            &mut s,
            "(post-d4-nested-scope (quote early) (quote x))"
        ),
        "(inner-returned x)"
    );

    assert_eq!(
        run(
            &mut s,
            "(post-d4-nested-scope (quote normal) (quote x))"
        ),
        "(inner-after x)"
    );
}

#[test]
fn derivation_uses_only_existing_d3_d4_control_and_closure_primitives() {
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

    for token in DERIVED_CONTROL.split(|c: char| c != '0' && c != '1') {
        if token.len() == 8 {
            assert!(
                ALLOWED.contains(&token),
                "unexpected non-D3/D4 identity in control derivation: {token}"
            );
        }
    }

    for forbidden in ["PROG", " GO ", "RETURN", "FEXPR", "FSUBR"] {
        assert!(
            !DERIVED_CONTROL.contains(forbidden),
            "derived source must not import historical control/staging surface {forbidden}"
        );
    }
}
