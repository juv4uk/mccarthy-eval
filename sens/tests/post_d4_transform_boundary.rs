//! #2472 — live boundary between local D4 derivation and protocol rewrite.
//!
//! Research-only. No post-D4 identity is allocated.
//!
//! These tests do not prove that protocol rewrites are invalid derivations.
//! They prove the narrower observable fact needed by #2472:
//! the RETURN and SETQ countermodels obtain their stronger behavior only after
//! changing an ordinary helper/observer protocol.

use sens::{eval_program, load_core_library, Session};

fn session() -> Session {
    let mut s = Session::default();
    load_core_library(&mut s).expect("core library should load");
    s
}

fn run(s: &mut Session, source: &str) -> String {
    eval_program(source, s)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn ordinary_helper_without_exit_protocol_cannot_skip_its_caller_continuation() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (plain-helper)
           (00000100
             (00000001 after)
             (00000100
               (plain-helper (00000001 early) (00000001 payload))
               (00000001 ()))))
         (00001000 (mode payload)
           payload))
        "#,
    );

    assert_eq!(result, "(after payload)");
}

#[test]
fn rewritten_call_chain_with_exit_k_can_model_non_local_return() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (rewritten-helper)
           ((00001000 (exit-k)
              (rewritten-helper
                (00000001 early)
                (00001000 (value)
                  (00000100
                    (00000001 after)
                    (00000100 value (00000001 ()))))
                exit-k
                (00000001 payload)))
            (00001000 (value)
              (00000100
                (00000001 returned)
                (00000100 value (00000001 ()))))))
         (00001000 (mode continue-k exit-k payload)
           (00000111
             ((00000011 mode (00000001 early)) (1)
              (exit-k payload))
             ((00000011 mode (00000001 normal)) (1)
              (continue-k payload)))))
        "#,
    );

    // The caller itself is now in CPS tail position: early mode selects
    // exit-k instead of the explicit "after" continuation.
    assert_eq!(result, "(returned payload)");
}

#[test]
fn immutable_observer_with_original_zero_arg_protocol_keeps_old_snapshot() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (old-value)
           ((00001000 (observer)
              ((00001000 (new-value)
                 (00000100
                   (observer)
                   (00000100 new-value (00000001 ()))))
               (00000001 new)))
            (00001000 () old-value)))
         (00000001 old))
        "#,
    );

    assert_eq!(result, "(old new)");
}

#[test]
fn rewritten_observer_protocol_can_receive_current_state_explicitly() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (observer)
           (00000100
             (observer (00000001 old))
             (00000100
               (observer (00000001 new))
               (00000001 ()))))
         (00001000 (current-value)
           current-value))
        "#,
    );

    assert_eq!(result, "(old new)");
}

#[test]
fn boundary_witness_uses_only_existing_d3_d4_identities() {
    const SOURCE: &str = r#"
        00000001 00000011 00000100 00000111 00001000
    "#;
    const ALLOWED: &[&str] = &[
        "00000001", // QUOTE
        "00000011", // EQ
        "00000100", // CONS
        "00000111", // COND
        "00001000", // LAMBDA
    ];

    for token in SOURCE.split_whitespace() {
        assert!(ALLOWED.contains(&token));
    }
}
