//! Exact-width Text7 and D2 reader boundaries only.
//! Contextual DEFINE/LAMBDA binding behavior is proved by SENS/Lisp witnesses.

use sens::{parse_canonical_binary, DomainIdentity, syntax::ExprKind};

fn text7_frame(cells: &[u8]) -> String {
    let body = cells
        .iter()
        .map(|cell| format!("{cell:07b}"))
        .collect::<Vec<_>>()
        .join(" 00 ");
    format!("10 {body} 01")
}

#[test]
fn ordinary_w7_words_remain_exact_domain_data_in_d2_lists() {
    let source = "10 1000001 00 1000010 01";
    let expressions =
        parse_canonical_binary(source).expect("ordinary W7 list remains valid D2 structure");
    assert_eq!(expressions.len(), 1);
    let ExprKind::List(items) = &expressions[0].kind else {
        panic!("ordinary W7 D2 list changed AST shape");
    };
    assert_eq!(items.len(), 2);
    for (item, expected) in items.iter().zip([0b1000001, 0b1000010]) {
        let ExprKind::DomainIdentity(identity) = &item.kind else {
            panic!("W7 payload must remain an exact domain identity");
        };
        assert_eq!(identity.width(), 7);
        assert_eq!(identity.packed_bits(), expected);
        assert!(matches!(identity, DomainIdentity::D7(_)));
    }
}

#[test]
fn same_numeric_payload_at_different_widths_keeps_different_identity() {
    let d1 = parse_canonical_binary("1").expect("exact D1 source");
    let d7 = parse_canonical_binary("0000001").expect("exact D7 source");

    let ExprKind::DomainIdentity(d1_identity) = &d1[0].kind else {
        panic!("expected D1 identity");
    };
    let ExprKind::DomainIdentity(d7_identity) = &d7[0].kind else {
        panic!("expected D7 identity");
    };

    assert_eq!((d1_identity.width(), d1_identity.packed_bits()), (1, 1));
    assert_eq!((d7_identity.width(), d7_identity.packed_bits()), (7, 1));
    assert_ne!(d1_identity, d7_identity);
}

#[test]
fn packed_text7_candidate_retains_cell_widths_without_retyping_reader_ast() {
    let source = text7_frame(&[0x41, 0x42]);
    let expressions = parse_canonical_binary(&source).expect("width-qualified sequence parses");
    assert_eq!(expressions.len(), 1);
    let ExprKind::List(items) = &expressions[0].kind else {
        panic!("D2 framing must retain list structure");
    };
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| matches!(
        &item.kind,
        ExprKind::DomainIdentity(DomainIdentity::D7(_))
    )));
}
