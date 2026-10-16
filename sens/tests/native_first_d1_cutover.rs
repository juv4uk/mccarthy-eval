//! Runtime witness for thirty exact-D1 native-first source cuts.
//! Historical compatibility closures stay available to other machine files.
use sens::{
    eval_parsed_expressions, parse_mixed_exact_domain_machine_source, ErrorKind,
    Session,
};

const PATH: &str = "lib/machine/dispatch/native-first.lisp";
const SOURCE: &str = include_str!("../../../lib/machine/dispatch/native-first.lisp");

fn eval(session: &mut Session, source: &str) -> Result<sens::Value, sens::LanguageError> {
    let forms = parse_mixed_exact_domain_machine_source(PATH, source)?;
    Ok(eval_parsed_expressions(&forms, session)?.value)
}

fn session() -> Session {
    let mut s = Session::default();
    let defs = parse_mixed_exact_domain_machine_source(PATH, SOURCE)
        .expect("current exact machine reader");
    assert_eq!(defs.len(), 17);
    eval_parsed_expressions(&defs, &mut s)
        .expect("all 17 native-first machine definitions load");
    s
}

#[test]
fn inline_exact_d1_matches_both_unmodified_legacy_helpers() {
    let mut s = session();
    for (direct, previous, expected) in [
        ("#d1:1", "(native-first-domain-true (001 ()))", true),
        ("#d1:0", "(native-first-domain-false (001 ()))", false),
    ] {
        let new_value = eval(&mut s, direct).expect("exact domain call executable");
        let old_value = eval(&mut s, previous).expect("retained historical helper executable");
        assert_eq!(new_value.as_predicate_bit(), Some(expected));
        assert_eq!(old_value.as_predicate_bit(), Some(expected));
        assert_eq!(new_value, old_value);
    }
}

#[test]
fn real_native_first_classifier_still_rejects_unlowerable_forms() {
    let mut s = session();
    for argument in ["(001 ())", "(001 (symbol))"] {
        let source = format!("(native-first-plan {argument})");
        let value = eval(&mut s, &source).expect("native-first evaluation for unsupported form");
        assert!(value.to_string().starts_with("(evaluator-fallback "),
            "{source} must use evaluator-fallback; got {value}");
    }
}

#[test]
fn no_coercion_from_empty_list_or_numeric_one_into_d1() {
    let mut s = session();
    for source in ["(0100 (001 ()))", "(0100 1)"] {
        let error = eval(&mut s, source).expect_err("exact D4 NOT must reject non-D1");
        assert_eq!(error.kind, ErrorKind::Type);
    }
}

#[test]
fn exact_reader_does_not_generalize_machine_source_paths() {
    assert_eq!(SOURCE.matches("#d1:0").count(), 6);
    assert_eq!(SOURCE.matches("#d1:1").count(), 24);
    assert!(!SOURCE.contains("(native-first-domain-true ())"));
    assert!(!SOURCE.contains("(native-first-domain-false ())"));
    assert!(parse_mixed_exact_domain_machine_source("lib/untrusted.lisp", SOURCE).is_err());
}
