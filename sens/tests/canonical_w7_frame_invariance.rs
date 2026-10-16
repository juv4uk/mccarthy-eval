//! P0 #3910: D2/W7 Text7-name collision, an ORIGINAL migration blocker.
//!
//! This is a crate-external *negative* admission gate, not an implementation
//! of binder framing. D2 already gives the byte/word stream below a list
//! meaning. A parser must not retag it as a global/lexical Symbol only because
//! every item happens to have width seven. An identifier role still requires
//! the distinct, owner-ratified positional/framing law from #3910.
//!
//! No historical SID8 inference, new domain coordinate, Text7 decoder, or
//! original-program migration is admitted by these checks.

use sens::{
    parse_canonical_binary,
    syntax::{Expr, ExprKind},
    DomainIdentity,
};

const FRAME: &str = "10 1000001 00 1000010 01";
const DOT: &str = "10 1000001 11 1000010 01";

fn one(source: &str) -> Expr {
    let mut program = parse_canonical_binary(source).expect("canonical D2/W7 source parses");
    assert_eq!(program.len(), 1, "single canonical expression");
    program.remove(0)
}

fn d7(expr: &Expr, bits: u16) {
    let ExprKind::DomainIdentity(identity) = &expr.kind else {
        panic!("W7 leaf must stay D7 identity, not a Symbol or callable: {expr:?}");
    };
    assert_eq!(identity.width(), 7, "W7 width is an exact source identity");
    assert_eq!(identity.packed_bits(), bits, "W7 bits may not change");
    assert!(
        identity.core_operation().is_none(),
        "Text7/Sound7 leaf cannot acquire builtin callability"
    );
}

fn two_w7_list(expr: &Expr) {
    let ExprKind::List(items) = &expr.kind else {
        panic!("existing D2 two-W7 list was silently reclassified as an identifier: {expr:?}");
    };
    assert_eq!(items.len(), 2);
    d7(&items[0], 65);
    d7(&items[1], 66);
}

#[test]
fn same_d2_w7_words_are_an_ordinary_two_element_list() {
    two_w7_list(&one(FRAME));
}

#[test]
fn w7_sequence_without_d2_frame_is_two_independent_values() {
    let roots = parse_canonical_binary("1000001 00 1000010")
        .expect("D2 separators preserve top-level W7 leaf identities");
    assert_eq!(roots.len(), 2);
    d7(&roots[0], 65);
    d7(&roots[1], 66);
}

#[test]
fn quoted_w7_list_does_not_turn_into_text7_identifier() {
    // D3:001 QUOTE is a code identity; the nested value is still D2 data.
    let expr = one("10 001 00 10 1000001 00 1000010 01 01");
    let ExprKind::List(items) = expr.kind else {
        panic!("QUOTE expression must remain a D2 list");
    };
    assert_eq!(items.len(), 2);
    let ExprKind::DomainIdentity(quote) = &items[0].kind else {
        panic!("head must remain exact D3 QUOTE");
    };
    assert_eq!((quote.width(), quote.packed_bits()), (3, 1));
    two_w7_list(&items[1]);
}

#[test]
fn nested_w7_list_never_silently_becomes_symbol() {
    let outer = one("10 10 1000001 00 1000010 01 01");
    let ExprKind::List(items) = outer.kind else {
        panic!("outer D2 structure must remain a list");
    };
    assert_eq!(items.len(), 1);
    two_w7_list(&items[0]);
}

#[test]
fn d2_dot_with_same_w7_atoms_is_a_pair_not_text7() {
    let expr = one(DOT);
    let ExprKind::Pair(head, tail) = expr.kind else {
        panic!("D2:11 DOT must preserve pair semantics");
    };
    d7(&head, 65);
    d7(&tail, 66);
}

#[test]
fn mixed_w7_d3_list_stays_structurally_distinct() {
    let expr = one("10 1000001 00 100 01");
    let ExprKind::List(items) = expr.kind else {
        panic!("mixed-width data needs D2 list, not Text7/name dispatch");
    };
    assert_eq!(items.len(), 2);
    d7(&items[0], 65);
    let ExprKind::DomainIdentity(other) = &items[1].kind else {
        panic!("D3 atom must retain exact domain identity");
    };
    assert_eq!((other.width(), other.packed_bits()), (3, 4));
}

#[test]
fn unclosed_w7_structure_still_fails_closed() {
    let error = parse_canonical_binary("10 1000001 00 1000010")
        .expect_err("missing D2:01 must not be rescued by heuristic Text7 framing");
    assert_eq!(error.kind, sens::ErrorKind::Parse);
}

#[test]
fn explicit_width_distinction_is_not_lost_to_spelling() {
    let roots = parse_canonical_binary("0000001 00 00000001")
        .expect("D7 W7=1 and D8 W8=1 are two distinct identities");
    assert_eq!(roots.len(), 2);
    d7(&roots[0], 1);
    let ExprKind::DomainIdentity(second) = &roots[1].kind else {
        panic!("D8 must remain an exact-domain identity, not historical SID8");
    };
    assert_eq!((second.width(), second.packed_bits()), (8, 1));
    assert_ne!(
        DomainIdentity::from_source_word(
            sens::parse_binary_source_words("0000001").unwrap()[0].word
        ),
        *second
    );
}
