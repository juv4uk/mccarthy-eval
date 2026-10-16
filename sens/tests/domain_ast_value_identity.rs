use sens::{
    fasl_decode_program, fasl_encode_program, wire_decode_program, wire_encode_program, Bija3,
    Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9, CoreD4, CoreD5, CoreD6, CoreD8,
    CoreD9, CoreDomainIdentity, DomainIdentity, Exactness, Expr, ExprKind, PredicateBit, Racana2,
    D7DataWord, Span, Value,
};
use std::rc::Rc;

fn d1(raw: u8) -> DomainIdentity {
    PredicateBit::from_word(Bit1::new(raw).unwrap()).into()
}
fn d2(raw: u8) -> DomainIdentity {
    Racana2::from_word(Bit2::new(raw).unwrap()).into()
}
fn d3(raw: u8) -> DomainIdentity {
    Bija3::from_word(Bit3::new(raw).unwrap()).into()
}
fn d4(raw: u8) -> DomainIdentity {
    CoreD4::from_word(Bit4::new(raw).unwrap()).into()
}
fn d5(raw: u8) -> DomainIdentity {
    CoreD5::from_word(Bit5::new(raw).unwrap()).into()
}
fn d6(raw: u8) -> DomainIdentity {
    CoreD6::from_word(Bit6::new(raw).unwrap()).into()
}
fn d7(raw: u8) -> DomainIdentity {
    D7DataWord::from_word(Bit7::new(raw).unwrap()).into()
}
fn d8(raw: u8) -> DomainIdentity {
    CoreD8::from_word(Bit8::new(raw).unwrap()).into()
}
fn d9(raw: u16) -> DomainIdentity {
    CoreD9::from_word(Bit9::new(raw).unwrap()).into()
}

fn expr(identity: DomainIdentity) -> Expr {
    Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: Span { start: 0, end: 0 },
    }
}

#[test]
fn predicate_value_bridge_uses_only_exact_d1_domain_identity() {
    let yes = Value::predicate_bit(true);
    let no = Value::predicate_bit(false);

    assert_eq!(yes, Value::DomainIdentity(d1(1)));
    assert_eq!(no, Value::DomainIdentity(d1(0)));
    assert_eq!(yes.as_predicate_bit(), Some(true));
    assert_eq!(no.as_predicate_bit(), Some(false));

    assert_eq!(
        Value::Number(1.0, Exactness::Exact).as_predicate_bit(),
        None
    );
    assert_eq!(
        Value::Number(0.0, Exactness::Exact).as_predicate_bit(),
        None
    );
    assert_eq!(Value::Bool(true).as_predicate_bit(), None);
    assert_eq!(Value::Bool(false).as_predicate_bit(), None);
    assert_eq!(Value::Nil.as_predicate_bit(), None);
    assert_eq!(Value::DomainIdentity(d2(1)).as_predicate_bit(), None);
    assert_eq!(Value::DomainIdentity(d3(1)).as_predicate_bit(), None);

    assert_ne!(yes, Value::Number(1.0, Exactness::Exact));
    assert_ne!(no, Value::Number(0.0, Exactness::Exact));
    assert_ne!(yes, Value::DomainIdentity(d2(1)));
}

#[test]
fn same_payload_across_all_d1_d9_remains_distinct_in_ast_and_value() {
    let identities = [d1(1), d2(1), d3(1), d4(1), d5(1), d6(1), d7(1), d8(1), d9(1)];

    for identity in identities {
        assert_eq!(identity.packed_bits(), 1);
    }
    for left in 0..identities.len() {
        for right in left + 1..identities.len() {
            assert_ne!(identities[left], identities[right]);
            assert_ne!(expr(identities[left]).kind, expr(identities[right]).kind);
            assert_ne!(
                Value::DomainIdentity(identities[left]),
                Value::DomainIdentity(identities[right])
            );
        }
    }

    assert_eq!(Value::DomainIdentity(d1(1)).to_string(), "1");
    assert_eq!(Value::DomainIdentity(d2(1)).to_string(), "01");
    assert_eq!(Value::DomainIdentity(d7(1)).to_string(), "0000001");
    assert_eq!(Value::DomainIdentity(d8(1)).to_string(), "00000001");
    assert_eq!(Value::DomainIdentity(d9(1)).to_string(), "000000001");
    assert_eq!(d9(257).packed_bits(), 257);
    assert_ne!(d9(257), d9(1));
}

#[test]
fn every_domain_identity_round_trips_through_fasl_and_wire_without_width_loss() {
    for identity in [
        d1(1),
        d2(0b10),
        d3(0b101),
        d4(0b1010),
        d5(0b10101),
        d6(0b101010),
        d7(0b1010101),
        d8(0b10101010),
        d9(0b1_01010101),
    ] {
        let original = vec![expr(identity)];

        let fasl = fasl_encode_program(&original, &[7; 32]);
        let (fasl_decoded, hash) = fasl_decode_program(&fasl).expect("domain FASL");
        assert_eq!(hash, [7; 32]);
        assert_eq!(fasl_decoded, original);

        let wire = wire_encode_program(&original);
        let wire_decoded = wire_decode_program(&wire).expect("domain wire");
        assert_eq!(wire_decoded, original);
    }
}

#[test]
fn domain_call_uses_only_core_operation_identity_but_serializes_with_domain_head() {
    let identity = CoreDomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0010).unwrap()));
    let call = Expr {
        kind: ExprKind::DomainCall(
            identity,
            Rc::from([Expr {
                kind: ExprKind::Number(42.0, Exactness::Exact),
                span: Span { start: 0, end: 0 },
            }]),
        ),
        span: Span { start: 0, end: 0 },
    };

    let decoded = wire_decode_program(&wire_encode_program(&[call])).expect("domain call wire");
    let ExprKind::List(items) = &decoded[0].kind else {
        panic!("lowered domain call must serialize as source-shaped list");
    };
    assert!(matches!(
        items[0].kind,
        ExprKind::DomainIdentity(decoded_identity)
            if decoded_identity == DomainIdentity::from(identity)
    ));
    assert!(!matches!(items[0].kind, ExprKind::Sid(_)));
}
