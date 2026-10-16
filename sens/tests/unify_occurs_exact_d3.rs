//! Живий oracle: реальний Lisp-owned occurs-check, без симуляції helper.
//! D6 LET вилучений, body перенесений до точної D4 LAMBDA.
use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source,
    parse_mixed_exact_domain, parse_mixed_exact_domain_reasoning_source, Session,
};

fn session() -> Session {
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("legacy bootstrap must load by pinned path");
    let expressions = parse_mixed_exact_domain_reasoning_source(
        "lib/unify.lisp",
        include_str!("../../../lib/unify.lisp"),
    )
    .expect("real exact D3/D4 unify source must parse");
    eval_parsed_expressions(&expressions, &mut session)
        .expect("real Lisp unifier must load");
    session
}

fn observed(session: &mut Session, source: &str) -> String {
    let expressions = parse_mixed_exact_domain(source).expect("query exact-domain parse");
    eval_parsed_expressions(&expressions, session)
        .unwrap_or_else(|error| panic!("occurs-check failed for {source}: {error:?}"))
        .value.to_string()
}

#[test]
fn occurs_check_rejects_self_reference_and_accepts_acyclic_term() {
    let mut session = session();
    for (source, expected) in [
        ("(occurs-check? (001 (var x)) (001 (var x)) (001 ()))", "1"),
        ("(occurs-check? (001 (var x)) (001 (f (var x))) (001 ()))", "1"),
        ("(occurs-check? (001 (var x)) (001 (f y)) (001 ()))", "0"),
        ("(occurs-check? (001 (var x)) (001 y) (001 ()))", "0"),
    ] {
        assert_eq!(observed(&mut session, source), expected, "{source}");
    }
}

#[test]
fn one_walk_helper_preserves_frozen_inner_cond_body() {
    let mut session = session();
    for (source, expected) in [
        ("(уніфікація-входження-розв'язано? (001 (var x)) (001 (var x)) (001 ()))", "1"),
        ("(уніфікація-входження-розв'язано? (001 (var x)) (001 y) (001 ()))", "0"),
    ] {
        assert_eq!(observed(&mut session, source), expected, "{source}");
    }
}
