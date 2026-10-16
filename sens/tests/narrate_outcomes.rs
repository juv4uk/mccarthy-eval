//! B2: presentation regressions use the current exact-domain source readers.

use sens::{
    eval_parsed_expressions, load_core_library, parse_exact_domain_source, parse_mixed_exact_domain,
    parse_mixed_exact_domain_reasoning_source, Session,
};

fn eval_outcome_narration(source: &str) -> String {
    let mut session = Session::default();
    load_core_library(&mut session)
        .expect("canonical Core4 bootstrap must load");
    for (path, source) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
    ] {
        let parsed = parse_mixed_exact_domain_reasoning_source(path, source)
            .unwrap_or_else(|error| panic!("{path}: exact reasoning reader: {error}"));
        eval_parsed_expressions(&parsed, &mut session)
            .unwrap_or_else(|error| panic!("{path}: exact reasoning evaluator: {error}"));
    }
    for (path, source) in [
        ("lib/result-status.lisp", include_str!("../../../lib/result-status.lisp")),
        ("lib/narrate.lisp", include_str!("../../../lib/narrate.lisp")),
    ] {
        let parsed = parse_exact_domain_source(source)
            .unwrap_or_else(|error| panic!("{path}: exact-domain reader: {error}"));
        eval_parsed_expressions(&parsed, &mut session)
            .unwrap_or_else(|error| panic!("{path}: exact-domain evaluator: {error}"));
    }
    let expressions = parse_mixed_exact_domain(source)
        .unwrap_or_else(|error| panic!("fixture exact reader: {error}\nsource: {source}"));
    eval_parsed_expressions(&expressions, &mut session)
        .unwrap_or_else(|error| panic!("evaluation failed: {error}\nsource: {source}"))
        .value
        .to_string()
}
#[test]
fn proved_outcome_keeps_the_ground_answer_and_real_premise() {
    let source = r#"
        (let* ((rules (quote (((has (var x) mass) (planet (var x)))
                              ((planet earth)))))
               (goal (quote (has earth mass))))
          (narrate-outcome (reason-observe goal rules)))
    "#;
    assert_eq!(
        eval_outcome_narration(source),
        "(proved earth has mass because earth is a planet)"
    );
}

#[test]
fn disputed_outcome_keeps_both_evidence_sides_visible() {
    let source = r#"
        (let* ((rules (quote (((mortal socrates))
                              ((not? (mortal socrates))))))
               (narration
                 (narrate-outcome
                   (reason-observe (quote (mortal socrates)) rules))))
          (list (car narration)
                (second narration)
                (third narration)
                (length (fourth narration))))
    "#;
    assert_eq!(
        eval_outcome_narration(source),
        "(disputed because both-sides-have-evidence 2)"
    );
}

#[test]
fn invalid_outcome_names_validation_failure_and_payload() {
    assert_eq!(
        eval_outcome_narration(
            r#"(narrate-outcome (make-invalid (quote invalid-goal) 42))"#
        ),
        "(invalid because invalid-goal payload 42)"
    );
}

#[test]
fn partial_and_blocked_do_not_collapse_into_unknown() {
    assert_eq!(
        eval_outcome_narration(
            r#"(narrate-outcome (make-partial (quote ()) (quote (depth . 12))))"#
        ),
        "(partial value () bound (depth . 12))"
    );
    assert_eq!(
        eval_outcome_narration(
            r#"(narrate-outcome (make-blocked (quote (depends-on corpus))))"#
        ),
        "(blocked because (depends-on corpus))"
    );
}

#[test]
fn malformed_or_unknown_outcome_shape_is_presented_as_invalid() {
    assert_eq!(
        eval_outcome_narration(r#"(narrate-outcome 42)"#),
        "(invalid outcome-shape 42)"
    );
    assert_eq!(
        eval_outcome_narration(r#"(narrate-outcome (quote (mystery payload)))"#),
        "(invalid outcome-tag mystery)"
    );
    assert_eq!(
        eval_outcome_narration(r#"(narrate-outcome (quote (unknown)))"#),
        "(invalid outcome-shape (unknown))"
    );
    assert_eq!(
        eval_outcome_narration(r#"(narrate-outcome (quote ((bad-tag) payload)))"#),
        "(invalid outcome-tag (bad-tag))"
    );
}
