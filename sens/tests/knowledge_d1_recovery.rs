//! Real executable contract: three recovered knowledge predicates/accessors.
//! No whole-world bootstrap, D24 numeric alias, or D7 historical reinterpretation.
use sens::{eval_parsed_expressions, parse_mixed_exact_domain, Session, Value};

const SRC: &str = include_str!("../../../lib/knowledge.lisp");
const START: &str = "(0011 knowledge-proper-list?";

fn session() -> Session {
    let offset = SRC.find(START).expect("knowledge current validated suffix");
    let forms = parse_mixed_exact_domain(&SRC[offset..])
        .expect("recovered knowledge must parse in current exact D3/D4 source");
    // 3 current functions + exactly 3 recovered definitions.
    assert_eq!(forms.len(), 6);
    let mut s = Session::default();
    eval_parsed_expressions(&forms, &mut s)
        .expect("pure knowledge predicates must load with current domain identities");
    s
}

fn eval(s: &mut Session, code: &str) -> sens::Value {
    let forms = parse_mixed_exact_domain(code)
        .unwrap_or_else(|err| panic!("{code}: syntax {err:?}"));
    eval_parsed_expressions(&forms, s)
        .unwrap_or_else(|err| panic!("{code}: runtime {err:?}"))
        .value
}

#[test]
fn both_negation_spelling_aliases_have_exact_d1() {
    let mut s = session();
    for (token, expected) in [("not", true), ("not?", true), ("plain", false)] {
        let source = format!("(knowledge-not-head? (001 {token}))");
        assert_eq!(eval(&mut s, &source).as_predicate_bit(), Some(expected), "{source}");
    }
}

#[test]
fn package_field_returns_data_and_missing_field_is_d3_empty() {
    let mut s = session();
    let yes = eval(&mut s,
        "(knowledge-package-field (001 author) (001 ((author a) (version b))))");
    assert_eq!(yes.to_string(), "(a)");
    let missing = eval(&mut s,
        "(knowledge-package-field (001 missing) (001 ((author a) (version b))))");
    assert!(matches!(missing, Value::Nil));
    assert_eq!(missing.as_predicate_bit(), None);
}

#[test]
fn list_validation_follows_exact_d1_not_host_truthiness() {
    let mut s = session();
    for (source, expected) in [
        ("(knowledge-clauses-valid? (001 ()))", true),
        ("(knowledge-clauses-valid? (001 malformed))", false),
        ("(knowledge-clauses-valid? (001 ((p x))))", true),
    ] {
        assert_eq!(eval(&mut s, source).as_predicate_bit(), Some(expected), "{source}");
    }
}

#[test]
fn strict_d1_false_is_never_structural_nil() {
    let mut s = session();
    let b = eval(&mut s, "(knowledge-not-head? (001 plain))");
    let nil = eval(&mut s,
        "(knowledge-package-field (001 missing) (001 ()))");
    assert_eq!(b.as_predicate_bit(), Some(false));
    assert_eq!(nil.as_predicate_bit(), None);
}
