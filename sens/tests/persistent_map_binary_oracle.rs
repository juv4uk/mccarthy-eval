//! Mechanical carrier only. SENS binary programs own all semantic laws,
//! conditional branches, map identity, and negative mutation outcomes.
use sens::{eval_parsed_expressions, parse_canonical_binary, Session};

const LAW: &str = include_str!("../../../tests/semantic/persistent-map-d1-law.bits");
const MUTANT: &str = include_str!("../../../tests/semantic/persistent-map-d1-mutant.bits");

fn verdict(binary: &str) -> bool {
    assert!(binary.bytes().all(|b| matches!(b, b'0' | b'1' | b' ' | b'\n')),
        "semantic logic must be authored only as binary SENS");
    let forms = parse_canonical_binary(binary).expect("canonical exact-domain SENS");
    assert_eq!(forms.len(), 1, "one SENS-owned logical verdict");
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    ).expect("ratified bootstrap");
    sens::eval_pinned_reasoning_source(
        "lib/persistent-map.lisp",
        include_str!("../../../lib/persistent-map.lisp"),
        &mut session,
    ).expect("exact D3/D4 map laws");
    eval_parsed_expressions(&forms, &mut session)
        .expect("SENS-owned logic must execute").value
        .as_predicate_bit()
        .expect("semantic law must answer exactly one D1 bit")
}

#[test]
fn map_laws_are_proved_only_by_binary_sens_and_mutation_is_rejected() {
    assert!(verdict(LAW), "SENS proof yielded D1:NO");
    assert!(!verdict(MUTANT), "SENS mutant must yield D1:NO");
}
