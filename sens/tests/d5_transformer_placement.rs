use sens::{eval_program, ErrorKind, Session, Value};
use std::rc::Rc;

#[test]
fn d5_transformer_placement_preserves_lambda_closure_payload_identity() {
    let mut session = Session::default();

    let closure = eval_program(
        "(define candidate-base (lambda (a b) a)) candidate-base",
        &mut session,
    )
    .expect("ordinary LAMBDA closure should be constructible")
    .value;

    let transformer = eval_program("(make-macro candidate-base)", &mut session)
        .expect("surrogate transformer materializer should accept a closure")
        .value;

    match (&closure, &transformer) {
        (Value::Closure(base), Value::Macro(staged)) => assert!(
            Rc::ptr_eq(base, staged),
            "transformer must preserve the exact LAMBDA closure payload"
        ),
        other => panic!("expected Closure -> Macro over one payload, got {other:?}"),
    }
}

#[test]
fn d5_transformer_placement_materializer_rejects_non_closure_input() {
    let mut session = Session::default();
    let error = eval_program("(make-macro 42)", &mut session)
        .expect_err("surrogate transformer materializer must require a closure");

    assert_eq!(error.kind, ErrorKind::Type);
    assert!(error.message.contains("closure"));
}

#[test]
fn d5_transformer_placement_changes_call_mode_not_body_payload() {
    let mut session = Session::default();

    eval_program(
        r#"
        (define same-body (lambda (a b) a))
        (define staged-same-body (make-macro same-body))
        "#,
        &mut session,
    )
    .expect("base and staged views should share one closure body");

    let ordinary = eval_program(
        "(same-body (quote ok) never-defined)",
        &mut session,
    )
    .expect_err("ordinary closure must eagerly evaluate never-defined");
    assert_eq!(ordinary.kind, ErrorKind::UnknownSymbol);

    let staged = eval_program(
        "(staged-same-body (quote ok) never-defined)",
        &mut session,
    )
    .expect("staged view must preserve unused raw operand")
    .value
    .to_string();

    assert_eq!(staged, "ok");
}

#[test]
fn d5_transformer_placement_candidate_word_is_free_of_selector_subtree() {
    const CANDIDATE: &str = "00101";
    const FIXED_SELECTORS: [&str; 8] = [
        "10100", "10101", "10110", "10111",
        "11000", "11001", "11010", "11011",
    ];

    assert!(!FIXED_SELECTORS.contains(&CANDIDATE));
    assert_eq!(&CANDIDATE[..4], "0010", "candidate must refine D4 LAMBDA");
    assert_eq!(&CANDIDATE[4..], "1", "candidate tests the staged/context suffix");
}

#[test]
fn d5_transformer_placement_zero_child_stays_unallocated_in_research_model() {
    const ORDINARY_PARENT: &str = "0010";
    const ZERO_CHILD: &str = "00100";
    const STAGED_CHILD: &str = "00101";

    assert_eq!(&ZERO_CHILD[..4], ORDINARY_PARENT);
    assert_eq!(&STAGED_CHILD[..4], ORDINARY_PARENT);

    // Research law under test: ordinary closure semantics already live at the
    // exact-width D4 parent. A zero-child would therefore duplicate the parent
    // unless independent evidence later gives it a distinct observable role.
    assert_ne!(ZERO_CHILD, STAGED_CHILD);
}
