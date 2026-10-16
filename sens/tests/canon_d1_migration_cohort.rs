//! Регресійна когорта #5029: виконуємо Canon напряму, без історичного
//! witness-runner (він мігрується окремо в #5025).
use sens::{eval_parsed_expressions, eval_program, load_core_library, parse_mixed_exact_domain, Session};

fn canon_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("current Core library");
    // Existing mixed-domain reader preserves compatibility D1 projections
    // while lifting canonical D3/D4 call heads into exact domain identities.
    let forms = parse_mixed_exact_domain(include_str!("../../../lib/canon.lisp"))
        .expect("Canon current D3/D4 source must parse");
    eval_parsed_expressions(&forms, &mut session)
        .expect("Canon source must load");
    session
}

#[test]
fn canon_constitutive_laws_use_current_predicate_results() {
    let mut session = canon_session();
    let cases: [(&str, &str); 13] = [
        ("canon-empty-list", "()"),
        ("(canon-law-empty-list)", "(canon-law-result empty-list satisfied)"),
        ("(canon-law-atom-cons (quote left) (quote right))", "(canon-law-result atom-cons satisfied)"),
        ("(canon-law-car-cons (quote left) (quote right))", "(canon-law-result car-cons satisfied)"),
        ("(canon-law-cdr-cons (quote left) (quote right))", "(canon-law-result cdr-cons satisfied)"),
        ("(canon-law-eq-reflexive-atom (quote left))", "(canon-law-result eq-reflexive-atom satisfied)"),
        ("(canon-law-cdr-dotted)", "(canon-law-result cdr-dotted satisfied)"),
        ("(canon-law-cdr-proper)", "(canon-law-result cdr-proper satisfied)"),
        ("(canon-law-cdr-improper)", "(canon-law-result cdr-improper satisfied)"),
        ("(canon-law-quote-suppresses-evaluation)", "(canon-law-result quote-suppresses-evaluation satisfied)"),
        ("(canon-law-cond-first-match-short-circuit)", "(canon-law-result cond-first-match-short-circuit satisfied)"),
        ("(canon-law-symbolic-surface)", "(canon-law-result symbolic-surface satisfied)"),
        ("(canon-conforms?)", "(canon-conformance satisfied)"),
    ];
    for (expr, expected) in cases {
        let actual = eval_program(expr, &mut session)
            .unwrap_or_else(|error| panic!("Canon evaluation failed: {expr}: {error}"))
            .value
            .to_string();
        assert_eq!(actual, expected, "Canon law {expr}");
    }
}

#[test]
fn canon_negative_predicate_polarity_and_exhaustion_are_not_truthiness() {
    let mut session = canon_session();
    // Пара не є атомом: YES і NO тут не взаємозамінні.
    let cases: [(&str, &str); 3] = [
        (
            "(canon-law-eq-reflexive-atom (quote (left right)))",
            "(canon-law-result eq-reflexive-atom violated)",
        ),
        (
            "(canon-conformance-from (quote ()))",
            "(canon-conformance satisfied)",
        ),
        (
            "(canon-conformance-from (quote ((canon-law-result example violated))))",
            "(canon-conformance violated)",
        ),
    ];
    for (expr, expected) in cases {
        let actual = eval_program(expr, &mut session)
            .unwrap_or_else(|error| panic!("Canon negative law failed: {expr}: {error}"))
            .value
            .to_string();
        assert_eq!(actual, expected, "Canon branch polarity {expr}");
    }
}
