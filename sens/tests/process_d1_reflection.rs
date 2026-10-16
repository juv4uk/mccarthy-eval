//! Незалежний точковий виконавчий свідок п'яти D4 NOT над точним D1 EQ.
//! Працює без процесу ОС та UTF-8/D24+ бібліотеки.
use sens::{
    eval_parsed_expressions, parse_mixed_exact_domain_process_tcp_source,
    Expr, ExprKind, ErrorKind, Session,
};

const SOURCE: &str = include_str!("../../../lib/process.lisp");
const PATH: &str = "lib/process.lisp";

fn visit(expr: &Expr, found: &mut usize) {
    if let ExprKind::List(items) = &expr.kind {
        if let Some(head) = items.first() {
            if let ExprKind::DomainIdentity(code) = head.kind {
                if code.width() == 4 && code.packed_bits() == 0b0100 {
                    assert_eq!(items.len(), 2, "D4:0100 NOT needs one D1 operand");
                    let ExprKind::List(inner) = &items[1].kind else {
                        panic!("NOT must consume the D3 EQ expression");
                    };
                    let ExprKind::DomainIdentity(eq) = inner[0].kind else {
                        panic!("EQ must be exact D3 identity");
                    };
                    assert_eq!((eq.width(), eq.packed_bits()), (3, 0b101));
                    assert_eq!(inner.len(), 3, "EQ must have two arguments");
                    *found += 1;
                }
            }
        }
        for part in items.iter() {
            visit(part, found);
        }
    }
}

#[test]
fn five_live_process_negations_are_real_d4_d3_domains() {
    let parsed = parse_mixed_exact_domain_process_tcp_source(PATH, SOURCE)
        .expect("must read process.lisp through its approved exact-domain loader");
    let mut count = 0;
    for form in &parsed {
        visit(form, &mut count);
    }
    assert_eq!(count, 5, "all five process negations must become exact D4");
    assert!(!SOURCE.contains("(тотожне?"), "no human EQ-to-NO alias may survive");
}

fn eval_exact(source: &str) -> Result<sens::Value, sens::LanguageError> {
    let expressions = parse_mixed_exact_domain_process_tcp_source(PATH, source)?;
    Ok(eval_parsed_expressions(&expressions, &mut Session::default())?.value)
}

#[test]
fn domain_not_eq_truth_table_and_strict_fail_closed() {
    for (source, expected) in [
        ("(0100 (101 (001 decoded) (001 decoded)))", false),
        ("(0100 (101 (001 decoded) (001 rejected)))", true),
    ] {
        let actual = eval_exact(source).expect("D3 EQ and D4 NOT must run");
        assert_eq!(
            actual.as_predicate_bit(), Some(expected),
            "{source}: exact D1 result is required, never numeric truthiness"
        );
    }
    let failure = eval_exact("(0100 (001 ()))")
        .expect_err("D3 structural empty must never coerce into D1 NO");
    assert_eq!(failure.kind, ErrorKind::Type);
    assert!(failure.message.contains("D1"), "strict D1 rejection: {failure:?}");
}

#[test]
fn path_pinned_process_reader_does_not_admit_ambient_files() {
    assert!(parse_mixed_exact_domain_process_tcp_source(
        "lib/random.lisp",
        "(0100 (101 (001 yes) (001 no)))",
    ).is_err());
}
