//! Regression witnesses for the Lisp-owned Jev/System-One-like probabilistic decision slice.
//!
//! These tests exercise only the contract/data layer. There is no model call,
//! provider identity, new SID, or Rust Value variant.

use sens::{eval_program, load_core_library, Session};

fn eval_probabilistic_decision(source: &str) -> String {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    eval_program(
        include_str!("../../../lib/result-status.lisp"),
        &mut session,
    )
    .expect("result-status should load");
    eval_program(
        include_str!("../../../experiments/probabilistic-decision-jev-like.lisp"),
        &mut session,
    )
    .expect("probabilistic decision layer should load");
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn choice_observation_keeps_probability_confidence_and_provenance() {
    let source = r#"
        (let* ((question
                (pd-question-choice
                  (quote route)
                  (quote (billing technical))))
              (observation
                (pd-choice-observe
                  question
                  (quote ((billing 3/4) (technical 1/4)))
                  (quote ((source synthetic) (revision 1))))))
          (list
            (pd-observation-confidence observation)
            (pd-observation-provenance observation)
            (second (pd-choice-policy observation (quote billing) 2/3))))
    "#;

    assert_eq!(
        eval_probabilistic_decision(source),
        r#"(3/4 ((source synthetic) (revision 1)) accept)"#
    );
}

#[test]
fn invalid_choice_distribution_fails_closed_when_not_normalized() {
    let source = r#"
        (let* ((question
                (pd-question-choice
                  (quote route)
                  (quote (billing technical)))))
          (result-status
            (pd-choice-observe
              question
              (quote ((billing 2/3) (technical 1/4)))
              (quote ((source malformed))))))
    "#;

    assert_eq!(eval_probabilistic_decision(source), "invalid");
}

#[test]
fn noul_policy_is_explicit_threshold_control_not_truthiness() {
    let source = r#"
        (let* ((question (pd-question-noul (quote urgent)))
              (observation
                (pd-noul-observe
                  question
                  3/4
                  (quote ((source synthetic))))))
          (list
            (second (pd-noul-policy observation 2/3))
            (second (pd-noul-policy observation 4/5))
            (pd-observation-confidence observation)))
    "#;

    assert_eq!(eval_probabilistic_decision(source), "(accept defer 3/4)");
}

#[test]
fn score_observation_uses_the_same_typed_distribution_contract() {
    let source = r#"
        (let* ((question
                (pd-question-score
                  (quote severity)
                  (quote (low medium high))))
              (observation
                (pd-score-observe
                  question
                  (quote ((low 1/4) (medium 1/2) (high 1/4)))
                  (quote ((source synthetic))))))
          (list
            (car observation)
            (pd-observation-confidence observation)
            (pd-observation-provenance observation)))
    "#;

    assert_eq!(
        eval_probabilistic_decision(source),
        "(score-observation/1 1/2 ((source synthetic)))"
    );
}

#[test]
fn independent_questions_do_not_invent_joint_consistency() {
    let source = r#"
        (let ((left
                (pd-choice-observe
                  (pd-question-choice
                    (quote route)
                    (quote (billing technical)))
                  (quote ((billing 3/4) (technical 1/4)))
                  (quote ((source a)))))
              (right
                (pd-choice-observe
                  (pd-question-choice
                    (quote route-2)
                    (quote (billing technical)))
                  (quote ((billing 1/4) (technical 3/4)))
                  (quote ((source b))))))
          (car (pd-two-question-consistency left right (quote independent))))
    "#;

    assert_eq!(eval_probabilistic_decision(source), "decision-relation/1");
}

#[test]
fn declared_agreement_with_different_distributions_is_disputed() {
    let source = r#"
        (let ((left
                (pd-choice-observe
                  (pd-question-choice
                    (quote route)
                    (quote (billing technical)))
                  (quote ((billing 3/4) (technical 1/4)))
                  (quote ((source a)))))
              (right
                (pd-choice-observe
                  (pd-question-choice
                    (quote route)
                    (quote (billing technical)))
                  (quote ((billing 1/4) (technical 3/4)))
                  (quote ((source b))))))
          (result-status
            (pd-two-question-consistency left right (quote agree))))
    "#;

    assert_eq!(eval_probabilistic_decision(source), "disputed");
}

#[test]
fn noul_policy_rejects_out_of_domain_threshold() {
    let source = r#"
        (let* ((question (pd-question-noul (quote urgent)))
              (observation
                (pd-noul-observe
                  question
                  3/4
                  (quote ((source synthetic))))))
          (result-status
            (pd-noul-policy observation 3/2)))
    "#;

    assert_eq!(eval_probabilistic_decision(source), "invalid");
}

#[test]
fn choice_policy_rejects_out_of_domain_threshold() {
    let source = r#"
        (let* ((question
                (pd-question-choice
                  (quote route)
                  (quote (billing technical))))
              (observation
                (pd-choice-observe
                  question
                  (quote ((billing 3/4) (technical 1/4)))
                  (quote ((source synthetic))))))
          (result-status
            (pd-choice-policy observation (quote billing) 3/2)))
    "#;

    assert_eq!(eval_probabilistic_decision(source), "invalid");
}

#[test]
fn synthetic_witness_carries_one_state_through_three_typed_questions() {
    let source = r#"
        (let ((witness
                (pd-synthetic-witness
                  (quote ((case billing) (revision 1)))
                  (pd-question-choice
                    (quote route)
                    (quote (billing technical)))
                  (pd-question-noul (quote urgent))
                  (pd-question-score
                    (quote severity)
                    (quote (low medium high))))))
          (list
            (car witness)
            (second witness)
            (car (third witness))
            (pd-observation-confidence (fourth witness))
            (pd-observation-confidence (fifth witness))))
    "#;

    assert_eq!(
        eval_probabilistic_decision(source),
        "(probabilistic-witness/1 ((case billing) (revision 1)) choice-observation/1 2/3 1/2)"
    );
}
