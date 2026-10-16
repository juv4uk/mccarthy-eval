//! B3: a small adversarial Advice Taker corpus through one semantic path:
//! candidate knowledge -> advise/advise-all -> reason-in-observe -> narrate-outcome.
//! The cases are chosen for distinct failure/derivation modes, not fixture count.

use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source, eval_pinned_reasoning_source,
    parse_exact_domain_source, parse_mixed_exact_domain, Session,
};

fn eval_advice_corpus(source: &str) -> String {
    let mut session = Session::default();
    // Legacy eval_program treats bare 0011 as an arithmetic literal. The
    // repository-pinned Core reader keeps the original D4 DEFINE identity.
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("pinned exact-width SENS core bootstrap");
    for (library_path, library) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp")),
        // Admission and package-import policies now belong to immutable World.
        // The host only loads the exact-domain SENS source; it defines no law.
        ("lib/world.lisp", include_str!("../../../lib/world.lisp")),
        ("lib/understand.lisp", include_str!("../../../lib/understand.lisp")),
        ("lib/result-status.lisp", include_str!("../../../lib/result-status.lisp")),
        ("lib/narrate.lisp", include_str!("../../../lib/narrate.lisp")),
    ] {
        let result = match library_path {
            "lib/unify.lisp" | "lib/reason.lisp" | "lib/forward.lisp"
            | "lib/knowledge.lisp" | "lib/world.lisp" => {
                eval_pinned_reasoning_source(library_path, library, &mut session)
            }
            "lib/understand.lisp" => {
                let parsed = parse_mixed_exact_domain(library)
                    .unwrap_or_else(|error| panic!("{library_path}: source reader: {error}"));
                eval_parsed_expressions(&parsed, &mut session)
            }
            "lib/result-status.lisp" | "lib/narrate.lisp" => {
                let parsed = parse_exact_domain_source(library)
                    .unwrap_or_else(|error| panic!("{library_path}: exact reader: {error}"));
                eval_parsed_expressions(&parsed, &mut session)
            }
            _ => unreachable!("only the pinned corpus libraries are admitted"),
        };
        result
            .unwrap_or_else(|error| panic!("SENS corpus dependency {library_path} failed: {error}"));
    }
    // The authored corpus contains literal 3-bit QUOTE/CAR heads. The mixed
    // legacy reader may reinterpret 001/100 as numeric values (#q2:1/1).
    // The exact reader preserves their domain widths; W8/D8 remains distinct.
    let program = parse_exact_domain_source(source)
        .unwrap_or_else(|e| panic!("Advice Taker exact-domain reader: {e}\nsource: {source}"));
    eval_parsed_expressions(&program, &mut session)
        .unwrap_or_else(|e| panic!("Advice Taker exact-domain eval: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn direct_controlled_fact_reaches_proved_narration() {
    let source = r#"
        (advise astronomy (understand (001 (earth is a planet))))
        (narrate-outcome
          (reason-in-observe (001 astronomy) (001 (planet earth))))
    "#;
    assert_eq!(eval_advice_corpus(source), "(proved earth is a planet)");
}

#[test]
fn multistep_rules_survive_admission_reasoning_and_presentation() {
    let source = r#"
        (advise-all science
          (001 (
            ((planet earth))
            ((has (var x) mass) (planet (var x)))
            ((valuable (var x)) (has (var x) mass))
          )))
        (let* ((outcome
                 (reason-in-observe (001 science) (001 (valuable earth))))
               (words (narrate-outcome outcome)))
          (list (result-status outcome) (100 words)))
    "#;
    assert_eq!(eval_advice_corpus(source), "(proved proved)");
}

#[test]
fn recursive_rule_survives_the_same_end_to_end_path() {
    let source = r#"
        (advise-all family
          (001 (
            ((parent alice bob))
            ((parent bob charlie))
            ((ancestor (var x) (var y)) (parent (var x) (var y)))
            ((ancestor (var x) (var y))
              (parent (var x) (var z))
              (ancestor (var z) (var y)))
          )))
        (let* ((outcome
                 (reason-in-observe
                   (001 family)
                   (001 (ancestor alice charlie))))
               (words (narrate-outcome outcome)))
          (list (result-status outcome) (100 words)))
    "#;
    assert_eq!(eval_advice_corpus(source), "(proved proved)");
}

#[test]
fn explicit_conflict_is_rejected_and_cannot_replace_the_existing_fact() {
    let source = r#"
        (advise ethics (001 ((mortal socrates))))
        (let* ((decision
                 (advise ethics (001 ((not? (mortal socrates))))))
               (outcome
                 (reason-in-observe (001 ethics) (001 (mortal socrates)))))
          (list (100 decision) (result-status outcome)))
    "#;
    assert_eq!(eval_advice_corpus(source), "(conflict proved)");
}

#[test]
fn knowledge_package_round_trip_precedes_reasoning_without_eval_of_package_data() {
    let source = r#"
        (def package
          (make-knowledge-package
            (001 archive)
            (001 (((planet mars))))))
        (def *knowledge-journal* (001 ()))
        (import-knowledge-package package)
        (narrate-outcome
          (reason-in-observe (001 archive) (001 (planet mars))))
    "#;
    assert_eq!(eval_advice_corpus(source), "(proved mars is a planet)");
}
