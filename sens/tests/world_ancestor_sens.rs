//! Native SENS executes its own self-verdicts; Rust only loads source and observes D1/error receipts.

use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source,
    eval_pinned_reasoning_source, parse_mixed_exact_domain, Session,
};

const WORLD_WITNESSES: &str =
    include_str!("../../../tests/semantic/world-exact-arity-witness.lisp");
const ANCESTOR_LAW: &str =
    include_str!("../../../tests/semantic/world-ancestor-d1-law.lisp");
const ANCESTOR_MUTANT: &str =
    include_str!("../../../tests/semantic/world-ancestor-d1-mutant.lisp");

fn world_session() -> Session {
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("pinned Core must load before the actual SENS World");
    for (path, source) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp")),
        ("lib/world.lisp", include_str!("../../../lib/world.lisp")),
    ] {
        eval_pinned_reasoning_source(path, source, &mut session)
            .unwrap_or_else(|error| panic!("SENS library {path}: {error}"));
    }
    session
}

/// The fixture contains executable SENS self-verdicts. The host observes only
/// their exact D1:YES completion; it does not calculate the ancestor or encode
/// a second expected-result table.
fn native_sens_self_verdicts(source: &str, fixture: &str) -> usize {
    let parsed = parse_mixed_exact_domain(source)
        .unwrap_or_else(|error| panic!("test_id=world-ancestor layer=parse fixture={fixture} error={error}"));
    assert!(!parsed.is_empty(), "test_id=world-ancestor fixture={fixture} is empty");
    let mut session = world_session();
    for (i, expression) in parsed.iter().enumerate() {
        let result = eval_parsed_expressions(std::slice::from_ref(expression), &mut session)
            .unwrap_or_else(|error| {
                panic!("test_id=world-ancestor layer=eval fixture={fixture} form={i} error={error}")
            });
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(true),
            "SENS self-verdict must complete with exact D1:YES; fixture={fixture} form={i} actual={:?}",
            result.value,
        );
    }
    parsed.len()
}

#[test]
fn world_ancestor_is_proved_by_native_sens() {
    let witnessed = native_sens_self_verdicts(
        WORLD_WITNESSES,
        "tests/semantic/world-exact-arity-witness.lisp",
    );
    assert_eq!(witnessed, 24, "world self-verdict fixture must retain exactly 24 executable SENS assertions");
}

#[test]
fn world_ancestor_one_bit_mutant_is_rejected_by_sens() {
    assert_eq!(ANCESTOR_LAW.len(), ANCESTOR_MUTANT.len(), "only the D1 word may change");
    let changes = ANCESTOR_LAW.bytes().zip(ANCESTOR_MUTANT.bytes())
        .filter(|(a, b)| a != b)
        .collect::<Vec<_>>();
    assert_eq!(changes.as_slice(), &[(b'1', b'0')], "one-bit D1 mutation only");

    assert_eq!(
        native_sens_self_verdicts(ANCESTOR_LAW, "tests/semantic/world-ancestor-d1-law.lisp"),
        1
    );

    let parsed = parse_mixed_exact_domain(ANCESTOR_MUTANT)
        .expect("one-bit mutant remains syntactically valid exact-domain SENS");
    assert_eq!(parsed.len(), 1, "mutant must remain one executable SENS self-verdict");
    let mut session = world_session();
    let result = eval_parsed_expressions(&parsed, &mut session);
    let error = match result {
        Ok(value) => panic!("SENS admitted its one-bit falsifier; unexpected value {:?}", value.value),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("not callable"),
        "the mutated D1:NO branch must fail inside SENS as non-callable, got: {error}"
    );
}
