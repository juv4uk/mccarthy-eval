//! Migration proof: direct D1 answers in the active native-first execution bridge.
//! No CPU/x86 mechanism required; unsupported plan routes remain explicit debt.
use sens::{
    eval_parsed_expressions, parse, parse_mixed_exact_domain_machine_source, DomainIdentity,
    ErrorKind, ExprKind, Session,
};
const PATH: &str = "lib/machine/dispatch/native-first-execute.lisp";
const SOURCE: &str = include_str!("../../../lib/machine/dispatch/native-first-execute.lisp");

fn eval(s: &mut Session, expression: &str) -> Result<sens::Value, sens::LanguageError> {
    let ast = parse_mixed_exact_domain_machine_source(PATH, expression)?;
    Ok(eval_parsed_expressions(&ast, s)?.value)
}
fn session() -> Session {
    let mut s = Session::default();
    let ast = parse_mixed_exact_domain_machine_source(PATH, SOURCE)
        .expect("all 13 current-domain bridge forms parse");
    assert_eq!(ast.len(), 13);
    eval_parsed_expressions(&ast, &mut s).expect("install full execution bridge");
    s
}
#[test]
fn exact_d1_words_not_synthetic_predicate_calls() {
    for (source, expected) in [("#d1:1", true), ("#d1:0", false)] {
        let ast = parse(source).expect("compatibility reader D1 projection");
        assert!(matches!(&ast[0].kind, ExprKind::DomainIdentity(DomainIdentity::D1(_))));
        let mut s = Session::default();
        assert_eq!(eval(&mut s, source).unwrap().as_predicate_bit(), Some(expected));
    }
    for source in ["0", "1", "(001 ())"] {
        let parsed = parse(source).unwrap();
        assert!(
            !matches!(&parsed[0].kind, ExprKind::DomainIdentity(DomainIdentity::D1(_))),
            "numeric/structural source may not alias exact D1: {source}"
        );
    }
}
#[test]
fn real_bridge_accepts_both_d1_answers_and_keeps_short_circuit() {
    let mut s = session();
    for (p, result) in [
        ("#d1:1", "selected"),
        ("#d1:0", "fallback"),
    ] {
        let source = format!("(110 ({p} (001 selected)) (#d1:1 (001 fallback)))");
        assert_eq!(eval(&mut s, &source).unwrap().to_string(), result);
    }
    assert_eq!(
        eval(&mut s, "(110 (#d1:1 (001 selected)) (#d1:1 (non-existent-operation)))")
            .expect("unreached branch must not execute").to_string(),
        "selected"
    );
}
#[test]
fn bridge_maintains_explicit_outcome_shapes_without_cpu() {
    let mut s = session();
    let completed = eval(&mut s,
        "(native-first-execution-completed (001 evaluator) (001 success))")
        .expect("route envelope construction");
    assert!(completed.to_string().contains("execution-route"));
    assert!(completed.to_string().contains("completed"));
    let rejected = eval(&mut s,
        "(native-first-execution-rejected (001 native) (001 denied))")
        .expect("native rejection is data, not hidden evaluator fallback");
    assert!(rejected.to_string().contains("rejected"));
    assert!(rejected.to_string().contains("denied"));
}
#[test]
fn invalid_predicate_types_and_binder_mismatch_fail_closed() {
    let mut s = session();
    let error = eval(&mut s, "(110 ((001 ()) (001 false)) (#d1:1 (001 fallback)))")
        .expect_err("empty structure is not logical NO");
    assert_eq!(error.kind, ErrorKind::Type);
    let error = eval(&mut s, "(0100 (001 ()))")
        .expect_err("NOT must reject structural nil");
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(parse_mixed_exact_domain_machine_source("lib/not-machine.lisp", SOURCE).is_err());
}
