//! B4 external translation boundary.
//! The translator may propose versioned Lisp data; only Lisp validation and
//! the existing advice boundary decide whether knowledge is admissible.

use sens::{eval_program, parse, Session};

fn eval_translation(source: &str) -> String {
    let mut session = Session::default();
    for library in [
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/forward.lisp"),
        include_str!("../../../lib/knowledge.lisp"),
        include_str!("../../../lib/result-status.lisp"),
        include_str!("../../../lib/translation.lisp"),
    ] {
        eval_program(library, &mut session).unwrap();
    }
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("translation evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn accepted_translation_review_is_pure_and_only_exposes_an_admission_payload() {
    let source = r#"
        (def before *knowledge-journal*)
        (def proposal
          (quote (translation/1 candidate clause
                   "Socrates is human."
                   ((human socrates)))))
        (def review (translation-review (quote corpus) proposal))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (equal? before *knowledge-journal*)
          (equal? (translation-admission-payload review)
                  (quote ((human socrates)))))
    "#;
    assert_eq!(
        eval_translation(source),
        "(accepted knowledge-accepted (1) (1))"
    );
}

#[test]
fn accepted_candidate_becomes_knowledge_only_after_explicit_advise() {
    let source = r#"
        (def proposal
          (quote (translation/1 candidate clause
                   "Socrates is human."
                   ((human socrates)))))
        (def review (translation-review (quote corpus) proposal))
        (def before-known (module-known? (quote corpus)))
        (def admission (advise corpus (translation-admission-payload review)))
        (list
          before-known
          (car admission)
          (result-status
            (reason-in-observe (quote corpus) (quote (human socrates)))))
    "#;
    assert_eq!(eval_translation(source), "(() accepted proved)");
}

#[test]
fn ambiguous_translation_is_evidence_not_knowledge() {
    let source = r#"
        (def before *knowledge-journal*)
        (def proposal
          (quote (translation/1 ambiguous clause
                   "Mercury is hot."
                   (((hot mercury-planet))
                    ((hot mercury-element))))))
        (def review (translation-review (quote corpus) proposal))
        (def evidence (translation-evidence-next review (quote ())))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (length evidence)
          (equal? before *knowledge-journal*)
          (translation-admittable? review))
    "#;
    assert_eq!(
        eval_translation(source),
        "(ambiguous translator-ambiguous 1 (1) ())"
    );
}

#[test]
fn malformed_semantic_candidate_is_rejected_and_cannot_reach_advice() {
    let source = r#"
        (def before *knowledge-journal*)
        (def proposal
          (quote (translation/1 candidate clause
                   "Socrates is human."
                   ((human (var))))))
        (def review (translation-review (quote corpus) proposal))
        (def evidence (translation-evidence-next review (quote ())))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (length evidence)
          (equal? before *knowledge-journal*)
          (translation-admission-payload review))
    "#;
    assert_eq!(
        eval_translation(source),
        "(rejected invalid-candidate 1 (1) ())"
    );
}

#[test]
fn translator_refusal_is_recordable_evidence_and_never_knowledge() {
    let source = r#"
        (def before *knowledge-journal*)
        (def proposal
          (quote (translation/1 rejected clause
                   "Colorless green ideas sleep furiously."
                   unsupported-translation)))
        (def review (translation-review (quote corpus) proposal))
        (def evidence (translation-evidence-next review (quote ())))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (length evidence)
          (equal? before *knowledge-journal*)
          (translation-admission-payload review))
    "#;
    assert_eq!(
        eval_translation(source),
        "(rejected translator-rejected 1 (1) ())"
    );
}

#[test]
fn existing_explicit_opposite_overrules_an_external_candidate() {
    let source = r#"
        (advise corpus (quote ((not? (human socrates)))))
        (def before *knowledge-journal*)
        (def proposal
          (quote (translation/1 candidate clause
                   "Socrates is human."
                   ((human socrates)))))
        (def review (translation-review (quote corpus) proposal))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (equal? before *knowledge-journal*)
          (translation-admission-payload review))
    "#;
    assert_eq!(
        eval_translation(source),
        "(rejected knowledge-conflict (1) ())"
    );
}

#[test]
fn accepted_query_is_a_question_not_a_knowledge_write() {
    let source = r#"
        (def before *knowledge-journal*)
        (def proposal
          (quote (translation/1 candidate query
                   "Is Plato metallic?"
                   (metallic plato))))
        (def review (translation-review (quote corpus) proposal))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (translation-admission-payload review)
          (equal? before *knowledge-journal*)
          (result-status
            (reason-in-observe (quote corpus) (translation-payload proposal))))
    "#;
    assert_eq!(eval_translation(source), "(accepted query () (1) blocked)");
}

#[test]
fn one_alternative_is_not_allowed_to_masquerade_as_ambiguity() {
    let source = r#"
        (def proposal
          (quote (translation/1 ambiguous clause
                   "Mercury is hot."
                   (((hot mercury))))))
        (def review (translation-review (quote corpus) proposal))
        (list
          (translation-review-status review)
          (translation-review-code review)
          (translation-admission-payload review))
    "#;
    assert_eq!(
        eval_translation(source),
        "(rejected invalid-ambiguity ())"
    );
}

#[test]
fn malformed_protocol_envelope_is_named_before_semantic_admission() {
    let source = r#"
        (def review
          (translation-review
            (quote corpus)
            (quote (translation/2 candidate clause "text" ((human socrates))))))
        (list
          (translation-review-status review)
          (translation-review-code review))
    "#;
    assert_eq!(eval_translation(source), "(rejected invalid-translation)");
}

#[test]
fn versioned_translation_corpus_is_data_only_and_contains_all_b4_modes() {
    let corpus = include_str!("../../../tests/fixtures/translation-corpus-v1.lisp");
    let forms = parse(corpus).expect("translation corpus must remain parseable data");
    assert_eq!(forms.len(), 1, "corpus must be one versioned data value");

    assert!(corpus.contains("(translation-corpus/1"));
    let case_count = corpus
        .lines()
        .filter(|line| line.trim_start().starts_with("(translation-case/1"))
        .count();
    assert_eq!(case_count, 6);

    for id in [
        "direct-fact",
        "rule-batch",
        "valid-question-without-proof",
        "malformed-semantic-candidate",
        "lexical-ambiguity",
        "translator-refusal",
    ] {
        assert!(corpus.contains(id), "missing corpus case {id}");
    }

    for status in ["accepted", "rejected", "ambiguous"] {
        assert!(
            corpus.contains(status),
            "corpus must exercise review status {status}"
        );
    }
    for outcome in ["proved", "unknown", "not-run"] {
        assert!(
            corpus.contains(outcome),
            "corpus must name downstream outcome {outcome}"
        );
    }
}
