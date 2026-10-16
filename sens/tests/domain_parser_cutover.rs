//! Позитивні й негативні свідки доменного читача ядра, без cfg(test) залежностей.

use sens::{parse, parse_exact_domain_source, DomainIdentity, ErrorKind, Expr, ExprKind};

fn exact(source: &str) -> Vec<Expr> {
    parse_exact_domain_source(source).expect("точна доменна програма")
}

#[test]
fn точні_слова_не_зводяться_до_числа_чи_байтового_sid() {
    let source = "0 1 10 001 0001 0000001 100000001";
    let expressions = exact(source);
    let actual: Vec<_> = expressions.iter().map(|expression| {
        let ExprKind::DomainIdentity(identity) = expression.kind else {
            panic!("доменний читач не може видати число чи SID");
        };
        (identity.width(), identity.packed_bits())
    }).collect();
    assert_eq!(actual, [
        (1, 0), (1, 1), (2, 2), (3, 1), (4, 1), (7, 1), (9, 257)
    ]);
    assert_ne!(expressions[3].kind, expressions[4].kind,
        "рівні числові payload D3 і D4 не означають однакову семантику");
    assert!(matches!(expressions[0].kind, ExprKind::DomainIdentity(DomainIdentity::D1(_))));
    assert!(matches!(expressions[5].kind, ExprKind::DomainIdentity(DomainIdentity::D7(_))));
}

#[test]
fn точний_читач_зберігає_структуру_цитату_та_utf8_діапазони() {
    let source = "; примітка\n(001 (100 змінна)) '001 \"10101000\"";
    let expressions = exact(source);
    assert_eq!(expressions.len(), 3);
    let ExprKind::List(outer) = &expressions[0].kind else { panic!("список") };
    assert!(matches!(outer[0].kind, ExprKind::DomainIdentity(DomainIdentity::D3(_))));
    let ExprKind::List(inner) = &outer[1].kind else { panic!("вкладений список") };
    assert!(matches!(inner[0].kind, ExprKind::DomainIdentity(DomainIdentity::D3(_))));
    assert!(matches!(&inner[1].kind, ExprKind::Symbol(name) if name.as_ref() == "змінна"));
    assert_eq!(&source[inner[1].span.start..inner[1].span.end], "змінна");
    let ExprKind::List(quoted) = &expressions[1].kind else { panic!("цитата") };
    assert!(matches!(quoted[0].kind, ExprKind::DomainIdentity(DomainIdentity::D3(_))));
    assert!(matches!(quoted[1].kind, ExprKind::DomainIdentity(DomainIdentity::D3(_))));
    assert!(matches!(&expressions[2].kind, ExprKind::String(value) if value.as_ref() == "10101000"));
}

#[test]
fn невідомий_w8_та_ширина_d10_не_стають_d8_чи_числом() {
    for source in ["10101000", "(10101000 x)", "'10101000", "00000001",
        "0000000001", "(0000000001 x)"] {
        let error = parse_exact_domain_source(source).expect_err("W8/D10 BLOCK");
        assert_eq!(error.kind, ErrorKind::Parse, "{source}");
    }
    // Транспортне значення W8, прочитане старим API, лишається тільки старим.
    assert!(matches!(parse("10101000").unwrap()[0].kind, ExprKind::Sid(_)));
}

#[test]
fn математичні_літерали_без_d24_відхиляються_без_fallback() {
    for source in ["7", "12", "-2", "3.5", "3/2", "#d10", "#b1010",
        "#q2:11/1", "#i32(1)", "#f32(1.0)", "(001 12)"] {
        let error = parse_exact_domain_source(source).expect_err("тільки D24+");
        assert_eq!(error.kind, ErrorKind::Parse, "{source}");
    }
    // Цитована цифра 1 вже є доменним словом D1, не літералом.
    assert!(matches!(exact("'1")[0].kind, ExprKind::List(_)));
}
