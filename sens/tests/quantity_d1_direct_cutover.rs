//! Focused executable parity of strict D1 status/kind answers.
//! Only two arithmetic-independent definitions are loaded, never suppressing D24+ failures.
use sens::{
    eval_parsed_expressions, parse_mixed_exact_domain, Session, Value,
};
const SRC: &str = include_str!("../../../lib/quantity.lisp");

const STATUS: &[&str] = &["exact-by-definition", "exact-derived", "measured"];
const KIND: &[&str] = &[
    "physical-defining", "physical-derived", "physical-measured", "mathematical"
];

fn admitted_slice(current: bool) -> String {
    let begin = SRC.find("(0011 scientific-constant-status-valid?")
        .expect("current status definition");
    let end = SRC.find("(0011 make-scientific-constant")
        .expect("next unrelated definition");
    let scope = SRC[begin..end].to_owned();
    if current {
        scope
    } else {
        scope.replace("#d1:1", "(010 (001 ()))")
    }
}

fn evaluator(source: &str) -> Session {
    let forms = parse_mixed_exact_domain(source).expect("two D3/D4 definitions");
    assert_eq!(forms.len(), 2);
    let mut session = Session::default();
    eval_parsed_expressions(&forms, &mut session).expect("definitions accepted");
    session
}

fn run(session: &mut Session, function: &str, argument: &str) -> Value {
    let source = format!("({function} (001 {argument}))");
    let expr = parse_mixed_exact_domain(&source).expect("D3 QUOTE argument");
    eval_parsed_expressions(&expr, session)
        .unwrap_or_else(|err| panic!("{source}: {err:?}"))
        .value
}

#[test]
fn seven_admitted_status_kind_answers_are_exact_one_bit_yes() {
    let mut current = evaluator(&admitted_slice(true));
    let mut old = evaluator(&admitted_slice(false));
    for (func, options) in [
        ("scientific-constant-status-valid?", STATUS),
        ("scientific-constant-kind-valid?", KIND)
    ] {
        for option in options {
            let new_result = run(&mut current, func, option);
            let old_result = run(&mut old, func, option);
            assert_eq!(new_result.as_predicate_bit(), Some(true), "{func}/{option}");
            assert_eq!(new_result, old_result);
        }
    }
}

#[test]
fn disallowed_statuses_preserve_no_match_as_structural_empty() {
    // COND exhaustion is D3:EMPTY per the current contract, not D1:NO.
    let mut current = evaluator(&admitted_slice(true));
    let mut old = evaluator(&admitted_slice(false));
    for (func, options) in [
        ("scientific-constant-status-valid?", &["unknown", "physical-defining"][..]),
        ("scientific-constant-kind-valid?", &["unknown", "measured"][..]),
    ] {
        for option in options {
            let new_result = run(&mut current, func, option);
            let old_result = run(&mut old, func, option);
            assert!(matches!(new_result, Value::Nil), "{func}/{option}: no-match D3 empty");
            assert_eq!(new_result.as_predicate_bit(), None);
            assert_eq!(new_result, old_result);
        }
    }
}

#[test]
fn d1_direct_words_and_empty_are_distinct_in_same_reader() {
    let mut session = Session::default();
    for (source, expected) in [("#d1:0", false), ("#d1:1", true)] {
        let parsed = parse_mixed_exact_domain(source).unwrap();
        let value = eval_parsed_expressions(&parsed, &mut session).unwrap().value;
        assert_eq!(value.as_predicate_bit(), Some(expected));
    }
    let parsed = parse_mixed_exact_domain("(001 ())").unwrap();
    let value = eval_parsed_expressions(&parsed, &mut session).unwrap().value;
    assert!(matches!(value, Value::Nil));
    assert_eq!(value.as_predicate_bit(), None);
}

#[test]
fn cond_short_circuit_does_not_evaluate_unsupported_branch() {
    let source = "(110 (#d1:1 (001 chosen)) (#d1:1 (unknown-unreachable)))";
    let parsed = parse_mixed_exact_domain(source).expect("D3 COND");
    let output = eval_parsed_expressions(&parsed, &mut Session::default())
        .expect("only first selected branch evaluated")
        .value;
    assert_eq!(output.to_string(), "chosen");
}
