//! Тільки транспорт для двійкових самодоказів SENS.
//! Жодної Rust-таблиці очікуваних значень, семантики лямбди або логічного оракула.
//! Власне твердження й fail-closed-мутант — у tests/semantic/*.bits.

use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source,
    parse_canonical_binary, parse_mixed_exact_domain_reasoning_source, Session,
};

const SOURCE: &str = include_str!("../../../lib/quantity.lisp");
const PROOF: &str = include_str!("../../../tests/semantic/quantity-d4-bind-law.bits");
const MUTANT: &str = include_str!("../../../tests/semantic/quantity-d4-bind-mutant.bits");

#[test]
fn quantity_d4_binary_sens_owns_the_semantic_verdict() {
    // Mechanical integrity of the one-file source migration.
    assert!(SOURCE.contains("((0010 (name)"));
    assert!(!SOURCE.contains("(001000 ((name"));
    for bits in [PROOF, MUTANT] {
        assert!(bits.bytes().all(|b| matches!(b, b'0' | b'1' | b' ' | b'\r' | b'\n' | b'\t')),
            "тільки точні двійкові токени та D2-роздільники");
    }

    // The host does not decide any expected logical result.
    let forms = parse_canonical_binary(PROOF).expect("адмісія канонічної D2-рамки");
    assert_eq!(forms.len(), 4, "чотири незалежні SENS-самодокази");
    for form in &forms {
        eval_parsed_expressions(std::slice::from_ref(form), &mut Session::default())
            .expect("SENS відхилив власну позитивну логічну програму");
    }

    // One mutated D1 source bit must activate an invalid D1 call in the
    // *SENS program itself*. Rust observes rejection, not predicate meaning.
    let mutant = parse_canonical_binary(MUTANT).expect("канонічна рамка мутанта");
    assert_eq!(mutant.len(), 1);
    assert!(eval_parsed_expressions(&mutant, &mut Session::default()).is_err(),
        "сам SENS повинен відхилити хибну гілку без host truthiness");

    // The migrated library must still be loadable under the existing Core
    // implementation; this is a transport smoke test, not a semantic oracle.
    let mut session = Session::default();
    eval_pinned_legacy_core_source("lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session)
        .expect("чинний Core не завантажився");
    let parsed = parse_mixed_exact_domain_reasoning_source("lib/quantity.lisp", SOURCE)
        .expect("наявний reader не прийняв бібліотеку");
    eval_parsed_expressions(&parsed, &mut session)
        .expect("мігрована бібліотека не завантажилася");
}
