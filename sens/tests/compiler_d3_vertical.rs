//! Retired host-authored compiler result oracle. Rust retains only D3 width mechanics.
use sens::{parse_canonical_binary, syntax::ExprKind};

#[test]
fn exact_d3_words_preserve_width_and_payload() {
    for payload in 0usize..8 {
        let source = format!("{payload:03b}");
        let forms = parse_canonical_binary(&source).expect("exact D3 word parses");
        assert_eq!(forms.len(), 1);
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("D3 word must remain a domain identity");
        };
        assert_eq!(identity.width(), 3);
        assert_eq!(identity.packed_bits() as usize, payload);
    }
}

#[test]
fn identical_d3_payload_is_not_reinterpreted_at_other_widths() {
    let d1 = parse_canonical_binary("1").expect("D1 word parses");
    let d3 = parse_canonical_binary("001").expect("D3 word parses");
    let d8 = parse_canonical_binary("00000001").expect("D8 word parses");
    let identity = |forms: &[sens::syntax::Expr]| -> (usize, usize) {
        match &forms[0].kind {
            ExprKind::DomainIdentity(value) => (value.width(), value.packed_bits() as usize),
            _ => panic!("expected domain identity"),
        }
    };
    assert_eq!(identity(&d1), (1, 1));
    assert_eq!(identity(&d3), (3, 1));
    assert_eq!(identity(&d8), (8, 1));
    assert_ne!(identity(&d1), identity(&d3));
    assert_ne!(identity(&d3), identity(&d8));
}