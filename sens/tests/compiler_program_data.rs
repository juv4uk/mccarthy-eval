//! sens#3838 — ratchet the existing SW\x01 wire as C1 program-data transport.

use sens::{
    lower_program, parse_mixed_exact_domain, wire_decode_program, wire_encode_program, Bija3, Bit3, Bit4, Bit8,
    CoreD4, CoreD8, CoreDomainIdentity, DomainIdentity, Expr, ExprKind, Span,
};
use std::rc::Rc;

const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");
const CONTRACT: &str = include_str!("../../../contracts/compiler-program-data-v1.lisp");

fn identity_expr(identity: DomainIdentity) -> Expr {
    Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: Span::default(),
    }
}

#[test]
fn contract_reuses_existing_wire_and_forbids_semantic_host_work() {
    for required in [
        "compiler-program-data/1",
        "sens-syntax-wire/1",
        "domain-width-preserved . required",
        "same-payload-different-domain . distinct",
        "sid8-substitution . forbidden",
        "host-role-selection . forbidden",
        "compiler-admission . fail-closed",
    ] {
        assert!(CONTRACT.contains(required), "missing program-data law: {required}");
    }
}

#[test]
fn same_payload_wrong_domain_remains_distinct_on_program_wire() {
    let d3: DomainIdentity = Bija3::from_word(Bit3::new(0b010).unwrap()).into();
    let d4: DomainIdentity = CoreD4::from_word(Bit4::new(0b0010).unwrap()).into();
    let d8: DomainIdentity = CoreD8::from_word(Bit8::new(0b0000_0010).unwrap()).into();

    let program = vec![
        identity_expr(d3),
        identity_expr(d4),
        identity_expr(d8),
    ];
    let wire = wire_encode_program(&program);
    let decoded = wire_decode_program(&wire).expect("canonical program-data wire");
    assert_eq!(decoded, program);
    assert_eq!(wire_encode_program(&decoded), wire);
    assert_ne!(decoded[0].kind, decoded[1].kind);
    assert_ne!(decoded[1].kind, decoded[2].kind);
}

#[test]
fn domain_call_transport_is_exact_domain_head_plus_ordered_children() {
    let identity = CoreDomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0010).unwrap()));
    let call = Expr {
        kind: ExprKind::DomainCall(
            identity,
            Rc::from([
                identity_expr(Bija3::from_word(Bit3::new(0b100).unwrap()).into()),
                identity_expr(Bija3::from_word(Bit3::new(0b011).unwrap()).into()),
            ]),
        ),
        span: Span::default(),
    };

    let decoded = wire_decode_program(&wire_encode_program(&[call])).expect("domain call wire");
    let ExprKind::List(items) = &decoded[0].kind else {
        panic!("wire must expose domain call as source-shaped program data");
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(
        items[0].kind,
        ExprKind::DomainIdentity(found) if found == DomainIdentity::from(identity)
    ));
    assert!(matches!(items[1].kind, ExprKind::DomainIdentity(_)));
    assert!(matches!(items[2].kind, ExprKind::DomainIdentity(_)));
}

#[test]
fn current_nucleus_lowered_program_wire_is_deterministic() {
    let parsed = parse_mixed_exact_domain(NUCLEUS).expect("current compiler nucleus parses");
    let lowered = lower_program(&parsed);
    let first = wire_encode_program(&lowered);
    let decoded = wire_decode_program(&first).expect("current nucleus wire decodes");
    let second = wire_encode_program(&decoded);
    assert_eq!(second, first, "C1 program-data transport must be byte deterministic");

    fn reject_legacy(expr: &Expr) {
        match &expr.kind {
            ExprKind::Sid(_) | ExprKind::Call(_, _) => {
                panic!("current lowered compiler program-data contains historical Sid/Call")
            }
            ExprKind::List(items) => items.iter().for_each(reject_legacy),
            ExprKind::Pair(head, tail) => {
                reject_legacy(head);
                reject_legacy(tail);
            }
            ExprKind::DomainCall(_, args) => args.iter().for_each(reject_legacy),
            _ => {}
        }
    }
    for expr in &lowered {
        reject_legacy(expr);
    }
}
