//! Public exact-domain migration boundary for host consumers.
//! The historical reader is never a fallback for this API.
use sens::{
    eval_exact_domain_program, parse, parse_exact_domain_source, ErrorKind, ExprKind,
    Session, Value,
};

#[test]
fn public_reader_keeps_d9_width_and_is_not_the_historical_byte_reader() {
    let current = parse_exact_domain_source("100000001")
        .expect("D9 word must be decoded without byte truncation");
    assert!(matches!(
        &current[0].kind,
        ExprKind::DomainIdentity(identity)
            if identity.width() == 9 && identity.packed_bits() == 0b100000001
    ));

    let historical = parse("11110111").expect("historical source is isolated");
    let current8 = parse_exact_domain_source("11110111")
        .expect("current D8 source is explicit");
    assert!(matches!(&historical[0].kind, ExprKind::Sid(_)));
    assert!(matches!(
        &current8[0].kind,
        ExprKind::DomainIdentity(identity)
            if identity.width() == 8 && identity.packed_bits() == 0b11110111
    ));
}

#[test]
fn public_execution_quotes_current_d8_without_historical_dispatch() {
    let result = eval_exact_domain_program("(001 11110111)", &mut Session::bare())
        .expect("D3 QUOTE is executable as an admitted exact-domain form");
    assert!(matches!(
        result.value,
        Value::DomainIdentity(identity)
            if identity.width() == 8 && identity.packed_bits() == 0b11110111
    ));
}

#[test]
fn public_execution_rejects_noncallable_d8_and_unqualified_math() {
    let error = eval_exact_domain_program("(11110110 (001 1))", &mut Session::bare())
        .expect_err("ratified residency is not a runtime mechanism");
    assert_eq!(error.kind, ErrorKind::Type);

    let error = eval_exact_domain_program("(001 42)", &mut Session::bare())
        .expect_err("D24+ numeric grammar cannot be silently replaced by decimal");
    assert_eq!(error.kind, ErrorKind::Parse);
}
