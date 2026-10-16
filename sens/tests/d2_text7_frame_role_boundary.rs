//! #3910/#3135: an untagged D2 list of W7 cells is NOT automatically
//! a Text7 binder. The same bytes also represent ordinary D7 data.
//! This is a semantic non-collapse test pending explicit owner ratification.
use sens::{parse_canonical_binary, Expr, ExprKind};

fn only(source: &str) -> Expr {
    let mut exprs = parse_canonical_binary(source)
        .unwrap_or_else(|e| panic!("invalid canonical program {source}: {e:?}"));
    assert_eq!(exprs.len(), 1);
    exprs.remove(0)
}

fn d7_identity(expr: &Expr, bits: u16) {
    match &expr.kind {
        ExprKind::DomainIdentity(identity) => {
            assert_eq!(identity.width(), 7);
            assert_eq!(identity.packed_bits(), bits);
        }
        other => panic!("W7 data changed into a named local: {other:?}"),
    }
}

#[test]
fn d2_frame_with_two_w7_leaves_is_list_not_untyped_symbol() {
    // Same bytes a global speculative Text7 parser could retag to #t7:4142.
    let expression = only("10 1000001 00 1000010 01");
    let ExprKind::List(items) = &expression.kind else {
        panic!("untyped W7 list must remain data without a ratified role");
    };
    assert_eq!(items.len(), 2);
    d7_identity(&items[0], 65);
    d7_identity(&items[1], 66);
}

#[test]
fn quote_keeps_d7_structural_data_not_a_binder() {
    // QUOTE D3:001 and its literal body are both explicit D2 forms.
    let expression = only("10 001 00 10 1000001 00 1000010 01 01");
    let ExprKind::List(top) = &expression.kind else {
        panic!("QUOTE expression needs structure");
    };
    assert_eq!(top.len(), 2);
    let ExprKind::DomainIdentity(head) = &top[0].kind else {
        panic!("QUOTE head must stay exact D3");
    };
    assert_eq!((head.width(), head.packed_bits()), (3, 1));
    let ExprKind::List(data) = &top[1].kind else {
        panic!("quoted D7 data cannot become implicit local Symbol");
    };
    assert_eq!(data.len(), 2);
    d7_identity(&data[0], 65);
    d7_identity(&data[1], 66);
}

#[test]
fn one_cell_d2_list_and_bare_w7_are_not_same_value() {
    let list = only("10 1101010 01");
    let ExprKind::List(data) = &list.kind else {
        panic!("one-cell D2 list remains ordinary list");
    };
    assert_eq!(data.len(), 1);
    d7_identity(&data[0], 106);
    d7_identity(&only("1101010"), 106);
}

#[test]
fn mixed_w7_and_d3_preserves_exact_domain_width() {
    let expression = only("10 1000001 00 001 01");
    let ExprKind::List(items) = &expression.kind else {
        panic!("mixed payloads require structural list");
    };
    assert_eq!(items.len(), 2);
    d7_identity(&items[0], 65);
    let ExprKind::DomainIdentity(second) = &items[1].kind else {
        panic!("D3 payload must survive parse");
    };
    assert_eq!((second.width(), second.packed_bits()), (3, 1));
}

#[test]
fn d2_dot_retains_pair_not_text7_identifier() {
    let expression = only("10 1000001 11 1000010 01");
    let ExprKind::Pair(first, tail) = &expression.kind else {
        panic!("dotted D2 remains Pair");
    };
    d7_identity(first, 65);
    d7_identity(tail, 66);
}

#[test]
fn malformed_structures_do_not_implicitly_frame_text7() {
    for source in ["10 1000001", "10 1000001 11 01",
                   "10 11 1000001 01", "01"] {
        assert!(parse_canonical_binary(source).is_err(), "{source}");
    }
}
