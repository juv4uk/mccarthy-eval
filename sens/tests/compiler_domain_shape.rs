//! sens#3808 — representation-only DomainIdentity decomposition.
//!
//! The host mechanism intentionally exposes only exact width and exact bits.
//! SENS receives it as a first-class function value; no semantic operation is
//! registered globally.  Role derivation belongs to sens#3809.

use sens::syntax::{Expr, ExprKind, Span};
use sens::{
    domain_identity_shape_mechanism, eval_parsed_expressions, load_core_library, parse_mixed_exact_domain,
    Bija3, Bit3, Bit4, Bit8, CoreD4, CoreD8, DomainIdentity, ErrorKind, Exactness, Session, Value,
};
use std::rc::Rc;

const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");
const MECHANISM_NAME: &str = "__compiler_domain_shape_mechanism";

fn span() -> Span {
    Span::default()
}

fn list(items: Vec<Expr>) -> Expr {
    Expr {
        kind: ExprKind::List(Rc::from(items.into_boxed_slice())),
        span: span(),
    }
}

fn symbol(name: &str) -> Expr {
    Expr {
        kind: ExprKind::Symbol(name.into()),
        span: span(),
    }
}

fn domain(identity: DomainIdentity) -> Expr {
    Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: span(),
    }
}

fn call_shape(identity: DomainIdentity) -> Expr {
    list(vec![
        symbol("compiler-domain-shape"),
        symbol(MECHANISM_NAME),
        domain(identity),
    ])
}

fn list_values(value: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    let mut cursor = value;
    loop {
        match cursor {
            Value::Nil => return out,
            Value::Pair(head, tail) => {
                out.push(head.as_ref());
                cursor = tail.as_ref();
            }
            other => panic!("expected proper list, got {other}"),
        }
    }
}

fn assert_shape(value: &Value, width: i64, bits: &[bool]) {
    let outer = list_values(value);
    assert_eq!(outer.len(), 2);
    assert!(
        matches!(outer[0], Value::Number(number, Exactness::Exact) if *number == width as f64),
        "shape width must be exact host representation data"
    );

    let observed_bits = list_values(outer[1])
        .into_iter()
        .map(|value| value.as_predicate_bit().expect("shape bits are exact D1"))
        .collect::<Vec<_>>();
    assert_eq!(observed_bits, bits);
}

fn session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("active core loads");
    session
        .environment
        .define(MECHANISM_NAME, domain_identity_shape_mechanism());
    let expressions = parse_mixed_exact_domain(NUCLEUS).expect("compiler nucleus parses");
    eval_parsed_expressions(&expressions, &mut session).expect("compiler nucleus loads");
    session
}

#[test]
fn same_packed_payload_in_different_domains_keeps_exact_width() {
    let mut session = session();

    let d3 = DomainIdentity::D3(Bija3::from_word(Bit3::new(0b100).unwrap()));
    let d4 = DomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0100).unwrap()));

    let d3_shape = eval_parsed_expressions(&[call_shape(d3)], &mut session)
        .expect("D3 shape")
        .value;
    let d4_shape = eval_parsed_expressions(&[call_shape(d4)], &mut session)
        .expect("D4 shape")
        .value;

    assert_shape(&d3_shape, 3, &[true, false, false]);
    assert_shape(&d4_shape, 4, &[false, true, false, false]);
    assert_ne!(d3_shape, d4_shape);
}

#[test]
fn d8_can_be_decomposed_mechanically_without_becoming_semantically_admitted() {
    let mut session = session();
    let d8 = DomainIdentity::D8(CoreD8::from_word(Bit8::new(0b0000_0100).unwrap()));

    let shape = eval_parsed_expressions(&[call_shape(d8)], &mut session)
        .expect("mechanical D8 carrier shape")
        .value;

    assert_shape(
        &shape,
        8,
        &[false, false, false, false, false, true, false, false],
    );
    assert!(
        d8.core_operation().is_none(),
        "mechanical decomposition must not admit D8 as a callable semantic operation"
    );
}

#[test]
fn non_domain_value_fails_named_type_error() {
    let mut session = session();
    let bad = list(vec![
        symbol("compiler-domain-shape"),
        symbol(MECHANISM_NAME),
        Expr {
            kind: ExprKind::Number(4.0, Exactness::Exact),
            span: span(),
        },
    ]);

    let error = eval_parsed_expressions(&[bad], &mut session)
        .expect_err("Number must not masquerade as DomainIdentity");
    assert_eq!(error.kind, ErrorKind::Type);
}
