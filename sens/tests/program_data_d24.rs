//! Свідки перенесення даних компілятора на числові домени D24+.
//! Зовнішній тест не залежить від історичного #[cfg(test)] реєстру SID8.

use sens::{
    expr_to_exact_program_data, lowered_program_to_exact_data, Bija3, BinaryNumber, Bit3,
    CoreDomainIdentity, ErrorKind, Exactness, Expr, ExprKind, Rational, Span, Value,
};
use std::rc::Rc;

fn вузол(kind: ExprKind) -> Expr {
    Expr { kind, span: Span::default() }
}

fn доменне_число(width: usize) -> Expr {
    let mut bits = vec![0u8; width];
    bits[width - 1] = 1;
    вузол(ExprKind::BinaryNumber(
        BinaryNumber::from_numeric_domain_bits(width, &bits)
            .expect("явний ратифікований D24+"),
    ))
}

#[test]
fn тотожність_d24_і_d48_не_зливається_за_рівним_значенням() {
    let d24 = expr_to_exact_program_data(&доменне_число(24)).unwrap();
    let d48 = expr_to_exact_program_data(&доменне_число(48)).unwrap();
    assert_ne!(d24, d48);
    assert!(matches!(d24, Value::BinaryNumber(ref x) if x.numeric_domain_width() == Some(24)));
    assert!(matches!(d48, Value::BinaryNumber(ref x) if x.numeric_domain_width() == Some(48)));
}

#[test]
fn звичайні_числові_літерали_та_голі_біти_не_є_d24() {
    let old = [
        вузол(ExprKind::Number(1.0, Exactness::Exact)),
        вузол(ExprKind::Rational(Rational::from_literal("1", "2").unwrap())),
        вузол(ExprKind::BinaryNumber(BinaryNumber::parse("1").unwrap())),
    ];
    for expr in old {
        let error = expr_to_exact_program_data(&expr).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidForm);
    }
}

#[test]
fn вкладення_не_надає_старому_числу_нової_семантики() {
    let old = вузол(ExprKind::Number(3.0, Exactness::Exact));
    let list = вузол(ExprKind::List(Rc::from([old.clone()])));
    assert!(expr_to_exact_program_data(&list).is_err());
    let pair = вузол(ExprKind::Pair(Rc::new(доменне_число(24)), Rc::new(old.clone())));
    assert!(expr_to_exact_program_data(&pair).is_err());
    let quote = CoreDomainIdentity::D3(Bija3::from_word(Bit3::new(0b001).unwrap()));
    let call = вузол(ExprKind::DomainCall(quote, Rc::from([old])));
    assert!(expr_to_exact_program_data(&call).is_err());
}

#[test]
fn доменний_виклик_з_d24_проходить_точне_перетворення() {
    let quote = CoreDomainIdentity::D3(Bija3::from_word(Bit3::new(0b001).unwrap()));
    let call = вузол(ExprKind::DomainCall(quote, Rc::from([доменне_число(24)])));
    let program = lowered_program_to_exact_data(&[call]).expect("D3 і D24 узгоджені");
    assert!(matches!(program, Value::Pair(_, _)));
}
